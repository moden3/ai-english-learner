# 3. Rustによるサーバーレスバックエンド

## Cargo Lambdaによる開発
AWS LambdaをRustで開発するためのデファクトツール。スクリプト言語とは異なり、コンパイルによる高速な起動(コールドスタート対策)と安全性が得られる。

### 実装サンプル (コールドスタート最適化とシークレットキャッシュ)
Lambdaの初回起動時（`main()`）にAWS SSMからAPIキーを取得し、スレッドセーフな参照カウントポインタ `Arc` でハンドラに渡すことで、リクエストごとのSSM呼び出しオーバーヘッド（数百ms）を完全にゼロにします。また、HTTPクライアント（`reqwest::Client`）も `Arc` でプールを使い回します。

```rust
use aws_sdk_ssm::Client as SsmClient;
use eng_app_backend::validate_api_key;
use lambda_http::{run, service_fn, Body, Error, Request, Response};
use reqwest::Client as HttpClient;
use std::sync::Arc;

#[derive(Clone)]
pub struct ApiEndpoints {
    pub tavily_url: String,
    pub gemini_base_url: String,
}

impl Default for ApiEndpoints {
    fn default() -> Self {
        Self {
            tavily_url: "https://api.tavily.com/search".into(),
            gemini_base_url: "https://generativelanguage.googleapis.com".into(),
        }
    }
}

async fn function_handler(
    event: Request,
    http_client: Arc<HttpClient>,
    app_api_key: &str,
    gemini_api_key: Arc<String>,
    tavily_api_key: Arc<String>,
    endpoints: ApiEndpoints,
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
        let endpoints = ApiEndpoints::default();
        async move {
            function_handler(event, http_client, &app_api_key, gemini_api_key, tavily_api_key, endpoints).await
        }
    }))
    .await
}
```

---

## バックエンドのテスト戦略とモック技術 (Testability & Mocking)

サーバーレス（Lambda）や外部AI API（Tavily, Gemini）に依存したバックエンド開発では、「テスト実行のたびに課金やトークンを消費してしまう」「外部ネットワークの不調でテストが不安定になる」「異常系（500/503エラー）を意図的に再現しにくい」という課題が生じます。
これらを解決し、CI環境やローカルで高速・安全に検証するための一般的なテスト設計プラクティスを以下にまとめます。

### 1. どこをモック化し、どこを純粋関数とするか（責務の分離）

テスト容易性（Testability）を高める基本原則は、**「I/O（ネットワーク・DB）を伴う処理」と「純粋な計算ロジック（ビジネスロジック）」を分離すること**です。

| 対象レイヤー | 処理内容の例 | テスト手法 | モックの要否 |
| :--- | :--- | :--- | :---: |
| **純粋関数 (Core Logic)** | プロンプト組み立て、ニュース文字数の切り詰め、Markdown除去・JSON抽出、APIキー文字列検証 | `#[test]` による通常の単体テスト | **不要** (入力と出力のみで完結) |
| **I/O・外部連携 (Integration)** | Tavily REST API、Gemini REST API、AWS DynamoDB | `#[tokio::test]` + HTTPモック (`wiremock`) | **必要** (外部通信を遮断) |

#### 純粋関数（Pure Functions）の切り出し (`src/lib.rs`)
Lambdaハンドラの中にプロンプト生成や正規表現パースを混在させず、副作用を持たない関数として独立させます。
```rust
// プロンプト生成: 入力（トピック名・ニュース）から出力（文字列）への純粋関数
pub fn build_generate_prompt(topic: &str, news_context: Option<&str>) -> String;

// JSON抽出: LLMの生出力からMarkdownや前後テキストを除去して安全にパースする純粋関数
pub fn extract_json_payload<T: DeserializeOwned>(raw: &str) -> Result<T, Error>;
```
これにより、HTTP通信やモックライブラリを一切セットアップすることなく、ミリ秒単位で境界値テスト（空文字、長文、不正JSONなど）を実行可能になります。

---

### 2. 外部APIのモック化手法：エンドポイント注入 ＋ `wiremock`

本プロジェクトでは、外部通信（Tavily, Gemini）を安全にテストするため、**エンドポイントURLの依存性注入 (Dependency Injection)** とローカルHTTPモックサーバー **`wiremock`** を採用しています。

#### 設計の仕組み
HTTPクライアントの接続先URLを `ApiEndpoints` 構造体として切り出し、ハンドラへ注入します。
```rust
#[derive(Clone)]
pub struct ApiEndpoints {
    pub tavily_url: String,
    pub gemini_base_url: String,
}

impl Default for ApiEndpoints {
    fn default() -> Self {
        Self {
            tavily_url: "https://api.tavily.com/search".into(),
            gemini_base_url: "https://generativelanguage.googleapis.com".into(),
        }
    }
}
```
- **本番環境**: デフォルト値（`ApiEndpoints::default()`）を使用して公式の外部APIへ接続。
- **テスト環境**: ローカルで動的に空きポートを確保した `wiremock::MockServer` のURLを注入。

#### なぜこの手法を採用したのか
1. **通信層全体を一気通貫で検証できる**:
   Traitによるインターフェースモックとは異なり、`reqwest` による実際のリクエスト組み立て（HTTPメソッド、Path、ヘッダー、JSONシリアライズ）から、ステータスコード判定、デシリアライズまで、**通信処理全体を本番と全く同じコードパスで検証**できます。
2. **過剰な抽象化を避け、コードをシンプルに保てる**:
   外部APIごとにTrait定義やジェネリクスを導入する複雑な設計を排し、「URLを差し替えるだけ」という最小限の変更でテスタビリティを確保できます。

---

### 3. `wiremock` による HTTP モックの実装例

ローカルにインプロセスのHTTPサーバーを立ち上げ、期待されるリクエスト条件と返却レスポンスを宣言的に定義します。

```rust
#[tokio::test]
async fn test_generate_with_web_search_success() {
    // 1. ローカルモックサーバーをランダムな空きポートで起動
    let mock_server = MockServer::start().await;

    // 2. Tavily API のモック（POST /search に 200 JSON を返す）
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "results": [...] })))
        .mount(&mock_server)
        .await;

    // 3. Gemini API のモック（POST /generateContent に 200 JSON を返す）
    Mock::given(method("POST"))
        .and(path("/v1beta/models/gemini-3.5-flash-lite:generateContent"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ ... })))
        .mount(&mock_server)
        .await;

    // 4. ハンドラにモックサーバーのURLを注入して実行
    let endpoints = ApiEndpoints {
        tavily_url: format!("{}/search", mock_server.uri()),
        gemini_base_url: mock_server.uri(),
    };

    let response = function_handler(request, http_client, "test-key", gemini_key, tavily_key, endpoints).await.unwrap();
    assert_eq!(response.status(), 200);
}
```

---

### 4. 外部AI（LLM）連携テスト特有の検証ポイント

LLMを組み込んだバックエンドでは、従来のWeb APIテストにはない特有のテスト観点が存在します。

1. **出力の不確実性（Markdown装飾・前後ノイズ）の耐性**:
   LLMはプロンプトの指示にもかかわらず、JSONの前後に解説文を付与したり、` ```json ` のコードブロックで囲んで返却することがあります。モックの `set_body_json` や文字列レスポンスで意図的に装飾混じりのJSONを返却し、パーサーが正しく抽出できるかをテストします。
2. **外部障害時のフェイルファスト（500/503エラー）**:
   外部の検索APIやLLMが過負荷でエラーを返した際、長時間ハングアップしたり不完全なデータを返したりせず、即座に適切なHTTPステータス（例: Tavily障害時は `503 Service Unavailable`）でクライアントに返せるかをモックで確実に再現・検証します。

---

### 5. コードカバレッジ計測 (`cargo-llvm-cov`) による品質可視化

テストがソースコードのどの分岐をカバーできているかを客観的に把握するため、LLVMソースベースカバレッジツールを活用します。
- **行カバレッジ (Line Coverage)**: 実行された行の割合。
- **分岐・領域カバレッジ (Region Coverage)**: `if / match` 等の分岐条件網羅率。
テスト実行時にターミナルサマリーと同時にHTMLレポート（`target/llvm-cov/html/index.html`）を生成し、未テストのエラーハンドリング分岐（例: 異常系ステータス時の処理）を視覚的に特定・補強します。


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
