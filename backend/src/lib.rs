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
    // 1. 環境変数 (ローカル開発用。空文字の場合は未設定とみなす)
    if let Ok(val) = std::env::var("APP_API_KEY")
        && !val.is_empty()
    {
        return val;
    }

    // 2. AWS SSM Parameter Store
    fetch_ssm_parameter(ssm_client, "/eng-app/api-key", "CHANGE_ME_INITIAL_VALUE").await
}

/// Gemini API キーを取得する（ローカル開発用環境変数 → AWS SSM の順で探す）
pub async fn get_gemini_api_key(ssm_client: &SsmClient) -> String {
    // 1. 環境変数 (ローカル開発用。空文字の場合は未設定とみなす)
    if let Ok(val) = std::env::var("GEMINI_API_KEY")
        && !val.is_empty()
    {
        return val;
    }

    // 2. AWS SSM Parameter Store
    fetch_ssm_parameter(ssm_client, "/eng-app/gemini-api-key", "").await
}

/// Tavily API キーを取得する（ローカル開発用環境変数 → AWS SSM の順で探す）
pub async fn get_tavily_api_key(ssm_client: &SsmClient) -> String {
    // 1. 環境変数 (ローカル開発用。空文字の場合は未設定とみなす)
    if let Ok(val) = std::env::var("TAVILY_API_KEY")
        && !val.is_empty()
    {
        return val;
    }

    // 2. AWS SSM Parameter Store
    fetch_ssm_parameter(ssm_client, "/eng-app/tavily-api-key", "").await
}

/// AWS SSM Parameter Store から指定したパラメータを取得する（失敗時は default_val にフォールバック）
pub async fn fetch_ssm_parameter(
    ssm_client: &SsmClient,
    param_name: &str,
    default_val: &str,
) -> String {
    let ssm_res = ssm_client
        .get_parameter()
        .name(param_name)
        .with_decryption(true)
        .send()
        .await;

    match ssm_res {
        Ok(out) => out
            .parameter
            .and_then(|p| p.value)
            .unwrap_or_else(|| default_val.to_string()),
        Err(e) => {
            println!("SSM error (could not read {}): {:?}", param_name, e);
            default_val.to_string()
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

/// 最新ニュース記事リストから1件を選定し、Geminiプロンプト用コンテキスト文字列と参照元URLに整形する
///
/// 【単一記事フォーカスの設計理由】
/// 複数の異なるニュース記事を結合して150〜250語の短い英文にまとめさせると、
/// 話題が分散して起承転結のない散漫な文章になり、さらに `source_url` と本文の内容が乖離してしまう。
/// そのため、取得した候補記事の中から「本文が十分に充実している（100文字以上の）記事」を優先して
/// ランダムに1件だけを選び出し、最大1000文字の十分なコンテキストと1対1で一致する `source_url` を返す。
pub fn format_news_context(results: &[TavilyResult]) -> (String, String) {
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as usize)
        .unwrap_or(0);
    format_news_context_with_seed(results, seed)
}

/// テストで決定的な検証を行えるよう、選択シード値を注入可能にした内部関数
pub fn format_news_context_with_seed(results: &[TavilyResult], seed: usize) -> (String, String) {
    if results.is_empty() {
        return (String::new(), String::new());
    }

    // 本文が極端に短い（100文字未満の）スニペット記事を除外した候補リストを作成
    // （すべて100文字未満だった場合は全記事を候補にする）
    let rich_candidates: Vec<&TavilyResult> = results
        .iter()
        .filter(|r| r.content.chars().count() >= 100)
        .collect();

    let selected = if !rich_candidates.is_empty() {
        rich_candidates[seed % rich_candidates.len()]
    } else {
        &results[seed % results.len()]
    };

    let source_url = selected.url.clone();
    // 1件に集中するため、従来の400文字から最大1000文字までコンテキストを拡大
    let truncated_content: String = selected.content.chars().take(1000).collect();
    let news_context = format!(
        "Title: {}\nURL: {}\nContent: {}",
        selected.title, selected.url, truncated_content
    );

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
            "You are an English teacher. Based on the following single latest news article retrieved from the web, write a short, highly professional business English article (between 150 to 250 words) about '{}' suitable for upper-intermediate to advanced business English learners (TOEIC 800+, CEFR B2-C1). Focus deeply on this single news story and structure the article with a clear narrative flow (introduction/background, key developments, and business implications). Incorporate practical and advanced business vocabulary from the news context. Do not fabricate facts; base the article strictly on the provided news content.

--- Latest News Article ---
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
    if let Some(start) = cleaned.find('{')
        && let Some(end) = cleaned.rfind('}')
        && start <= end
    {
        cleaned = &cleaned[start..=end];
    }

    serde_json::from_str(cleaned).map_err(|e| format!("Failed to parse JSON payload: {:?}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_sdk_ssm::Config as SsmConfig;
    use aws_sdk_ssm::config::{Credentials, Region};
    use lambda_http::http::Request as HttpRequest;
    use serde_json::json;
    use wiremock::matchers::{any, method};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn create_mock_ssm_client(mock_server: &MockServer) -> SsmClient {
        let creds = Credentials::new("test", "test", None, None, "test");
        let config = SsmConfig::builder()
            .behavior_version_latest()
            .region(Region::new("ap-northeast-1"))
            .credentials_provider(creds)
            .endpoint_url(mock_server.uri())
            .build();
        SsmClient::from_conf(config)
    }

    #[tokio::test]
    async fn test_get_keys_from_ssm_and_fallback() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(any())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "Parameter": {
                    "Name": "/eng-app/key",
                    "Type": "SecureString",
                    "Value": "ssm-secret-value"
                }
            })))
            .mount(&mock_server)
            .await;

        let ssm_client = create_mock_ssm_client(&mock_server);
        assert_eq!(
            fetch_ssm_parameter(&ssm_client, "/eng-app/api-key", "CHANGE_ME_INITIAL_VALUE").await,
            "ssm-secret-value"
        );
        assert_eq!(get_api_key(&ssm_client).await, "ssm-secret-value");
        assert_eq!(get_gemini_api_key(&ssm_client).await, "ssm-secret-value");
        assert_eq!(get_tavily_api_key(&ssm_client).await, "ssm-secret-value");

        // SSMエラー時のフォールバック検証
        let err_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(any())
            .respond_with(ResponseTemplate::new(500))
            .mount(&err_server)
            .await;
        let err_ssm_client = create_mock_ssm_client(&err_server);
        assert_eq!(
            fetch_ssm_parameter(
                &err_ssm_client,
                "/eng-app/api-key",
                "CHANGE_ME_INITIAL_VALUE"
            )
            .await,
            "CHANGE_ME_INITIAL_VALUE"
        );
        assert_eq!(
            fetch_ssm_parameter(&err_ssm_client, "/eng-app/gemini-api-key", "").await,
            ""
        );
    }

    #[test]
    fn test_format_news_context_empty() {
        let (context, url) = format_news_context(&[]);
        assert!(context.is_empty());
        assert!(url.is_empty());
    }

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
    fn test_format_news_context_selects_single_article_and_truncates_at_1000_chars() {
        let long_text = "A".repeat(1200);
        let results = vec![
            TavilyResult {
                title: "First Rich News".into(),
                content: long_text,
                url: "https://example.com/first".into(),
            },
            TavilyResult {
                title: "Second Short News".into(),
                content: "Too short (< 100 chars)".into(),
                url: "https://example.com/second".into(),
            },
        ];

        // 2件目は100文字未満のためフィルタ除外され、必ず1件目の充実した記事が選ばれる
        let (context, source_url) = format_news_context(&results);
        assert_eq!(source_url, "https://example.com/first");
        assert!(context.contains("Title: First Rich News"));
        // 複数の記事が混ざっていないこと（2件目が含まれないこと）を確認
        assert!(!context.contains("Second Short News"));
        // 1000文字でトランケーションされているか確認
        assert!(!context.contains(&"A".repeat(1001)));
        assert!(context.contains(&"A".repeat(1000)));
    }

    #[test]
    fn test_format_news_context_with_seed_rotates_among_rich_articles() {
        let results = vec![
            TavilyResult {
                title: "Article One".into(),
                content: "B".repeat(150),
                url: "https://example.com/1".into(),
            },
            TavilyResult {
                title: "Article Two".into(),
                content: "C".repeat(150),
                url: "https://example.com/2".into(),
            },
        ];

        let (ctx0, url0) = format_news_context_with_seed(&results, 0);
        assert_eq!(url0, "https://example.com/1");
        assert!(ctx0.contains("Title: Article One"));
        assert!(!ctx0.contains("Title: Article Two"));

        let (ctx1, url1) = format_news_context_with_seed(&results, 1);
        assert_eq!(url1, "https://example.com/2");
        assert!(ctx1.contains("Title: Article Two"));
        assert!(!ctx1.contains("Title: Article One"));
    }

    #[test]
    fn test_format_news_context_fallback_when_all_articles_are_short() {
        let results = vec![TavilyResult {
            title: "Short Article".into(),
            content: "Short content".into(),
            url: "https://example.com/short".into(),
        }];

        let (context, source_url) = format_news_context_with_seed(&results, 0);
        assert_eq!(source_url, "https://example.com/short");
        assert!(context.contains("Title: Short Article"));
    }

    #[test]
    fn test_format_news_context_multibyte_utf8_does_not_panic() {
        // マルチバイト文字（’ や — や日本語）が1000文字境界をまたいでもパニックしないことを検証
        let multibyte_text = "’—€あ".repeat(300); // 1200 chars
        let results = vec![TavilyResult {
            title: "Multibyte News".into(),
            content: multibyte_text,
            url: "https://example.com/utf8".into(),
        }];

        let (context, source_url) = format_news_context(&results);
        assert_eq!(source_url, "https://example.com/utf8");
        assert!(context.contains(&"’—€あ".repeat(250))); // 1000 chars
        assert!(!context.contains(&"’—€あ".repeat(251))); // 1004 chars
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
