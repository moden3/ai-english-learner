# 開発者向けセットアップ・デプロイ手順書 (Developer Guide)

このドキュメントは、本プロジェクトの開発環境構築、およびAWSへのデプロイ手順をまとめたものです。
ご自身のローカル環境で以下の手順を実行し、クラウド開発の基礎を学びながら進めてください。

## 1. 開発ツールのインストール

Terraform およびバックエンド開発用のRustツールチェーンが必要です。

### 1.1 Terraformのインストール
AWSリソースをコード（IaC）で管理するためのツールです。
Ubuntuの場合は以下のコマンドでインストールできます。
```bash
sudo snap install terraform --classic
```
インストール後、バージョンが確認できれば成功です。
```bash
terraform --version
```

### 1.2 Cargo Lambdaのインストール
RustでAWS Lambda関数をビルド・ローカルテストするためのツールです。
（事前に `cargo` がインストールされている必要があります）
```bash
pip3 install cargo-lambda
```
※またはMacの場合は `brew tap cargo-lambda/cargo-lambda && brew install cargo-lambda`

> **PATHの設定について（Linux環境等でのエラー対策）**
> `pip3 install cargo-lambda` を実行すると、Pythonのユーザーディレクトリ（例: `~/.local/bin`）に `cargo-lambda` というラッパーがインストールされます。
> もし `cargo lambda --version` を実行して `error: no such command: lambda` というエラーが出た場合、このディレクトリにPATHが通っていない可能性があります。
>
> 以下の方法のいずれかで解決できます。
> 
> **方法A: PATHを追加する (推奨)**
> ```bash
> export PATH="$HOME/.local/bin:$PATH"
> ```
> これを `~/.bashrc` や `~/.zshrc` に追記しておくと、次回ログイン時からも有効になります。
>
> **方法B: ラッパーを直接実行する**
> （PATHを通さずに直接呼び出す場合）
> ```bash
> ~/.local/bin/cargo-lambda lambda --version
> ```
> 今後ビルド等を行う際も、`cargo lambda build ...` の代わりに `~/.local/bin/cargo-lambda lambda build ...` と入力します。

インストール確認:
```bash
cargo lambda --version
```

---

## 2. AWS認証情報の設定

TerraformがAWSにリソースを作成できるよう、AWSの認証情報を環境変数に設定します。

```bash
export AWS_ACCESS_KEY_ID="あなたのアクセスキー"
export AWS_SECRET_ACCESS_KEY="あなたのシークレットキー"
export AWS_DEFAULT_REGION="ap-northeast-1"
```

> **注意**: 実際の開発では `~/.aws/credentials` にプロファイルを設定し、`AWS_PROFILE` を指定する方法が一般的で安全です。

---

## 3. Terraformによる基盤デプロイ (デプロイテスト)

`eng-app-backend/infra/` ディレクトリ配下に、本アプリに必要なインフラ構成が記述されています。

### 手順

1. **インフラディレクトリへ移動**
   ```bash
   cd eng-app-backend/infra
   ```

2. **Terraformの初期化**
   必要なプロバイダー（AWSプラグイン等）をダウンロードします。
   ```bash
   terraform init
   ```

3. **実行計画の確認 (Plan)**
   実際に作成・変更されるAWSリソースの一覧を確認します。この時点ではまだAWS上のリソースは変更されません。
   ```bash
   terraform plan
   ```
   出力結果を読み、意図したリソース（API Gateway, IAM Role, DynamoDB, SSM等）が作成されることを確認してください。

4. **デプロイの実行 (Apply)**
   AWS上にリソースを作成します。
   ```bash
   terraform apply
   ```
   途中で `Enter a value:` と聞かれるので `yes` と入力してEnterを押します。

5. **API Gateway URLの確認**
   デプロイが完了すると、ターミナルに `api_gateway_url` が出力されます。これがAPIのエンドポイントになります。（今後のフロントエンド開発で使用します）

---

## 4. バックエンド実装 (Rust) のセットアップ

ここからはRustを用いたLambda関数の実装に入ります。今回は学習のため、TDD（テスト駆動開発）のサイクルを回しながら進めます。
まずはプロジェクトの初期化を行いましょう。

### 手順

1. **バックエンドのディレクトリへ移動**
   ```bash
   cd /home/moden3_ubuntu/aws-work/eng-app-backend
   ```

2. **Cargoプロジェクトの初期化**
   すでに `eng-app-backend` フォルダは存在しますが、この中にRustプロジェクトとしての設定（`Cargo.toml`など）を作成します。
   ```bash
   cargo init --bin
   ```
   ※実行後、`Cargo.toml` と `src/main.rs` が自動生成されます。

3. **依存関係（ライブラリ）の追加**
   今回はサーバーレスAPI向けに `lambda_http` や、非同期処理の `tokio`、DynamoDB操作用の `aws-sdk-dynamodb` などを利用します。
   以下のコマンドを実行して必要なライブラリを追加してください。
   ```bash
   cargo add lambda_http tokio --features tokio/full
   cargo add serde serde_json --features serde/derive
   cargo add aws-config aws-sdk-dynamodb aws-sdk-ssm
   cargo add reqwest --features reqwest/json
   ```

### 今後の開発（TDD）の進め方
今後の実装は、以下のサイクルで進めていきます。
1. **AI（私）**が「わざと失敗するテストコード」と「空の関数」を書きます。
2. **あなた**が `cargo test` を実行し、**テストが失敗する (FAILED) ことを確認**します。
3. **AI（私）**がテストを通すための「実際の実装コード」を書きます。
4. **あなた**が再度 `cargo test` を実行し、**テストが成功する (ok) ことを確認**し、書かれたコードをレビューします。

## 5. バックエンドのビルドとデプロイ

バックエンドの実装が完了したら、実際にAWSへデプロイして動かします。

1. **Rust（Lambda関数）のビルド**
   `eng-app-backend` フォルダ内で以下のコマンドを実行し、リリース用の最適化された実行ファイルを作成します。
   ```bash
   cd /home/moden3_ubuntu/aws-work/eng-app-backend
   cargo lambda build --release
   ```

2. **Terraformでデプロイ**
   ビルドされたファイルをAWSにアップロードし、インフラに反映させます。
   ```bash
   cd /home/moden3_ubuntu/aws-work/eng-app-backend/infra
   terraform apply
   ```

3. **Gemini APIキーの設定**
   デプロイ後、AWSコンソールを開いて以下の手順でAPIキーを設定します。
   - AWSコンソールの検索窓で「Systems Manager」を検索して開く。
   - 左側のメニューから「パラメーターストア (Parameter Store)」を選択。
   - 一覧から `/eng-app/gemini-api-key` をクリック。
   - 右上の「編集」を押し、「値 (Value)」の欄にご自身のGemini APIキーを入力して保存。

4. **フロントエンド用APIキーの設定**
   同じくパラメーターストアにある `/eng-app/api-key` も編集し、お好きな文字列（例: `my-super-secret-key-123`）に変更して保存します。（これはフロントエンドからAPIを呼び出す際のパスワードになります）

---

## 6. フロントエンド (Vite + React) のセットアップ

ここからはユーザーが直接触れるWeb画面の開発環境を構築します。
「モダンでリッチなデザイン」を実現するため、Tailwindは使わずにVanilla CSSを用いて進めます。

### 手順

1. **プロジェクトの初期化**
   Viteを使ってReact(TypeScript)の雛形を作成します。ルートディレクトリ(`aws-work/`)で以下のコマンドを実行してください。
   ```bash
   cd /home/moden3_ubuntu/aws-work
   npm create vite@latest eng-app-frontend -- --template react-ts
   ```

2. **依存関係のインストール**
   作成されたフォルダに入り、必要なライブラリをインストールします。今回はルーティングに `react-router-dom` などを利用する予定です。
   ```bash
   cd eng-app-frontend
   npm install
   npm install react-router-dom
   ```

3. **環境変数 (.env) の作成**
   `eng-app-frontend` ディレクトリの直下に `.env` ファイルを作成し、APIのURLとアクセスキーを設定します。
   ```bash
   touch .env
   ```
   `.env` ファイルを開き、以下の内容を記述してください。
   ```
   # /infra でデプロイした際に出力された api_gateway_url を設定
   VITE_API_URL=https://0h1c2w4a4e.execute-api.ap-northeast-1.amazonaws.com
   ```
   ※APIキーはセキュリティの観点からソースコードに埋め込まず、ブラウザ上のログイン画面で入力する設計となっています。

4. **開発サーバーの起動確認**
   設定が終わったら、ローカルサーバーを立ち上げてブラウザで確認してみましょう。
   ```bash
   npm run dev
   ```
   ターミナルに表示される `http://localhost:5173/` のURLにアクセスし、ViteとReactの初期画面が表示されれば成功です！

---

## 7. フロントエンドのビルドとデプロイ (AWS)

フロントエンドの実装が完了し、本番環境 (S3 + CloudFront) にデプロイする手順です。

### 手順

1. **Viteによるビルドの実行**
   ```bash
   cd /home/moden3_ubuntu/aws-work/eng-app-frontend
   npm run build
   ```
   ※成功すると `dist` フォルダ内に公開用の静的ファイルが生成されます。

2. **Terraformでインフラを適用し、S3バケット名を確認**
   まだS3やCloudFrontを作成していない場合は、先にTerraformを適用します。
   ```bash
   cd /home/moden3_ubuntu/aws-work/eng-app-backend/infra
   terraform apply
   ```
   適用が完了すると、コンソールに `frontend_bucket_name` （例：`eng-app-frontend-hosting-bucket-12345`）と `cloudfront_url` が出力されます。

3. **S3へファイルをアップロード (同期)**
   AWS CLIを使用して、ビルドしたファイルをS3バケットにアップロードします。
   ```bash
   aws s3 sync ../../eng-app-frontend/dist/ s3://<確認したバケット名> --delete
   ```

4. **動作確認**
   Terraformの出力結果にある `cloudfront_url` （例：`https://dxxxxxxx.cloudfront.net`）にブラウザからアクセスします。
   「ENG-APP」のログイン画面が表示され、指定したAPIキーでログインできればデプロイ成功です！
