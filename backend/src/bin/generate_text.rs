use aws_sdk_ssm::Client as SsmClient;
use eng_app_backend::{
    TavilyResult, build_analyze_prompt, build_generate_prompt, extract_json_payload,
    format_news_context, validate_api_key,
};
use lambda_http::{Body, Error, Request, RequestPayloadExt, Response, run, service_fn};
use reqwest::Client as HttpClient;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ApiEndpoints {
    pub tavily_url: String,
    pub gemini_base_url: String,
}

impl Default for ApiEndpoints {
    fn default() -> Self {
        Self {
            tavily_url: "https://api.tavily.com/search".to_string(),
            gemini_base_url: "https://generativelanguage.googleapis.com".to_string(),
        }
    }
}

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

/// Tavily AI Search API を呼び出し、最新ニュースを取得する
/// 成功時は Ok(Vec<TavilyResult>)、失敗時は Err(String) を返す
async fn call_tavily_search(
    http_client: &HttpClient,
    tavily_url: &str,
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
        .post(tavily_url)
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

/// Google Gemini API を呼び出し、生成された JSON 文字列を返す。
/// Google 側のサーバー一時過負荷（503 Service Unavailable）やレート制限（429）が発生した場合は、
/// 指数バックオフ（1秒 → 2秒）で最大3回まで自動リトライを行い、一時的なスパイク障害を吸収する。
async fn call_gemini_with_retry(
    http_client: &HttpClient,
    gemini_url: &str,
    request_body: &serde_json::Value,
    max_attempts: u32,
    base_delay_ms: u64,
) -> Result<String, (u16, String)> {
    let mut last_status = 500;
    let mut last_message = "Internal Server Error".to_string();

    for attempt in 1..=max_attempts {
        match http_client.post(gemini_url).json(request_body).send().await {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    match resp.json::<GeminiResponse>().await {
                        Ok(gemini_resp) => {
                            let generated_json_text = gemini_resp
                                .candidates
                                .and_then(|c| c.into_iter().next())
                                .and_then(|c| c.content.parts.into_iter().next())
                                .map(|p| p.text)
                                .unwrap_or_else(|| "{}".to_string());
                            return Ok(generated_json_text);
                        }
                        Err(e) => {
                            println!("Gemini response parse error: {:?}", e);
                            return Err((500, "Failed to parse Gemini API response".to_string()));
                        }
                    }
                }

                let code = status.as_u16();
                let err_text = resp.text().await.unwrap_or_default();
                println!(
                    "Gemini API Error Response (attempt {}/{}, status {}): {}",
                    attempt, max_attempts, code, err_text
                );

                last_status = if code == 503 || code == 429 { 503 } else { 500 };
                last_message = if code == 503 {
                    "Gemini AI is currently experiencing high demand (503). Please try again in a few seconds.".to_string()
                } else if code == 429 {
                    "Gemini AI rate limit reached (429). Please wait a moment and try again."
                        .to_string()
                } else {
                    "Gemini API returned an error".to_string()
                };

                // 503 (UNAVAILABLE) または 429 (RESOURCE_EXHAUSTED) の場合のみリトライ
                if (code == 503 || code == 429) && attempt < max_attempts {
                    let delay = base_delay_ms * (1 << (attempt - 1));
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                    continue;
                }
                break;
            }
            Err(e) => {
                println!(
                    "HTTP Request Failed (attempt {}/{}): {:?}",
                    attempt, max_attempts, e
                );
                last_status = 500;
                last_message = "Internal Server Error".to_string();
                if attempt < max_attempts {
                    let delay = base_delay_ms * (1 << (attempt - 1));
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                    continue;
                }
                break;
            }
        }
    }

    Err((last_status, last_message))
}

/// GenerateText API のメインハンドラ
async fn function_handler(
    event: Request,
    http_client: Arc<HttpClient>,
    app_api_key: &str,
    gemini_api_key: Arc<String>,
    tavily_api_key: Arc<String>,
    endpoints: Arc<ApiEndpoints>,
) -> Result<Response<Body>, Error> {
    if !validate_api_key(&event, app_api_key) {
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
    // 1. トピック名が "test" または "dummy" で始まる
    // 2. 解析対象テキストが "(Dummy Mode)" で始まる (ダミー生成記事の構文解析時)
    // 3. Gemini APIキーが未設定（空文字 or プレースホルダー）
    let topic_name = req_body.topic_name.clone().unwrap_or_default();
    let text_to_analyze = req_body.text.clone().unwrap_or_default();
    let is_dummy_mode = topic_name.to_lowercase().starts_with("test")
        || topic_name.to_lowercase().starts_with("dummy")
        || text_to_analyze.starts_with("(Dummy Mode)")
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
        "{}/v1beta/models/{}:generateContent?key={}",
        endpoints.gemini_base_url.trim_end_matches('/'),
        model_name,
        gemini_api_key
    );

    // アクションに応じたプロンプトの作成
    let prompt = if action == "analyze" {
        build_analyze_prompt(&text_to_analyze)
    } else {
        let use_web_search = req_body.use_web_search.unwrap_or(false);

        if use_web_search {
            // Tavily APIを呼び出す。失敗した場合は503を返して生成失敗とする
            let results = match call_tavily_search(
                &http_client,
                &endpoints.tavily_url,
                &tavily_api_key,
                &topic_name,
            )
            .await
            {
                Ok(r) => r,
                Err(e) => {
                    println!("Tavily search failed: {}", e);
                    return Ok(Response::builder()
                        .status(503)
                        .body(Body::Text(
                            "Web search failed. Please try again or disable web search.".into(),
                        ))
                        .expect("failed to render response"));
                }
            };

            let (news_context, source_url) = format_news_context(&results);
            build_generate_prompt(&topic_name, Some(&news_context), Some(&source_url))
        } else {
            // Web検索なし
            build_generate_prompt(&topic_name, None, None)
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

    match call_gemini_with_retry(&http_client, &gemini_url, &request_body, 3, 1000).await {
        Ok(generated_json_text) => {
            let out: serde_json::Value = extract_json_payload(&generated_json_text)
                .unwrap_or_else(|_| json!({ "error": "Failed to parse AI response" }));

            Ok(Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body(Body::Text(serde_json::to_string(&out).unwrap()))
                .expect("failed to render response"))
        }
        Err((status_code, err_msg)) => Ok(Response::builder()
            .status(status_code)
            .body(Body::Text(err_msg))
            .expect("failed to render response")),
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let ssm_client = SsmClient::new(&config);
    let http_client = Arc::new(HttpClient::new());
    let endpoints = Arc::new(ApiEndpoints::default());

    // 起動時（コールドスタート時）に全APIキーを一度だけSSMから取得する
    let app_api_key = Arc::new(eng_app_backend::get_api_key(&ssm_client).await);
    let gemini_api_key = Arc::new(eng_app_backend::get_gemini_api_key(&ssm_client).await);
    let tavily_api_key = Arc::new(eng_app_backend::get_tavily_api_key(&ssm_client).await);

    run(service_fn(move |event| {
        let http_client = http_client.clone();
        let app_api_key = app_api_key.clone();
        let gemini_api_key = gemini_api_key.clone();
        let tavily_api_key = tavily_api_key.clone();
        let endpoints = endpoints.clone();
        async move {
            function_handler(
                event,
                http_client,
                &app_api_key,
                gemini_api_key,
                tavily_api_key,
                endpoints,
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
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn make_test_endpoints(mock_server: &MockServer) -> Arc<ApiEndpoints> {
        Arc::new(ApiEndpoints {
            tavily_url: format!("{}/search", mock_server.uri()),
            gemini_base_url: mock_server.uri(),
        })
    }

    #[tokio::test]
    async fn test_unauthorized_when_api_key_missing_or_invalid() {
        let payload = r#"{"topic_name": "business"}"#;
        let request = HttpRequest::builder()
            .method(Method::POST)
            .uri("/generate_text")
            .header("content-type", "application/json")
            .body(Body::Text(payload.to_string()))
            .expect("failed to build request");

        let http_client = Arc::new(HttpClient::new());
        let gemini_api_key = Arc::new("dummy_gemini".into());
        let tavily_api_key = Arc::new("dummy_tavily".into());
        let endpoints = Arc::new(ApiEndpoints::default());

        let response = function_handler(
            request,
            http_client,
            "secret_key",
            gemini_api_key,
            tavily_api_key,
            endpoints,
        )
        .await
        .expect("handler failed");

        assert_eq!(response.status(), 401);
    }

    #[tokio::test]
    async fn test_bad_request_on_invalid_json_payload() {
        let request = HttpRequest::builder()
            .method(Method::POST)
            .uri("/generate_text")
            .header("x-api-key", "secret_key")
            .header("content-type", "application/json")
            .body(Body::Text("invalid json {{{".to_string()))
            .expect("failed to build request");

        let http_client = Arc::new(HttpClient::new());
        let gemini_api_key = Arc::new("dummy_gemini".into());
        let tavily_api_key = Arc::new("dummy_tavily".into());
        let endpoints = Arc::new(ApiEndpoints::default());

        let response = function_handler(
            request,
            http_client,
            "secret_key",
            gemini_api_key,
            tavily_api_key,
            endpoints,
        )
        .await
        .expect("handler failed");

        assert_eq!(response.status(), 400);
    }

    #[tokio::test]
    async fn test_dummy_mode_skips_external_api_calls() {
        let payload = r#"{"topic_name": "test-topic", "action": "generate"}"#;
        let request = HttpRequest::builder()
            .method(Method::POST)
            .uri("/generate_text")
            .header("x-api-key", "secret_key")
            .header("content-type", "application/json")
            .body(Body::Text(payload.to_string()))
            .expect("failed to build request");

        let http_client = Arc::new(HttpClient::new());
        let gemini_api_key = Arc::new("dummy_gemini".into());
        let tavily_api_key = Arc::new("dummy_tavily".into());
        // mock_serverは一切mountしない（呼ばれたらエラーになるはず）
        let mock_server = MockServer::start().await;
        let endpoints = make_test_endpoints(&mock_server);

        let response = function_handler(
            request,
            http_client,
            "secret_key",
            gemini_api_key,
            tavily_api_key,
            endpoints,
        )
        .await
        .expect("handler failed");

        assert_eq!(response.status(), 200);
        let body_str = match response.body() {
            Body::Text(s) => s.clone(),
            _ => String::new(),
        };
        assert!(body_str.contains("Dummy Mode"));
    }

    #[tokio::test]
    async fn test_dummy_mode_analyze_skips_external_api_calls() {
        let payload =
            r#"{"action": "analyze", "text": "(Dummy Mode) This is a generated test article."}"#;
        let request = HttpRequest::builder()
            .method(Method::POST)
            .uri("/generate_text")
            .header("x-api-key", "secret_key")
            .header("content-type", "application/json")
            .body(Body::Text(payload.to_string()))
            .expect("failed to build request");

        let http_client = Arc::new(HttpClient::new());
        let gemini_api_key = Arc::new("real_gemini_key".into());
        let tavily_api_key = Arc::new("real_tavily_key".into());
        let mock_server = MockServer::start().await;
        let endpoints = make_test_endpoints(&mock_server);

        let response = function_handler(
            request,
            http_client,
            "secret_key",
            gemini_api_key,
            tavily_api_key,
            endpoints,
        )
        .await
        .expect("handler failed");

        assert_eq!(response.status(), 200);
        let body_str = match response.body() {
            Body::Text(s) => s.clone(),
            _ => String::new(),
        };
        assert!(body_str.contains("This is a dummy"));
    }

    #[tokio::test]
    async fn test_generate_with_web_search_success() {
        let mock_server = MockServer::start().await;

        // 1. Tavily Searchのモック
        let tavily_body = json!({
            "results": [
                {
                    "title": "Quantum Leap in AI",
                    "content": "Researchers have discovered new AI paradigms.",
                    "url": "https://tech.example.com/ai-quantum"
                }
            ]
        });
        Mock::given(method("POST"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(200).set_body_json(tavily_body))
            .expect(1)
            .mount(&mock_server)
            .await;

        // 2. Gemini APIのモック
        let gemini_body = json!({
            "candidates": [{
                "content": {
                    "parts": [{
                        "text": "{\"text\": \"Recent breakthroughs in quantum computing are accelerating AI development.\", \"source_url\": \"https://tech.example.com/ai-quantum\"}"
                    }]
                }
            }]
        });
        Mock::given(method("POST"))
            .and(path("/v1beta/models/gemini-3.5-flash-lite:generateContent"))
            .respond_with(ResponseTemplate::new(200).set_body_json(gemini_body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let payload = r#"{"topic_name": "quantum-ai", "use_web_search": true}"#;
        let request = HttpRequest::builder()
            .method(Method::POST)
            .uri("/generate_text")
            .header("x-api-key", "secret_key")
            .header("content-type", "application/json")
            .body(Body::Text(payload.to_string()))
            .expect("failed to build request");

        let http_client = Arc::new(HttpClient::new());
        let gemini_api_key = Arc::new("real_gemini_key".into());
        let tavily_api_key = Arc::new("real_tavily_key".into());
        let endpoints = make_test_endpoints(&mock_server);

        let response = function_handler(
            request,
            http_client,
            "secret_key",
            gemini_api_key,
            tavily_api_key,
            endpoints,
        )
        .await
        .expect("handler failed");

        assert_eq!(response.status(), 200);
        let body_str = match response.body() {
            Body::Text(s) => s.clone(),
            _ => String::new(),
        };
        assert!(body_str.contains("https://tech.example.com/ai-quantum"));
        assert!(body_str.contains("Recent breakthroughs in quantum computing"));
    }

    #[tokio::test]
    async fn test_generate_with_web_search_tavily_500_returns_503() {
        let mock_server = MockServer::start().await;

        // Tavilyが500エラーを返却
        Mock::given(method("POST"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let payload = r#"{"topic_name": "quantum-ai", "use_web_search": true}"#;
        let request = HttpRequest::builder()
            .method(Method::POST)
            .uri("/generate_text")
            .header("x-api-key", "secret_key")
            .header("content-type", "application/json")
            .body(Body::Text(payload.to_string()))
            .expect("failed to build request");

        let http_client = Arc::new(HttpClient::new());
        let gemini_api_key = Arc::new("real_gemini_key".into());
        let tavily_api_key = Arc::new("real_tavily_key".into());
        let endpoints = make_test_endpoints(&mock_server);

        let response = function_handler(
            request,
            http_client,
            "secret_key",
            gemini_api_key,
            tavily_api_key,
            endpoints,
        )
        .await
        .expect("handler failed");

        assert_eq!(response.status(), 503);
    }

    #[tokio::test]
    async fn test_generate_with_web_search_tavily_empty_results_returns_503() {
        let mock_server = MockServer::start().await;

        // Tavilyが空結果を返却
        let empty_resp = json!({ "results": [] });
        Mock::given(method("POST"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(200).set_body_json(empty_resp))
            .expect(1)
            .mount(&mock_server)
            .await;

        let payload = r#"{"topic_name": "obscure-topic", "use_web_search": true}"#;
        let request = HttpRequest::builder()
            .method(Method::POST)
            .uri("/generate_text")
            .header("x-api-key", "secret_key")
            .header("content-type", "application/json")
            .body(Body::Text(payload.to_string()))
            .expect("failed to build request");

        let http_client = Arc::new(HttpClient::new());
        let gemini_api_key = Arc::new("real_gemini_key".into());
        let tavily_api_key = Arc::new("real_tavily_key".into());
        let endpoints = make_test_endpoints(&mock_server);

        let response = function_handler(
            request,
            http_client,
            "secret_key",
            gemini_api_key,
            tavily_api_key,
            endpoints,
        )
        .await
        .expect("handler failed");

        assert_eq!(response.status(), 503);
    }

    #[tokio::test]
    async fn test_generate_without_web_search_only_calls_gemini() {
        let mock_server = MockServer::start().await;

        // Gemini APIのみモック（Tavilyは呼ばれないはず）
        let gemini_body = json!({
            "candidates": [{
                "content": {
                    "parts": [{
                        "text": "{\"text\": \"General article on finance.\", \"source_url\": null}"
                    }]
                }
            }]
        });
        Mock::given(method("POST"))
            .and(path("/v1beta/models/gemini-3.5-flash-lite:generateContent"))
            .respond_with(ResponseTemplate::new(200).set_body_json(gemini_body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let payload = r#"{"topic_name": "finance", "use_web_search": false}"#;
        let request = HttpRequest::builder()
            .method(Method::POST)
            .uri("/generate_text")
            .header("x-api-key", "secret_key")
            .header("content-type", "application/json")
            .body(Body::Text(payload.to_string()))
            .expect("failed to build request");

        let http_client = Arc::new(HttpClient::new());
        let gemini_api_key = Arc::new("real_gemini_key".into());
        let tavily_api_key = Arc::new("real_tavily_key".into());
        let endpoints = make_test_endpoints(&mock_server);

        let response = function_handler(
            request,
            http_client,
            "secret_key",
            gemini_api_key,
            tavily_api_key,
            endpoints,
        )
        .await
        .expect("handler failed");

        assert_eq!(response.status(), 200);
        let body_str = match response.body() {
            Body::Text(s) => s.clone(),
            _ => String::new(),
        };
        assert!(body_str.contains("General article on finance."));
    }

    #[tokio::test]
    async fn test_analyze_success() {
        let mock_server = MockServer::start().await;

        let gemini_body = json!({
            "candidates": [{
                "content": {
                    "parts": [{
                        "text": "{\"segments\": [{\"id\": 1, \"text\": \"Segment one\", \"translation\": \"セグメント1\", \"grammar_note\": \"S\"}], \"keywords\": [{\"word\": \"paradigm\", \"meaning\": \"パラダイム\", \"part_of_speech\": \"noun\", \"example\": \"A new paradigm.\"}]}"
                    }]
                }
            }]
        });
        Mock::given(method("POST"))
            .and(path("/v1beta/models/gemini-3.5-flash-lite:generateContent"))
            .respond_with(ResponseTemplate::new(200).set_body_json(gemini_body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let payload = r#"{"action": "analyze", "text": "Segment one is here."}"#;
        let request = HttpRequest::builder()
            .method(Method::POST)
            .uri("/generate_text")
            .header("x-api-key", "secret_key")
            .header("content-type", "application/json")
            .body(Body::Text(payload.to_string()))
            .expect("failed to build request");

        let http_client = Arc::new(HttpClient::new());
        let gemini_api_key = Arc::new("real_gemini_key".into());
        let tavily_api_key = Arc::new("real_tavily_key".into());
        let endpoints = make_test_endpoints(&mock_server);

        let response = function_handler(
            request,
            http_client,
            "secret_key",
            gemini_api_key,
            tavily_api_key,
            endpoints,
        )
        .await
        .expect("handler failed");

        assert_eq!(response.status(), 200);
        let body_str = match response.body() {
            Body::Text(s) => s.clone(),
            _ => String::new(),
        };
        assert!(body_str.contains("paradigm"));
        assert!(body_str.contains("Segment one"));
    }

    #[tokio::test]
    async fn test_gemini_error_returns_500() {
        let mock_server = MockServer::start().await;

        // Geminiが500エラーを返却
        Mock::given(method("POST"))
            .and(path("/v1beta/models/gemini-3.5-flash-lite:generateContent"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Gemini Error"))
            .expect(1)
            .mount(&mock_server)
            .await;

        let payload = r#"{"topic_name": "ai-news", "use_web_search": false}"#;
        let request = HttpRequest::builder()
            .method(Method::POST)
            .uri("/generate_text")
            .header("x-api-key", "secret_key")
            .header("content-type", "application/json")
            .body(Body::Text(payload.to_string()))
            .expect("failed to build request");

        let http_client = Arc::new(HttpClient::new());
        let gemini_api_key = Arc::new("real_gemini_key".into());
        let tavily_api_key = Arc::new("real_tavily_key".into());
        let endpoints = make_test_endpoints(&mock_server);

        let response = function_handler(
            request,
            http_client,
            "secret_key",
            gemini_api_key,
            tavily_api_key,
            endpoints,
        )
        .await
        .expect("handler failed");

        assert_eq!(response.status(), 500);
    }

    #[tokio::test]
    async fn test_gemini_503_retries_and_succeeds() {
        let mock_server = MockServer::start().await;

        // 1回目は503 UNAVAILABLEを返し、2回目のリトライで200 OKを返すシナリオをテスト
        Mock::given(method("POST"))
            .and(path("/v1beta/models/gemini-3.5-flash-lite:generateContent"))
            .respond_with(ResponseTemplate::new(503).set_body_string(
                r#"{"error": {"code": 503, "message": "high demand", "status": "UNAVAILABLE"}}"#,
            ))
            .up_to_n_times(1)
            .mount(&mock_server)
            .await;

        let gemini_body = json!({
            "candidates": [{
                "content": {
                    "parts": [{
                        "text": "{\"text\": \"Recovered after retry!\", \"source_url\": null}"
                    }]
                }
            }]
        });
        Mock::given(method("POST"))
            .and(path("/v1beta/models/gemini-3.5-flash-lite:generateContent"))
            .respond_with(ResponseTemplate::new(200).set_body_json(gemini_body))
            .expect(1)
            .mount(&mock_server)
            .await;

        let http_client = HttpClient::new();
        let gemini_url = format!(
            "{}/v1beta/models/gemini-3.5-flash-lite:generateContent?key=test",
            mock_server.uri()
        );
        let req_body = json!({"contents": []});

        let res = call_gemini_with_retry(&http_client, &gemini_url, &req_body, 3, 10).await;
        assert!(res.is_ok());
        assert!(res.unwrap().contains("Recovered after retry!"));
    }

    #[tokio::test]
    async fn test_gemini_persistent_503_returns_503_after_retries() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1beta/models/gemini-3.5-flash-lite:generateContent"))
            .respond_with(ResponseTemplate::new(503).set_body_string("high demand"))
            .expect(3)
            .mount(&mock_server)
            .await;

        let http_client = HttpClient::new();
        let gemini_url = format!(
            "{}/v1beta/models/gemini-3.5-flash-lite:generateContent?key=test",
            mock_server.uri()
        );
        let req_body = json!({"contents": []});

        let res = call_gemini_with_retry(&http_client, &gemini_url, &req_body, 3, 10).await;
        assert!(res.is_err());
        let (status, msg) = res.unwrap_err();
        assert_eq!(status, 503);
        assert!(msg.contains("high demand"));
    }
}
