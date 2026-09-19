# 3. Rustによるサーバーレスバックエンド

## Cargo Lambdaによる開発
AWS LambdaをRustで開発するためのデファクトツール。スクリプト言語とは異なり、コンパイルによる高速な起動(コールドスタート対策)と安全性が得られる。

### 実装サンプル (コールドスタート最適化とシークレットキャッシュ)
Lambdaの初回起動時（`main()`）にAWS SSMからAPIキーを取得し、スレッドセーフな参照カウントポインタ `Arc` でハンドラに渡すことで、リクエストごとのSSM呼び出しオーバーヘッド（数百ms）を完全にゼロにします。また、HTTPクライアント（`reqwest::Client`）も `Arc` でプールを使い回します。

```rust
use aws_sdk_ssm::Client as SsmClient;
use lambda_http::{run, service_fn, Body, Error, Request, Response};
use reqwest::Client as HttpClient;
use std::sync::Arc;

async fn function_handler(
    event: Request,
    http_client: Arc<HttpClient>,
    app_api_key: &str,
    gemini_api_key: Arc<String>,
    tavily_api_key: Arc<String>,
) -> Result<Response<Body>, Error> {
    // 認証チェック (x-api-key ヘッダーの検証)
    if !validate_api_key(&event, app_api_key) {
        return Ok(Response::builder().status(401).body("Unauthorized".into())?);
    }
    
    // ビジネスロジック（Tavily検索やGemini API呼び出し）
    // ...
    Ok(Response::builder().status(200).body("OK".into())?)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let ssm_client = SsmClient::new(&config);
    let http_client = Arc::new(HttpClient::new());

    // 起動時（コールドスタート時）に全APIキーを一度だけSSMから取得してArc化
    let app_api_key = Arc::new(eng_app_backend::get_api_key(&ssm_client).await);
    let gemini_api_key = Arc::new(eng_app_backend::get_gemini_api_key(&ssm_client).await);
    let tavily_api_key = Arc::new(eng_app_backend::get_tavily_api_key(&ssm_client).await);

    run(service_fn(move |event| {
        let http_client = http_client.clone();
        let app_api_key = app_api_key.clone();
        let gemini_api_key = gemini_api_key.clone();
        let tavily_api_key = tavily_api_key.clone();
        async move {
            function_handler(event, http_client, &app_api_key, gemini_api_key, tavily_api_key).await
        }
    }))
    .await
}
```

## RustバイナリのZIPデプロイ方式
RustをAWS Lambdaで動かす場合、「カスタムランタイム (provided.al2023)」を利用する。

- **デプロイの流れ**:
  1. `cargo lambda build --release` でAmazon Linux用バイナリを生成。
  2. 生成されたバイナリを `bootstrap` という名前にリネームし、`function.zip` に圧縮。
  3. TerraformでこのZIPファイルをデプロイする。（※Terraform側の設定例については `01_terraform_infrastructure.md` を参照）

## DynamoDB シングルテーブル設計
NoSQLのベストプラクティス。RDBのようにテーブルを分割せず、1つのテーブル（`eng-app-table`）で複数種類のデータを扱う。

### キー構造と役割
| キー種別 | 名称 (Terraform) | 役割 | 本アプリでの利用例 | 検索仕様 |
| :--- | :--- | :--- | :--- | :--- |
| **PK** | `hash_key` | データが保存されるサーバーの決定、データの大分類 | `TOPIC`, `VOCAB` | **完全一致**のみ |
| **SK** | `range_key` | データの並び替え、一意な識別子 | `TOPIC#<uuid>` | 前方一致(`begins_with`)・範囲指定可能 |

（※DynamoDBテーブル構築のTerraformコード例については `01_terraform_infrastructure.md` を参照）

### UUIDによる一意性担保の工夫
同名のデータ（例: ユーザーが「Technology」というトピックを2つ作成）が存在する場合の、意図せぬデータ上書きを防ぐ設計。

- ❌ **SKをデータ名にした場合**: 重複した瞬間に古いデータが上書き（破壊）される。
- ⭕ **SKにUUIDを組み合わせた場合**: `SK = TOPIC#<UUID>` となり、システム上の完全な一意性を担保。表示名（name属性）での重複を安全に許容できる。
