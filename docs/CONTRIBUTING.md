# 開発者向けセットアップ・デプロイ手順書 (Developer Guide)

このドキュメントは、本プロジェクトの開発環境構築、およびAWSへのデプロイ手順をまとめたものです。
以下の手順に従って環境をセットアップしてください。

## 1. 前提ツールのインストール

開発にあたり、以下のツールが必要です。

### 1.1 Terraform
AWSリソースをコード（IaC）で管理するためのツールです。
```bash
sudo snap install terraform --classic
terraform --version
```

### 1.2 Cargo Lambda
RustでAWS Lambda関数をビルド・ローカルテストするためのツールです。
（事前に `cargo` がインストールされている必要があります）
```bash
pip3 install cargo-lambda
# またはMacの場合は `brew tap cargo-lambda/cargo-lambda && brew install cargo-lambda`
```

> **PATHの設定について（Linux環境等でのエラー対策）**
> `cargo lambda --version` 実行時に `error: no such command: lambda` が出る場合は、`~/.local/bin` にPATHを通してください。
> ```bash
> export PATH="$HOME/.local/bin:$PATH"
> ```

### 1.3 Node.js & npm
フロントエンド開発（Vite + React）に使用します。適宜インストールしてください。

---

## 2. AWS認証情報の設定

TerraformがAWSにリソースを作成できるよう、環境変数に設定します。
（※実際の開発では `~/.aws/credentials` を用いるのが一般的です）

```bash
export AWS_ACCESS_KEY_ID="あなたのアクセスキー"
export AWS_SECRET_ACCESS_KEY="あなたのシークレットキー"
export AWS_DEFAULT_REGION="ap-northeast-1"
```

---

## 3. プロジェクトのセットアップ (ローカル環境)

フロントエンドの依存関係をインストールします。

```bash
cd frontend
npm install
```

---

## 4. ローカル環境での動作確認

AWSへデプロイする前に、ローカルエミュレーターを用いて結合テストを行うことができます。

### 4.1 バックエンドのローカル起動
`backend` ディレクトリで以下のコマンドを実行し、ローカルAPIサーバー (`http://localhost:9000`) を立ち上げます。
```bash
cd backend
cargo lambda watch --env-file .env
```

### 4.2 環境変数 (.env) の設定
ローカルで動作させる場合、フロントエンドとバックエンドそれぞれのディレクトリに `.env` ファイルを作成（または修正）し、以下のように設定してください。

**`frontend/.env`** (バックエンドの接続先をローカルへ向ける)
```env
VITE_API_URL=http://localhost:9000/lambda-url
```

**`backend/.env`** (AIをダミーモードにしてAPI制限や課金を防ぐ)
```env
USE_MOCK_AI=true
```
> **📝 メモ: ダミーモードが作動する条件**
> 無駄なAPIトークンの消費を防ぐため、以下のいずれかに該当する場合は Gemini API と通信せず、固定のダミーテキストを返却します。
> 1. 上記のように `USE_MOCK_AI=true` 環境変数が設定されている場合
> 2. フロントエンドで入力したトピック名が `test` または `dummy` で始まる場合
> 3. AWS Systems Manager (SSM) からAPIキーが取得できない場合

### 4.3 フロントエンドのローカル起動
別のターミナルを開き、フロントエンドを立ち上げます。
```bash
cd frontend
npm run dev
```
ブラウザで `http://localhost:5173` にアクセスし、正常に動作するか確認してください。
（※確認が終わったら `.env` の `VITE_API_URL` を元のAWSエンドポイントに戻してください）

---

## 5. 本番環境 (AWS) へのデプロイ

動作確認が完了したら、実際にAWS環境へデプロイします。以下の手順でバックエンド・インフラ・フロントエンドをデプロイします。

### 5.1 バックエンド（Lambda）のビルド
```bash
cd backend
cargo lambda build --release
```

### 5.2 インフラストラクチャの適用 (Terraform)
ビルドしたLambdaと各種AWSリソース（S3, API Gateway, DynamoDBなど）を作成・更新します。
```bash
cd backend/infra
terraform init  # 初回のみ
terraform apply
```
適用が完了すると、コンソールに以下の値が出力されます。
- `api_gateway_url`: APIのエンドポイント（`frontend/.env` で使用）
- `frontend_bucket_name`: フロントエンド用S3バケット名
- `cloudfront_url`: 公開用URL

### 5.3 フロントエンドのビルドとS3アップロード
```bash
cd frontend
# 本番デプロイ時は .env の VITE_API_URL が api_gateway_url になっているか確認
npm run build
aws s3 sync dist/ s3://<確認したバケット名> --delete
```

### 5.4 AWS環境の設定 (APIキー)
デプロイ後、AWSコンソールで Systems Manager (パラメーターストア) を開き、設定を行います。
1. **`/eng-app/gemini-api-key`**: ご自身の Gemini APIキー を設定して保存します。
2. **`/eng-app/api-key`**: フロントエンドから呼び出す際の共通パスワードとなる文字列（例: `my-super-secret-key-123`）を設定して保存します。

以上でデプロイは完了です！
出力された `cloudfront_url` にアクセスし、「ENG-APP」のログイン画面が表示されれば成功です。
