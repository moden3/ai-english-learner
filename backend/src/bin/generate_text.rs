use aws_sdk_ssm::Client as SsmClient;
use lambda_http::{Body, Error, Request, RequestPayloadExt, Response, run, service_fn};
use reqwest::Client as HttpClient;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

#[derive(Deserialize, Debug)]
struct GenerateRequest {
    topic_name: Option<String>,
    use_lite_model: Option<bool>,
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

/// GenerateText API のメインハンドラ
async fn function_handler(
    event: Request,
    ssm_client: Arc<SsmClient>,
    http_client: Arc<HttpClient>,
    api_key: &str,
) -> Result<Response<Body>, Error> {
    if !eng_app_backend::validate_api_key(&event, api_key) {
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

    // SSMからGemini API Keyを取得
    let ssm_res = ssm_client
        .get_parameter()
        .name("/eng-app/gemini-api-key")
        .with_decryption(true)
        .send()
        .await;

    let api_key = match ssm_res {
        Ok(out) => out.parameter.unwrap().value.unwrap(),
        Err(e) => {
            println!("SSM error (could not read API Key): {:?}", e);
            "DUMMY_KEY_FOR_TESTING".to_string()
        }
    };

    // ダミーモードの判定
    // 1. 環境変数 USE_MOCK_AI が設定されている
    // 2. トピック名が "test" または "dummy" で始まる
    // 3. SSMからAPIキーが取得できず DUMMY_KEY_FOR_TESTING となっている
    let topic_name = req_body.topic_name.clone().unwrap_or_default();
    let is_dummy_mode = std::env::var("USE_MOCK_AI").is_ok()
        || topic_name.to_lowercase().starts_with("test")
        || topic_name.to_lowercase().starts_with("dummy")
        || api_key == "DUMMY_KEY_FOR_TESTING";

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
        format!(
            "You are an English teacher. Write a short, highly professional business English article (between 150 to 250 words) about '{}' suitable for upper-intermediate to advanced business English learners (TOEIC 800+, CEFR B2-C1). Incorporate practical and advanced business vocabulary. 
Use the latest news via your Google Search tool if possible. 
You MUST output strictly in valid JSON format matching this schema exactly:
{{
  \"text\": \"The generated english article...\",
  \"source_url\": \"The URL of the news source you referenced, or null if not applicable\"
}}",
            topic_name
        )
    };

    let mut is_lite = req_body.use_lite_model.unwrap_or(true);

    // analyzeの場合は強制的にLiteモデルを使う（検索不要・コスト削減のため）
    if action == "analyze" {
        is_lite = true;
    }

    let model_name = if is_lite {
        "gemini-3.5-flash-lite"
    } else {
        "gemini-3.6-flash"
    };

    let gemini_url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model_name, api_key
    );

    // Gemini APIリクエストの生成
    // 注: Google Search ツールと responseMimeType: "application/json" はGemini APIの仕様上併用不可（400エラーとなる）のため、
    // !is_lite（検索ツール利用時）は responseMimeType を含めない
    let generation_config = if is_lite {
        json!({
            "temperature": 0.9,
            "responseMimeType": "application/json"
        })
    } else {
        json!({
            "temperature": 0.9
        })
    };

    let mut request_body = json!({
        "contents": [{
            "parts": [{"text": prompt}]
        }],
        "generationConfig": generation_config
    });

    // Liteモデルではない（標準モデルの）場合のみ、Google Searchツールを有効化
    if !is_lite {
        if let Some(obj) = request_body.as_object_mut() {
            obj.insert("tools".to_string(), json!([{ "googleSearch": {} }]));
        }
    }

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
    let ssm_client = Arc::new(SsmClient::new(&config));
    let http_client = Arc::new(HttpClient::new());

    let app_api_key = eng_app_backend::get_api_key(&ssm_client).await;
    let app_api_key = Arc::new(app_api_key);

    run(service_fn(move |event| {
        let ssm_client = ssm_client.clone();
        let http_client = http_client.clone();
        let app_api_key = app_api_key.clone();
        async move { function_handler(event, ssm_client, http_client, &app_api_key).await }
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

        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let ssm_client = Arc::new(SsmClient::new(&config));
        let http_client = Arc::new(HttpClient::new());

        let response = function_handler(request, ssm_client, http_client, "")
            .await
            .expect("handler failed");

        assert_eq!(
            response.status(),
            500,
            "ダミーキーでの通信になるため500エラーになるはずです"
        );
    }
}
