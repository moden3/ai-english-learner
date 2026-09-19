use aws_sdk_ssm::Client as SsmClient;
use lambda_http::Request;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct TavilyResult {
    pub title: String,
    pub content: String,
    pub url: String,
}

pub async fn get_api_key(ssm_client: &SsmClient) -> String {
    // 1. 環境変数 (ローカル開発用)
    if let Ok(val) = std::env::var("APP_API_KEY") {
        return val;
    }

    // 2. AWS SSM Parameter Store
    let ssm_res = ssm_client
        .get_parameter()
        .name("/eng-app/api-key")
        .with_decryption(true)
        .send()
        .await;

    match ssm_res {
        Ok(out) => out.parameter.unwrap().value.unwrap(),
        Err(e) => {
            println!("SSM error (could not read API Key): {:?}", e);
            "CHANGE_ME_INITIAL_VALUE".to_string()
        }
    }
}

/// Gemini API キーを取得する（ローカル開発用環境変数 → AWS SSM の順で探す）
pub async fn get_gemini_api_key(ssm_client: &SsmClient) -> String {
    // 1. 環境変数 (ローカル開発用)
    if let Ok(val) = std::env::var("GEMINI_API_KEY") {
        return val;
    }

    // 2. AWS SSM Parameter Store
    let ssm_res = ssm_client
        .get_parameter()
        .name("/eng-app/gemini-api-key")
        .with_decryption(true)
        .send()
        .await;

    match ssm_res {
        Ok(out) => out.parameter.unwrap().value.unwrap(),
        Err(e) => {
            println!("SSM error (could not read Gemini API Key): {:?}", e);
            String::new()
        }
    }
}

/// Tavily API キーを取得する（ローカル開発用環境変数 → AWS SSM の順で探す）
pub async fn get_tavily_api_key(ssm_client: &SsmClient) -> String {
    // 1. 環境変数 (ローカル開発用)
    if let Ok(val) = std::env::var("TAVILY_API_KEY") {
        return val;
    }

    // 2. AWS SSM Parameter Store
    let ssm_res = ssm_client
        .get_parameter()
        .name("/eng-app/tavily-api-key")
        .with_decryption(true)
        .send()
        .await;

    match ssm_res {
        Ok(out) => out.parameter.unwrap().value.unwrap(),
        Err(e) => {
            println!("SSM error (could not read Tavily API Key): {:?}", e);
            // キーがなくても検索なしモードでフォールバックするため空文字を返す
            String::new()
        }
    }
}

pub fn validate_api_key(req: &Request, expected_key: &str) -> bool {
    if expected_key.is_empty() {
        return false;
    }
    let provided_key = req
        .headers()
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    !provided_key.is_empty() && provided_key == expected_key
}

/// 最新ニュース記事リストをGeminiプロンプト用コンテキスト文字列と代表URLに整形する
/// 戻り値: (news_context_string, source_url)
pub fn format_news_context(results: &[TavilyResult]) -> (String, String) {
    if results.is_empty() {
        return (String::new(), String::new());
    }

    let source_url = results[0].url.clone();
    let news_context = results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let truncated_content = if r.content.len() > 400 {
                &r.content[..400]
            } else {
                &r.content
            };
            format!(
                "[{}] Title: {}\n    URL: {}\n    Content: {}",
                i + 1,
                r.title,
                r.url,
                truncated_content
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    (news_context, source_url)
}

/// 英文記事生成プロンプトを作成する（Web検索あり／なし両対応）
pub fn build_generate_prompt(
    topic_name: &str,
    news_context: Option<&str>,
    source_url: Option<&str>,
) -> String {
    match (news_context, source_url) {
        (Some(context), Some(url)) => format!(
            "You are an English teacher. Based on the following latest news articles retrieved from the web, write a short, highly professional business English article (between 150 to 250 words) about '{}' suitable for upper-intermediate to advanced business English learners (TOEIC 800+, CEFR B2-C1). Incorporate practical and advanced business vocabulary from the news context. Do not fabricate facts; base the article on the provided news content.

--- Latest News Context ---
{}
---

You MUST output strictly in valid JSON format matching this schema exactly:
{{
  \"text\": \"The generated english article based on the news...\",
  \"source_url\": \"{}\"
}}",
            topic_name, context, url
        ),
        _ => format!(
            "You are an English teacher. Write a short, highly professional business English article (between 150 to 250 words) about '{}' suitable for upper-intermediate to advanced business English learners (TOEIC 800+, CEFR B2-C1). Incorporate practical and advanced business vocabulary.
Focus on real-world business contexts, modern industry trends, and practical vocabulary.
You MUST output strictly in valid JSON format matching this schema exactly:
{{
  \"text\": \"The generated english article...\",
  \"source_url\": null
}}",
            topic_name
        ),
    }
}

/// 構文解析・語彙抽出プロンプトを作成する
pub fn build_analyze_prompt(text_to_analyze: &str) -> String {
    format!(
        "You are an English teacher. Break down the following english text into segments (chunks of meaning), translate each segment into Japanese, and provide a short grammar note for each. Also, extract highly advanced business keywords or idioms (CEFR B2-C1 level) from the text. Skip basic and intermediate words (A1-B2) as the user already knows them (TOEIC 800+). Do NOT include literal slash characters ('/') in the text.
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
}

/// LLMからの生テキストからJSON部分を抽出し、Valueとしてパースする
pub fn extract_json_payload(raw_text: &str) -> Result<serde_json::Value, String> {
    let mut cleaned = raw_text.trim();

    // Markdownコードブロックや余分なテキストを除去し、最外殻のJSON波括弧を抽出
    if let Some(start) = cleaned.find('{') {
        if let Some(end) = cleaned.rfind('}') {
            if start <= end {
                cleaned = &cleaned[start..=end];
            }
        }
    }

    serde_json::from_str(cleaned).map_err(|e| format!("Failed to parse JSON payload: {:?}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lambda_http::http::Request as HttpRequest;

    #[test]
    fn test_validate_api_key_valid() {
        let req = HttpRequest::builder()
            .header("x-api-key", "secret123")
            .body(lambda_http::Body::Empty)
            .unwrap();
        assert!(validate_api_key(&req, "secret123"));
    }

    #[test]
    fn test_validate_api_key_invalid() {
        let req = HttpRequest::builder()
            .header("x-api-key", "wrong_key")
            .body(lambda_http::Body::Empty)
            .unwrap();
        assert!(!validate_api_key(&req, "secret123"));
    }

    #[test]
    fn test_validate_api_key_missing_header() {
        let req = HttpRequest::builder()
            .body(lambda_http::Body::Empty)
            .unwrap();
        assert!(!validate_api_key(&req, "secret123"));
    }

    #[test]
    fn test_validate_api_key_empty_expected() {
        let req = HttpRequest::builder()
            .body(lambda_http::Body::Empty)
            .unwrap();
        // 期待値が空文字の場合、ヘッダー欠落でも true になってはならない
        assert!(!validate_api_key(&req, ""));
    }

    #[test]
    fn test_format_news_context_truncation_and_url() {
        let long_text = "A".repeat(500);
        let results = vec![
            TavilyResult {
                title: "First News".into(),
                content: long_text,
                url: "https://example.com/first".into(),
            },
            TavilyResult {
                title: "Second News".into(),
                content: "Short content".into(),
                url: "https://example.com/second".into(),
            },
        ];

        let (context, source_url) = format_news_context(&results);
        assert_eq!(source_url, "https://example.com/first");
        assert!(context.contains("[1] Title: First News"));
        assert!(context.contains("[2] Title: Second News"));
        // 400文字でトランケーションされているか確認
        assert!(!context.contains(&"A".repeat(401)));
        assert!(context.contains(&"A".repeat(400)));
    }

    #[test]
    fn test_build_generate_prompt_without_news() {
        let prompt = build_generate_prompt("Fintech", None, None);
        assert!(prompt.contains("'Fintech'"));
        assert!(prompt.contains("150 to 250 words"));
        assert!(prompt.contains("\"source_url\": null"));
        assert!(!prompt.contains("Latest News Context"));
    }

    #[test]
    fn test_build_generate_prompt_with_news() {
        let prompt = build_generate_prompt(
            "AI in Healthcare",
            Some("News Content Here"),
            Some("https://news.example.com"),
        );
        assert!(prompt.contains("'AI in Healthcare'"));
        assert!(prompt.contains("News Content Here"));
        assert!(prompt.contains("\"source_url\": \"https://news.example.com\""));
        assert!(prompt.contains("150 to 250 words"));
    }

    #[test]
    fn test_build_analyze_prompt() {
        let prompt = build_analyze_prompt("Artificial Intelligence is growing rapidly.");
        assert!(prompt.contains("Artificial Intelligence is growing rapidly."));
        assert!(prompt.contains("CEFR B2-C1 level"));
        assert!(prompt.contains("\"segments\""));
        assert!(prompt.contains("\"keywords\""));
    }

    #[test]
    fn test_extract_json_payload_clean() {
        let raw = r#"{"text": "Hello world", "source_url": null}"#;
        let val = extract_json_payload(raw).expect("should parse");
        assert_eq!(val["text"], "Hello world");
        assert!(val["source_url"].is_null());
    }

    #[test]
    fn test_extract_json_payload_markdown_wrapped() {
        let raw = "Here is the response:\n```json\n{\"text\": \"From markdown\", \"source_url\": \"https://test.com\"}\n```\nHope this helps!";
        let val = extract_json_payload(raw).expect("should parse markdown wrapped JSON");
        assert_eq!(val["text"], "From markdown");
        assert_eq!(val["source_url"], "https://test.com");
    }

    #[test]
    fn test_extract_json_payload_invalid() {
        let raw = "This is not JSON at all";
        assert!(extract_json_payload(raw).is_err());
    }
}
