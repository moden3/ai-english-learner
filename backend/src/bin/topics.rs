use aws_sdk_dynamodb::{Client, types::AttributeValue};
use chrono::Utc;
use lambda_http::{Body, Error, Request, RequestExt, RequestPayloadExt, Response, run, service_fn};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Deserialize, Serialize, Debug)]
struct TopicRequest {
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Topic {
    id: String,
    name: String,
    created_at: String,
}

/// トピックAPIのメインハンドラ
async fn function_handler(
    event: Request,
    client: Arc<Client>,
    table_name: &str,
    api_key: &str,
) -> Result<Response<Body>, Error> {
    if !eng_app_backend::validate_api_key(&event, api_key) {
        return Ok(Response::builder()
            .status(401)
            .body(Body::Text("Unauthorized".into()))
            .expect("failed to render response"));
    }

    match *event.method() {
        lambda_http::http::Method::GET => {
            // DynamoDBから PK="TOPIC" のデータを検索 (Query) します
            let res = client
                .query()
                .table_name(table_name)
                .key_condition_expression("PK = :pk")
                .expression_attribute_values(":pk", AttributeValue::S("TOPIC".into()))
                .send()
                .await;

            match res {
                Ok(out) => {
                    let mut topics = Vec::new();
                    // 取得したアイテム（HashMap）をTopic構造体に変換
                    if let Some(items) = out.items {
                        for item in items {
                            if let (
                                Some(AttributeValue::S(sk)),
                                Some(AttributeValue::S(name)),
                                Some(AttributeValue::S(created_at)),
                            ) = (item.get("SK"), item.get("name"), item.get("created_at"))
                            {
                                let id = sk.replace("TOPIC#", "");
                                topics.push(Topic {
                                    id,
                                    name: name.clone(),
                                    created_at: created_at.clone(),
                                });
                            }
                        }
                    }
                    let body = serde_json::to_string(&topics).unwrap_or_else(|_| "[]".to_string());
                    Ok(Response::builder()
                        .status(200)
                        .header("content-type", "application/json")
                        .body(Body::Text(body))
                        .expect("failed to render response"))
                }
                Err(e) => {
                    println!("DynamoDB error: {:?}", e);
                    Ok(Response::builder()
                        .status(500)
                        .body(Body::Text("Internal Server Error".into()))
                        .expect("failed to render response"))
                }
            }
        }
        lambda_http::http::Method::POST => {
            match event.payload::<TopicRequest>() {
                Ok(Some(topic_req)) => {
                    let id = Uuid::new_v4().to_string();
                    let created_at = Utc::now().to_rfc3339();

                    // DynamoDBに保存 (PutItem)
                    let res = client
                        .put_item()
                        .table_name(table_name)
                        .item("PK", AttributeValue::S("TOPIC".into()))
                        .item("SK", AttributeValue::S(format!("TOPIC#{}", id)))
                        .item("name", AttributeValue::S(topic_req.name.clone()))
                        .item("created_at", AttributeValue::S(created_at.clone()))
                        .send()
                        .await;

                    match res {
                        Ok(_) => {
                            let new_topic = Topic {
                                id,
                                name: topic_req.name,
                                created_at,
                            };
                            let body = serde_json::to_string(&new_topic).unwrap();
                            Ok(Response::builder()
                                .status(201)
                                .header("content-type", "application/json")
                                .body(Body::Text(body))
                                .expect("failed to render response"))
                        }
                        Err(e) => {
                            println!("DynamoDB error: {:?}", e);
                            Ok(Response::builder()
                                .status(500)
                                .body(Body::Text("Internal Server Error".into()))
                                .expect("failed to render response"))
                        }
                    }
                }
                _ => Ok(Response::builder()
                    .status(400)
                    .body(Body::Text("Invalid Request Body".into()))
                    .expect("failed to render response")),
            }
        }
        lambda_http::http::Method::DELETE => {
            // Path Parameter から "id" を取得する
            let path_params = event.path_parameters();
            let mut id = path_params.first("id").map(|s| s.to_string());

            // ローカル環境 (cargo lambda watch) では path_parameters が自動解析されないためのフォールバック
            if id.is_none() {
                let path = event.uri().path();
                if let Some(last_seg) = path.split('/').next_back()
                    && !last_seg.is_empty()
                    && last_seg != "topics"
                {
                    id = Some(last_seg.to_string());
                }
            }

            match id {
                Some(topic_id) => {
                    // DynamoDBから削除 (DeleteItem)
                    let res = client
                        .delete_item()
                        .table_name(table_name)
                        .key("PK", AttributeValue::S("TOPIC".into()))
                        .key("SK", AttributeValue::S(format!("TOPIC#{}", topic_id)))
                        .send()
                        .await;

                    match res {
                        Ok(_) => {
                            Ok(Response::builder()
                                .status(204) // No Content (削除成功)
                                .body(Body::Empty)
                                .expect("failed to render response"))
                        }
                        Err(e) => {
                            println!("DynamoDB error: {:?}", e);
                            Ok(Response::builder()
                                .status(500)
                                .body(Body::Text("Internal Server Error".into()))
                                .expect("failed to render response"))
                        }
                    }
                }
                None => Ok(Response::builder()
                    .status(400)
                    .body(Body::Text("Missing 'id' parameter".into()))
                    .expect("failed to render response")),
            }
        }
        _ => Ok(Response::builder()
            .status(405)
            .body(Body::Text("Method Not Allowed".into()))
            .expect("failed to render response")),
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // アプリ起動時（コールドスタート時）に1度だけAWSの設定とDynamoDBクライアントを読み込む
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let client = Arc::new(Client::new(&config));

    let ssm_client = aws_sdk_ssm::Client::new(&config);
    let api_key = eng_app_backend::get_api_key(&ssm_client).await;
    let api_key = Arc::new(api_key);

    // テーブル名は Terraform で作成した「eng-app-table」です
    let table_name = "eng-app-table".to_string();

    // 毎回のリクエスト処理
    run(service_fn(move |event| {
        // スレッド間で安全に共有するためにクローンしてハンドラに渡す
        let client = client.clone();
        let table_name = table_name.clone();
        let api_key = api_key.clone();
        async move { function_handler(event, client, &table_name, &api_key).await }
    }))
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_sdk_dynamodb::Config;
    use aws_sdk_dynamodb::config::{Credentials, Region};
    use lambda_http::http::{Method, Request as HttpRequest};
    use serde_json::json;
    use wiremock::matchers::{any, method};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // テスト用のDynamoDBクライアント（接続先URLだけをローカルの wiremock に向ける）
    fn create_mock_client(mock_server: &MockServer) -> Arc<Client> {
        let creds = Credentials::new("test", "test", None, None, "test");
        let config = Config::builder()
            .behavior_version_latest()
            .region(Region::new("ap-northeast-1"))
            .credentials_provider(creds)
            .endpoint_url(mock_server.uri())
            .build();
        Arc::new(Client::from_conf(config))
    }

    #[tokio::test]
    async fn test_topics_unauthorized_when_api_key_invalid() {
        let mock_server = MockServer::start().await;
        let client = create_mock_client(&mock_server);

        let request = HttpRequest::builder()
            .method(Method::GET)
            .uri("/topics")
            .header("x-api-key", "wrong_key")
            .body(Body::Empty)
            .expect("failed to build request");

        let response = function_handler(request, client, "eng-app-table", "secret_key")
            .await
            .expect("handler failed");

        assert_eq!(response.status(), 401);
    }

    #[tokio::test]
    async fn test_get_topics_success() {
        let mock_server = MockServer::start().await;
        let ddb_response = json!({
            "Items": [
                {
                    "PK": { "S": "TOPIC" },
                    "SK": { "S": "TOPIC#uuid-1" },
                    "name": { "S": "Quantum Computing" },
                    "created_at": { "S": "2026-11-22T00:00:00Z" }
                }
            ],
            "Count": 1,
            "ScannedCount": 1
        });

        Mock::given(method("POST"))
            .and(any())
            .respond_with(ResponseTemplate::new(200).set_body_json(ddb_response))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = create_mock_client(&mock_server);
        let request = HttpRequest::builder()
            .method(Method::GET)
            .uri("/topics")
            .header("x-api-key", "test_key")
            .body(Body::Empty)
            .expect("failed to build request");

        let response = function_handler(request, client, "eng-app-table", "test_key")
            .await
            .expect("handler failed");

        assert_eq!(response.status(), 200);
        let body_str = match response.body() {
            Body::Text(s) => s.clone(),
            _ => String::new(),
        };
        assert!(body_str.contains("uuid-1"));
        assert!(body_str.contains("Quantum Computing"));
    }

    #[tokio::test]
    async fn test_get_topics_dynamodb_error_returns_500() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(any())
            .respond_with(ResponseTemplate::new(500).set_body_json(json!({
                "__type": "com.amazonaws.dynamodb.v20120810#InternalServerError",
                "message": "Database error"
            })))
            .mount(&mock_server)
            .await;

        let client = create_mock_client(&mock_server);
        let request = HttpRequest::builder()
            .method(Method::GET)
            .uri("/topics")
            .header("x-api-key", "test_key")
            .body(Body::Empty)
            .expect("failed to build request");

        let response = function_handler(request, client, "eng-app-table", "test_key")
            .await
            .expect("handler failed");

        assert_eq!(response.status(), 500);
    }

    #[tokio::test]
    async fn test_post_topics_success() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(any())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = create_mock_client(&mock_server);
        let request = HttpRequest::builder()
            .method(Method::POST)
            .uri("/topics")
            .header("x-api-key", "test_key")
            .header("content-type", "application/json")
            .body(Body::Text(r#"{"name": "Cybersecurity"}"#.to_string()))
            .expect("failed to build request");

        let response = function_handler(request, client, "eng-app-table", "test_key")
            .await
            .expect("handler failed");

        assert_eq!(response.status(), 201);
        let body_str = match response.body() {
            Body::Text(s) => s.clone(),
            _ => String::new(),
        };
        assert!(body_str.contains("Cybersecurity"));
    }

    #[tokio::test]
    async fn test_post_topics_invalid_body_returns_400() {
        let mock_server = MockServer::start().await;
        let client = create_mock_client(&mock_server);

        let request = HttpRequest::builder()
            .method(Method::POST)
            .uri("/topics")
            .header("x-api-key", "test_key")
            .header("content-type", "application/json")
            .body(Body::Text("invalid json".to_string()))
            .expect("failed to build request");

        let response = function_handler(request, client, "eng-app-table", "test_key")
            .await
            .expect("handler failed");

        assert_eq!(response.status(), 400);
    }

    #[tokio::test]
    async fn test_post_topics_dynamodb_error_returns_500() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(any())
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let client = create_mock_client(&mock_server);
        let request = HttpRequest::builder()
            .method(Method::POST)
            .uri("/topics")
            .header("x-api-key", "test_key")
            .header("content-type", "application/json")
            .body(Body::Text(r#"{"name": "Cybersecurity"}"#.to_string()))
            .expect("failed to build request");

        let response = function_handler(request, client, "eng-app-table", "test_key")
            .await
            .expect("handler failed");

        assert_eq!(response.status(), 500);
    }

    #[tokio::test]
    async fn test_delete_topic_success_with_uri_fallback() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(any())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .expect(1)
            .mount(&mock_server)
            .await;

        let client = create_mock_client(&mock_server);
        let request = HttpRequest::builder()
            .method(Method::DELETE)
            .uri("/topics/topic-uuid-123")
            .header("x-api-key", "test_key")
            .body(Body::Empty)
            .expect("failed to build request");

        let response = function_handler(request, client, "eng-app-table", "test_key")
            .await
            .expect("handler failed");

        assert_eq!(response.status(), 204);
    }

    #[tokio::test]
    async fn test_delete_topic_missing_id_returns_400() {
        let mock_server = MockServer::start().await;
        let client = create_mock_client(&mock_server);

        let request = HttpRequest::builder()
            .method(Method::DELETE)
            .uri("/topics")
            .header("x-api-key", "test_key")
            .body(Body::Empty)
            .expect("failed to build request");

        let response = function_handler(request, client, "eng-app-table", "test_key")
            .await
            .expect("handler failed");

        assert_eq!(response.status(), 400);
    }

    #[tokio::test]
    async fn test_delete_topic_dynamodb_error_returns_500() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(any())
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let client = create_mock_client(&mock_server);
        let request = HttpRequest::builder()
            .method(Method::DELETE)
            .uri("/topics/topic-uuid-123")
            .header("x-api-key", "test_key")
            .body(Body::Empty)
            .expect("failed to build request");

        let response = function_handler(request, client, "eng-app-table", "test_key")
            .await
            .expect("handler failed");

        assert_eq!(response.status(), 500);
    }

    #[tokio::test]
    async fn test_topics_unsupported_method_returns_405() {
        let mock_server = MockServer::start().await;
        let client = create_mock_client(&mock_server);

        let request = HttpRequest::builder()
            .method(Method::PUT)
            .uri("/topics")
            .header("x-api-key", "test_key")
            .body(Body::Empty)
            .expect("failed to build request");

        let response = function_handler(request, client, "eng-app-table", "test_key")
            .await
            .expect("handler failed");

        assert_eq!(response.status(), 405);
    }
}
