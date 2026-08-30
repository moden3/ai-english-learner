use aws_sdk_dynamodb::{Client, types::AttributeValue};
use chrono::Utc;
use lambda_http::{Body, Error, Request, RequestExt, RequestPayloadExt, Response, run, service_fn};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Deserialize, Serialize, Debug)]
struct VocabularyRequest {
    word: String,
    translation: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Vocabulary {
    id: String,
    word: String,
    translation: String,
    created_at: String,
}

/// Vocabulary API のメインハンドラ
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
            // DynamoDBから PK="VOCAB" のデータを検索 (Query)
            let res = client
                .query()
                .table_name(table_name)
                .key_condition_expression("PK = :pk")
                .expression_attribute_values(":pk", AttributeValue::S("VOCAB".into()))
                .send()
                .await;

            match res {
                Ok(out) => {
                    let mut vocabularies = Vec::new();
                    if let Some(items) = out.items {
                        for item in items {
                            if let (
                                Some(AttributeValue::S(sk)),
                                Some(AttributeValue::S(word)),
                                Some(AttributeValue::S(translation)),
                                Some(AttributeValue::S(created_at)),
                            ) = (
                                item.get("SK"),
                                item.get("word"),
                                item.get("translation"),
                                item.get("created_at"),
                            ) {
                                let id = sk.replace("WORD#", "");

                                vocabularies.push(Vocabulary {
                                    id,
                                    word: word.clone(),
                                    translation: translation.clone(),
                                    created_at: created_at.clone(),
                                });
                            }
                        }
                    }
                    let body =
                        serde_json::to_string(&vocabularies).unwrap_or_else(|_| "[]".to_string());
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
        lambda_http::http::Method::POST => match event.payload::<VocabularyRequest>() {
            Ok(Some(vocab_req)) => {
                let id = Uuid::new_v4().to_string();
                let created_at = Utc::now().to_rfc3339();

                let put_req = client
                    .put_item()
                    .table_name(table_name)
                    .item("PK", AttributeValue::S("VOCAB".into()))
                    .item("SK", AttributeValue::S(format!("WORD#{}", id)))
                    .item("word", AttributeValue::S(vocab_req.word.clone()))
                    .item("translation", AttributeValue::S(vocab_req.translation.clone()))
                    .item("created_at", AttributeValue::S(created_at.clone()));

                let res = put_req.send().await;

                match res {
                    Ok(_) => {
                        let new_vocab = Vocabulary {
                            id,
                            word: vocab_req.word,
                            translation: vocab_req.translation,
                            created_at,
                        };
                        let body = serde_json::to_string(&new_vocab).unwrap();
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
        },
        lambda_http::http::Method::DELETE => {
            let path_params = event.path_parameters();
            let mut id = path_params.first("id").map(|s| s.to_string());

            // ローカル環境 (cargo lambda watch) では path_parameters が自動解析されないためのフォールバック
            if id.is_none() {
                let path = event.uri().path();
                if let Some(last_seg) = path.split('/').last() {
                    if !last_seg.is_empty() && last_seg != "vocabulary" {
                        id = Some(last_seg.to_string());
                    }
                }
            }

            match id {
                Some(vocab_id) => {
                    let res = client
                        .delete_item()
                        .table_name(table_name)
                        .key("PK", AttributeValue::S("VOCAB".into()))
                        .key("SK", AttributeValue::S(format!("WORD#{}", vocab_id)))
                        .send()
                        .await;

                    match res {
                        Ok(_) => Ok(Response::builder()
                            .status(204)
                            .body(Body::Empty)
                            .expect("failed to render response")),
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
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let client = Arc::new(Client::new(&config));

    let ssm_client = aws_sdk_ssm::Client::new(&config);
    let api_key = eng_app_backend::get_api_key(&ssm_client).await;
    let api_key = Arc::new(api_key);

    let table_name = "eng-app-table".to_string();

    run(service_fn(move |event| {
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
    use lambda_http::http::{Method, Request as HttpRequest};

    fn create_dummy_client() -> Arc<Client> {
        let config = Config::builder().behavior_version_latest().build();
        Arc::new(Client::from_conf(config))
    }

    #[tokio::test]
    async fn test_get_vocabulary_fails_without_real_db() {
        let request = HttpRequest::builder()
            .method(Method::GET)
            .uri("/vocabulary")
            .body(Body::Empty)
            .expect("failed to build request");

        let client = create_dummy_client();
        let response = function_handler(request, client, "dummy_table", "")
            .await
            .expect("handler failed");

        assert_eq!(
            response.status(),
            500,
            "実際のDBがないため500エラーになるはずです"
        );
    }

    #[tokio::test]
    async fn test_post_vocabulary_fails_without_real_db() {
        let payload = r#"{
            "word": "lazy",
            "translation": "怠惰な"
        }"#;

        let request = HttpRequest::builder()
            .method(Method::POST)
            .uri("/vocabulary")
            .header("content-type", "application/json")
            .body(Body::Text(payload.to_string()))
            .expect("failed to build request");

        let client = create_dummy_client();
        let response = function_handler(request, client, "dummy_table", "")
            .await
            .expect("handler failed");

        assert_eq!(
            response.status(),
            500,
            "実際のDBがないため500エラーになるはずです"
        );
    }
}
