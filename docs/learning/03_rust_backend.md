# 3. Rustによるサーバーレスバックエンド

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
