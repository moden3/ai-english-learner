# 2. Rustによるサーバーレスバックエンド

## Cargo Lambdaによる開発
AWS LambdaをRustで開発するためのデファクトツール。スクリプト言語とは異なり、コンパイルによる高速な起動(コールドスタート対策)と安全性が得られる。

### 実装サンプル (Lambdaハンドラの雰囲気)
Rustでは `lambda_http` クレートを使用し、非同期関数としてハンドラを定義する。

```rust
use lambda_http::{run, service_fn, Body, Error, Request, Response};

async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    // リクエストの処理
    let message = "Hello from Rust Lambda!";
    
    // HTTPレスポンスの構築
    let resp = Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(message.into())
        .map_err(Box::new)?;
    Ok(resp)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Lambdaランタイムの起動
    run(service_fn(function_handler)).await
}
```

## RustバイナリのZIPデプロイ方式
RustをAWS Lambdaで動かす場合、「カスタムランタイム (provided.al2023)」を利用する。

- **デプロイの流れ**:
  1. `cargo lambda build --release` でAmazon Linux用バイナリを生成。
  2. 生成されたバイナリを `bootstrap` という名前にリネームし、`function.zip` に圧縮。
  3. TerraformでこのZIPファイルをデプロイする。

### 実装サンプル (Terraform側)
```hcl
resource "aws_lambda_function" "api_lambda" {
  function_name = "my-rust-lambda"
  # 圧縮したZIPファイルを指定
  filename      = "function.zip"
  # カスタムランタイムの場合は handler の値は任意だが、通常は bootstrap などを指定
  handler       = "bootstrap"
  # OSのみを提供するカスタムランタイムを指定
  runtime       = "provided.al2023"
  role          = aws_iam_role.lambda_role.arn
}
```

## DynamoDB シングルテーブル設計
NoSQLのベストプラクティス。RDBのようにテーブルを分割せず、1つのテーブル(`eng-app-table`)で複数種類のデータを扱う。

- **PK (Partition Key)**: `hash_key` とも呼ばれる。データが物理的に保存されるサーバー（パーティション）を決定するキー。検索時は完全一致で指定する必要がある。本アプリではデータの「種類」や大分類を表す (例: `TOPIC`, `VOCAB`)。
- **SK (Sort Key)**: `range_key` とも呼ばれる。同じPKを持つデータを並び替えるためのキー。前方一致検索(`begins_with`)や範囲指定に利用でき、柔軟な検索を可能にする。本アプリではデータの一意な識別子として利用 (例: `TOPIC#<uuid>`)。

### 実装サンプル (Terraform)
```hcl
resource "aws_dynamodb_table" "main" {
  name         = "eng-app-table"
  billing_mode = "PAY_PER_REQUEST" # 使った分だけ課金（無料枠に最適）
  
  hash_key  = "PK"
  range_key = "SK"

  attribute {
    name = "PK"
    type = "S" # String
  }
  attribute {
    name = "SK"
    type = "S" # String
  }
}
```

### UUIDを利用した重複許可と一意性の担保
同じユーザーが「Technology」というトピックを2つ作ってしまった場合でも、システムエラーを起こさない工夫。
- `SK` を単なる名前にすると、重複した瞬間に古いデータが上書き（意図せぬ破壊）されてしまう。
- `SK = TOPIC#<UUID>` のようにランダムなIDを組み合わせることで、**システム上の完全な一意性を担保しつつ、表示名(name属性)としては重複を許容する** 柔軟な設計が可能になる。
