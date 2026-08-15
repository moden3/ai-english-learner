use aws_sdk_ssm::Client as SsmClient;
use lambda_http::{Body, Error, Request, RequestPayloadExt, Response, run, service_fn};
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

#[derive(Deserialize, Debug)]
struct GenerateRequest {
    topic_name_en: String,
}

// クライアントへ返す最終的なレスポンス構造体
// （AIにもこのJSONの形式で回答を作成してもらいます）
#[derive(Serialize, Deserialize, Debug)]
struct GeneratedResult {
    text: String,
    source_url: Option<String>,
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
) -> Result<Response<Body>, Error> {
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

    // プロンプトの作成：文字数に幅を持たせ、厳密にJSONを返すように指示
    let prompt = format!(
        "You are an English teacher. Write a short English article (between 150 to 250 words) about '{}' suitable for B1 level english learners. 
Use the latest news via your Google Search tool if possible. 
You MUST output strictly in valid JSON format matching this schema exactly:
{{
  \"text\": \"The generated english article...\",
  \"source_url\": \"The URL of the news source you referenced, or null if not applicable\"
}}",
        req_body.topic_name_en
    );

    let gemini_url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent?key={}",
        api_key
    );

    // Gemini APIリクエストの生成
    // ここで tools に googleSearch を指定し、temperature でランダム性を上げます
    let request_body = json!({
        "contents": [{
            "parts": [{"text": prompt}]
        }],
        "tools": [{
            "googleSearch": {}
        }],
        "generationConfig": {
            "temperature": 0.9,
            "responseMimeType": "application/json"
        }
    });

    let res = http_client.post(&gemini_url).json(&request_body).send().await;

    match res {
        Ok(resp) => {
            if resp.status().is_success() {
                let gemini_resp: GeminiResponse = resp.json().await?;
                // AIが返してきた文字列（JSONとして指示したので中身はJSON文字列のはず）
                let generated_json_text = gemini_resp.candidates
                    .and_then(|c| c.into_iter().next())
                    .and_then(|c| c.content.parts.into_iter().next())
                    .map(|p| p.text)
                    .unwrap_or_else(|| "{}".to_string());

                // AIの返答（JSON文字列）をRustのGeneratedResult構造体にパースする
                let out: GeneratedResult = serde_json::from_str(&generated_json_text).unwrap_or_else(|_| {
                    GeneratedResult {
                        text: "Failed to parse AI response".into(),
                        source_url: None,
                    }
                });

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

    run(service_fn(move |event| {
        let ssm_client = ssm_client.clone();
        let http_client = http_client.clone();
        async move { function_handler(event, ssm_client, http_client).await }
    })).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use lambda_http::http::{Method, Request as HttpRequest};

    #[tokio::test]
    async fn test_post_generate_text_fails_with_dummy_key() {
        let payload = r#"{
            "topic_name_en": "business"
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

        let response = function_handler(request, ssm_client, http_client).await.expect("handler failed");

        assert_eq!(response.status(), 500, "ダミーキーでの通信になるため500エラーになるはずです");
    }
}
