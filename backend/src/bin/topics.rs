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
) -> Result<Response<Body>, Error> {
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
                            ) = (
                                item.get("SK"),
                                item.get("name"),
                                item.get("created_at"),
                            ) {
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
                if let Some(last_seg) = path.split('/').last() {
                    if !last_seg.is_empty() && last_seg != "topics" {
                        id = Some(last_seg.to_string());
                    }
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

    // テーブル名は Terraform で作成した「eng-app-table」です
    let table_name = "eng-app-table".to_string();

    // 毎回のリクエスト処理
    run(service_fn(move |event| {
        // スレッド間で安全に共有するためにクローンしてハンドラに渡す
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

    // テスト用のダミーDynamoDBクライアントを作成する関数
    fn create_dummy_client() -> Arc<Client> {
        let config = Config::builder().behavior_version_latest().build();
        Arc::new(Client::from_conf(config))
    }

    #[tokio::test]
    async fn test_get_topics_fails_without_real_db() {
        // DynamoDBの実際の接続がないため、ダミークライアントを渡すと
        // TimeoutやCredentialsエラーなどで 500 Internal Server Error になることを確認します。
        // （本番レベルではモック(Mock)を利用しますが、今回は簡易的なテストにとどめます）
        let request = HttpRequest::builder()
            .method(Method::GET)
            .uri("/topics")
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
}
