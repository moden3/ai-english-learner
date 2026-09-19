use aws_sdk_ssm::Client as SsmClient;
use lambda_http::{Body, Error, Request, RequestPayloadExt, Response, run, service_fn};
use reqwest::Client as HttpClient;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

#[derive(Deserialize, Debug)]
struct GenerateRequest {
    topic_name: Option<String>,
    use_web_search: Option<bool>,
    action: Option<String>,
    text: Option<String>,
}

// --- Gemini API からのレスポンスをパースするための構造体群 ---
#[derive(Deserialize, Debug)]
struct GeminiResponse {
    candidates: Option<Vec<Candidate>>,
}

#[derive(Deserialize, Debug)]
struct Candidate {
    content: Content,
}

#[derive(Deserialize, Debug)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Deserialize, Debug)]
struct Part {
    text: String,
}

// --- Tavily AI Search API のレスポンス構造体 ---
#[derive(Deserialize, Debug)]
struct TavilyResponse {
    results: Vec<TavilyResult>,
}

#[derive(Deserialize, Debug)]
struct TavilyResult {
    title: String,
    content: String,
    url: String,
}

/// Tavily AI Search API を呼び出し、最新ニュースを取得する
/// 成功時は Ok(Vec<TavilyResult>)、失敗時は Err(String) を返す
async fn call_tavily_search(
    http_client: &HttpClient,
    tavily_api_key: &str,
    query: &str,
) -> Result<Vec<TavilyResult>, String> {
    if tavily_api_key.is_empty() || tavily_api_key == "CHANGE_ME_TAVILY_KEY" {
        return Err("Tavily API key is not configured.".to_string());
    }

    let request_body = json!({
        "query": format!("{} latest news", query),
        "topic": "news",
        "days": 3,
        "max_results": 3,
        "include_answer": false
    });

    let res = http_client
        .post("https://api.tavily.com/search")
        .header("Authorization", format!("Bearer {}", tavily_api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await;

    match res {
        Ok(resp) if resp.status().is_success() => match resp.json::<TavilyResponse>().await {
            Ok(tavily_resp) => {
                println!(
                    "Tavily search returned {} results.",
                    tavily_resp.results.len()
                );
                if tavily_resp.results.is_empty() {
                    Err("Tavily returned no results.".to_string())
                } else {
                    Ok(tavily_resp.results)
                }
            }
            Err(e) => {
                let msg = format!("Tavily response parse error: {:?}", e);
                println!("{}", msg);
                Err(msg)
            }
        },
        Ok(resp) => {
            let status = resp.status();
            let err_text = resp.text().await.unwrap_or_default();
            let msg = format!("Tavily API error ({}): {}", status, err_text);
            println!("{}", msg);
            Err(msg)
        }
        Err(e) => {
            let msg = format!("Tavily HTTP request failed: {:?}", e);
            println!("{}", msg);
            Err(msg)
        }
    }
}

/// GenerateText API のメインハンドラ
async fn function_handler(
    event: Request,
    http_client: Arc<HttpClient>,
    app_api_key: &str,
    gemini_api_key: Arc<String>,
    tavily_api_key: Arc<String>,
) -> Result<Response<Body>, Error> {
    if !eng_app_backend::validate_api_key(&event, app_api_key) {
        return Ok(Response::builder()
            .status(401)
            .body(Body::Text("Unauthorized".into()))
            .expect("failed to render response"));
    }
    if *event.method() != lambda_http::http::Method::POST {
        return Ok(Response::builder()
            .status(405)
            .body(Body::Text("Method Not Allowed".into()))
            .expect("failed to render response"));
    }

    let req_body = match event.payload::<GenerateRequest>() {
        Ok(Some(req)) => req,
        _ => {
            return Ok(Response::builder()
                .status(400)
                .body(Body::Text("Invalid Request Body".into()))
                .expect("failed to render response"));
        }
    };

    // ダミーモードの判定
    // 1. 環境変数 USE_MOCK_AI が設定されている
    // 2. トピック名が "test" または "dummy" で始まる
    // 3. Gemini APIキーが未設定（空文字 or プレースホルダー）
    let topic_name = req_body.topic_name.clone().unwrap_or_default();
    let is_dummy_mode = std::env::var("USE_MOCK_AI").is_ok()
        || topic_name.to_lowercase().starts_with("test")
        || topic_name.to_lowercase().starts_with("dummy")
        || gemini_api_key.is_empty()
        || gemini_api_key.as_str() == "CHANGE_ME_GEMINI_KEY";

    let action = req_body.action.unwrap_or_else(|| "generate".to_string());

    if is_dummy_mode {
        let dummy_res = if action == "analyze" {
            json!({
                "segments": [
                    { "id": 1, "text": "This is a dummy", "translation": "これはダミーです", "grammar_note": "主語(S)と動詞(V)" },
                    { "id": 2, "text": "analysis result", "translation": "解析結果です", "grammar_note": "名詞句" }
                ],
                "keywords": [
                    { "word": "dummy", "meaning": "ダミーの", "part_of_speech": "noun", "example": "This is a dummy text." }
                ]
            })
        } else {
            json!({
                "text": format!("(Dummy Mode) This is a generated test article about '{}'. It does not consume any API tokens.", topic_name),
                "source_url": "https://example.com/dummy-news-source"
            })
        };

        return Ok(Response::builder()
            .status(200)
            .header("content-type", "application/json")
            .body(Body::Text(serde_json::to_string(&dummy_res).unwrap()))
            .expect("failed to render response"));
    }

    // モデルは常に gemini-3.5-flash-lite を使用
    let model_name = "gemini-3.5-flash-lite";

    let gemini_url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model_name, gemini_api_key
    );

    // アクションに応じたプロンプトの作成
    let prompt = if action == "analyze" {
        let text_to_analyze = req_body.text.unwrap_or_default();
        format!(
            "You are an English teacher. Break down the following english text into segments (chunks of meaning), translate each segment into Japanese, and provide a short grammar note for each. Also, extract highly advanced business keywords or idioms (CEFR C1 level) from the text. Skip basic and intermediate words (A1-B2) as the user already knows them (TOEIC 800+). Extract a maximum of 10 words. Do NOT include literal slash characters ('/') in the text.
Text to analyze: \"{}\"
You MUST output strictly in valid JSON format matching this schema exactly:
{{
  \"segments\": [
    {{ \"id\": 1, \"text\": \"The quick brown fox\", \"translation\": \"素早い茶色のキツネが\", \"grammar_note\": \"主語(S)\" }},
    {{ \"id\": 2, \"text\": \"jumps over\", \"translation\": \"〜を飛び越える\", \"grammar_note\": \"動詞(V) + 前置詞(prep)\" }},
    {{ \"id\": 3, \"text\": \"the lazy dog.\", \"translation\": \"怠け者の犬を。\", \"grammar_note\": \"目的語(O)\" }}
  ],
  \"keywords\": [
    {{ \"word\": \"lazy\", \"meaning\": \"怠惰な\", \"part_of_speech\": \"adjective\", \"example\": \"He is a lazy dog.\" }}
  ]
}}",
            text_to_analyze
        )
    } else {
        let use_web_search = req_body.use_web_search.unwrap_or(false);

        if use_web_search {
            // Tavily APIを呼び出す。失敗した場合は503を返して生成失敗とする
            let results =
                match call_tavily_search(&http_client, &tavily_api_key, &topic_name).await {
                    Ok(r) => r,
                    Err(e) => {
                        println!("Tavily search failed: {}", e);
                        return Ok(Response::builder()
                            .status(503)
                            .body(Body::Text(
                                "Web search failed. Please try again or disable web search."
                                    .into(),
                            ))
                            .expect("failed to render response"));
                    }
                };

            // 最初の記事のURLをsource_urlとして使用
            let source_url = &results[0].url;
            // ニュース記事を番号付きで整形
            let news_context = results
                .iter()
                .enumerate()
                .map(|(i, r)| {
                    format!(
                        "[{}] Title: {}\n    URL: {}\n    Content: {}",
                        i + 1,
                        r.title,
                        r.url,
                        // コンテキストが長すぎる場合は400文字で切る
                        if r.content.len() > 400 {
                            &r.content[..400]
                        } else {
                            &r.content
                        }
                    )
                })
                .collect::<Vec<_>>()
                .join("\n\n");

            // 最新ニュースコンテキストあり（Tavily検索成功時）
            format!(
                "You are an English teacher. Based on the following latest news articles retrieved from the web, write a short, highly professional business English article (between 150 to 250 words) about '{}' suitable for upper-intermediate to advanced business English learners (TOEIC 800+, CEFR B2-C1). Incorporate practical and advanced business vocabulary from the news context. Do not fabricate facts; base the article on the provided news content.

--- Latest News Context ---
{}
---

You MUST output strictly in valid JSON format matching this schema exactly:
{{
  \"text\": \"The generated english article based on the news...\",
  \"source_url\": \"{}\"
}}",
                topic_name, news_context, source_url
            )
        } else {
            // Webサーチなし
            format!(
                "You are an English teacher. Write a short, highly professional business English article (between 150 to 250 words) about '{}' suitable for upper-intermediate to advanced business English learners (TOEIC 800+, CEFR B2-C1). Incorporate practical and advanced business vocabulary.
Focus on real-world business contexts, modern industry trends, and practical vocabulary.
You MUST output strictly in valid JSON format matching this schema exactly:
{{
  \"text\": \"The generated english article...\",
  \"source_url\": null
}}",
                topic_name
            )
        }
    };

    // Gemini APIリクエストの生成
    let request_body = json!({
        "contents": [{
            "parts": [{"text": prompt}]
        }],
        "generationConfig": {
            "temperature": 0.9,
            "responseMimeType": "application/json"
        }
    });

    let res = http_client
        .post(&gemini_url)
        .json(&request_body)
        .send()
        .await;

    match res {
        Ok(resp) => {
            if resp.status().is_success() {
                let gemini_resp: GeminiResponse = resp.json().await?;
                // AIが返してきた文字列（JSONとして指示したので中身はJSON文字列のはず）
                let mut generated_json_text = gemini_resp
                    .candidates
                    .and_then(|c| c.into_iter().next())
                    .and_then(|c| c.content.parts.into_iter().next())
                    .map(|p| p.text)
                    .unwrap_or_else(|| "{}".to_string());

                // LLMがマークダウンブロック(```json)や余計な挨拶を含めてしまった場合、JSONの波括弧部分だけを抽出する
                if let Some(start) = generated_json_text.find('{') {
                    if let Some(end) = generated_json_text.rfind('}') {
                        if start <= end {
                            generated_json_text = generated_json_text[start..=end].to_string();
                        }
                    }
                }

                // AIの返答（JSON文字列）を任意のJSON Valueにパースする
                let out: serde_json::Value = serde_json::from_str(&generated_json_text)
                    .unwrap_or_else(|_| json!({ "error": "Failed to parse AI response" }));

                Ok(Response::builder()
                    .status(200)
                    .header("content-type", "application/json")
                    .body(Body::Text(serde_json::to_string(&out).unwrap()))
                    .expect("failed to render response"))
            } else {
                let err_text = resp.text().await?;
                println!("Gemini API Error Response: {}", err_text);
                Ok(Response::builder()
                    .status(500)
                    .body(Body::Text("Gemini API returned an error".into()))
                    .expect("failed to render response"))
            }
        }
        Err(e) => {
            println!("HTTP Request Failed: {:?}", e);
            Ok(Response::builder()
                .status(500)
                .body(Body::Text("Internal Server Error".into()))
                .expect("failed to render response"))
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let ssm_client = SsmClient::new(&config);
    let http_client = Arc::new(HttpClient::new());

    // 起動時（コールドスタート時）に全APIキーを一度だけSSMから取得する
    let app_api_key = Arc::new(eng_app_backend::get_api_key(&ssm_client).await);
    let gemini_api_key = Arc::new(eng_app_backend::get_gemini_api_key(&ssm_client).await);
    let tavily_api_key = Arc::new(eng_app_backend::get_tavily_api_key(&ssm_client).await);

    run(service_fn(move |event| {
        let http_client = http_client.clone();
        let app_api_key = app_api_key.clone();
        let gemini_api_key = gemini_api_key.clone();
        let tavily_api_key = tavily_api_key.clone();
        async move {
            function_handler(
                event,
                http_client,
                &app_api_key,
                gemini_api_key,
                tavily_api_key,
            )
            .await
        }
    }))
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use lambda_http::http::{Method, Request as HttpRequest};

    #[tokio::test]
    async fn test_post_generate_text_fails_with_dummy_key() {
        let payload = r#"{
            "topic_name": "business"
        }"#;

        let request = HttpRequest::builder()
            .method(Method::POST)
            .uri("/generate_text")
            .header("content-type", "application/json")
            .body(Body::Text(payload.to_string()))
            .expect("failed to build request");

        let http_client = Arc::new(HttpClient::new());
        // gemini_api_keyが空 → ダミーモードになるため500ではなく200(ダミーレスポンス)になる
        let gemini_api_key = Arc::new(String::new());
        let tavily_api_key = Arc::new(String::new());

        let response = function_handler(
            request,
            http_client,
            "",
            gemini_api_key,
            tavily_api_key,
        )
        .await
        .expect("handler failed");

        // api_keyが空文字 → Unauthorized (401) になるはず
        assert_eq!(
            response.status(),
            401,
            "app_api_keyが空のため401 Unauthorizedになるはずです"
        );
    }
}
