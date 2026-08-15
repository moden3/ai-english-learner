use aws_sdk_dynamodb::{Client, types::AttributeValue};
use chrono::Utc;
use lambda_http::{Body, Error, Request, RequestExt, RequestPayloadExt, Response, run, service_fn};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Deserialize, Serialize, Debug)]
struct VocabularyRequest {
    word: String,
    meaning: String,
    part_of_speech: String,
    example: String,
    source_text_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Vocabulary {
    id: String,
    word: String,
    meaning: String,
    part_of_speech: String,
    example: String,
    source_text_id: Option<String>,
    created_at: String,
}

/// Vocabulary API のメインハンドラ
async fn function_handler(
    event: Request,
    client: Arc<Client>,
    table_name: &str,
) -> Result<Response<Body>, Error> {
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
                                Some(AttributeValue::S(meaning)),
                                Some(AttributeValue::S(part_of_speech)),
                                Some(AttributeValue::S(example)),
                                Some(AttributeValue::S(created_at)),
                            ) = (
                                item.get("SK"),
                                item.get("word"),
                                item.get("meaning"),
                                item.get("part_of_speech"),
                                item.get("example"),
                                item.get("created_at"),
                            ) {
                                let id = sk.replace("WORD#", "");

                                // source_text_id は Optional として扱う
                                let source_text_id = item.get("source_text_id").and_then(|v| {
                                    if let AttributeValue::S(s) = v {
                                        Some(s.clone())
                                    } else {
                                        None
                                    }
                                });

                                vocabularies.push(Vocabulary {
                                    id,
                                    word: word.clone(),
                                    meaning: meaning.clone(),
                                    part_of_speech: part_of_speech.clone(),
                                    example: example.clone(),
                                    source_text_id,
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

                let mut put_req = client
                    .put_item()
                    .table_name(table_name)
                    .item("PK", AttributeValue::S("VOCAB".into()))
                    .item("SK", AttributeValue::S(format!("WORD#{}", id)))
                    .item("word", AttributeValue::S(vocab_req.word.clone()))
                    .item("meaning", AttributeValue::S(vocab_req.meaning.clone()))
                    .item(
                        "part_of_speech",
                        AttributeValue::S(vocab_req.part_of_speech.clone()),
                    )
                    .item("example", AttributeValue::S(vocab_req.example.clone()))
                    .item("created_at", AttributeValue::S(created_at.clone()));

                if let Some(src_id) = &vocab_req.source_text_id {
                    put_req = put_req.item("source_text_id", AttributeValue::S(src_id.clone()));
                }

                let res = put_req.send().await;

                match res {
                    Ok(_) => {
                        let new_vocab = Vocabulary {
                            id,
                            word: vocab_req.word,
                            meaning: vocab_req.meaning,
                            part_of_speech: vocab_req.part_of_speech,
                            example: vocab_req.example,
                            source_text_id: vocab_req.source_text_id,
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
            let id = path_params.first("id");

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

    let table_name = "eng-app-table".to_string();

    run(service_fn(move |event| {
        let client = client.clone();
        let table_name = table_name.clone();
        async move { function_handler(event, client, &table_name).await }
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
        let response = function_handler(request, client, "dummy_table")
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
            "meaning": "怠惰な",
            "part_of_speech": "adjective",
            "example": "He is a lazy dog.",
            "source_text_id": "text-uuid-1234"
        }"#;

        let request = HttpRequest::builder()
            .method(Method::POST)
            .uri("/vocabulary")
            .header("content-type", "application/json")
            .body(Body::Text(payload.to_string()))
            .expect("failed to build request");

        let client = create_dummy_client();
        let response = function_handler(request, client, "dummy_table")
            .await
            .expect("handler failed");

        assert_eq!(
            response.status(),
            500,
            "実際のDBがないため500エラーになるはずです"
        );
    }
}
