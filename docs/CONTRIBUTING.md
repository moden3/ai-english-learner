# 開発者向けセットアップ・デプロイ手順書 (Developer Guide)

このドキュメントは、本プロジェクトの開発環境構築、およびAWSへのデプロイ手順をまとめたものである。
以下の手順に従って環境をセットアップしてください。

## 1. 前提ツールのインストール

開発にあたり、以下のツールが必要である。

### 1.1 mise (タスク・環境管理ツール)
本プロジェクトでは、開発コマンドの実行および各種ツール（Node.js, Terraform）のバージョン管理に `mise` を使用する。
以下のコマンドでインストールすること。
```bash
curl https://mise.run | sh
# Macの場合は brew install mise も可能
```
> インストール後、`mise` コマンドが認識されない場合は、シェルの設定ファイル（`.bashrc` や `.zshrc` など）にパスを追加するか、ターミナルを再起動してください。
> 参考: `echo 'eval "$(~/.local/bin/mise activate bash)"' >> ~/.bashrc`

### 1.2 Cargo Lambda
RustでAWS Lambda関数をビルド・ローカルテストするためのツールである。
（事前に `cargo` がインストールされている必要がある）
```bash
pip3 install cargo-lambda
# またはMacの場合は brew tap cargo-lambda/cargo-lambda && brew install cargo-lambda
```

> **PATHの設定について（Linux環境等でのエラー対策）**
> `cargo lambda --version` 実行時に `error: no such command: lambda` が出る場合は、`~/.local/bin` にPATHを通してください。
> ```bash
> export PATH="$HOME/.local/bin:$PATH"
> ```

※ **Node.js** と **Terraform** については、`mise` を通じて自動で適切なバージョンがインストールされるため、手動でのインストールは不要である。

---

## 2. AWS認証情報の設定

TerraformがAWSにリソースを作成できるよう、環境変数に設定する。
（※実際の開発では `~/.aws/credentials` を用いるのが一般的である）

```bash
export AWS_ACCESS_KEY_ID="あなたのアクセスキー"
export AWS_SECRET_ACCESS_KEY="あなたのシークレットキー"
export AWS_DEFAULT_REGION="ap-northeast-1"
```

---

## 3. プロジェクトのセットアップ (ローカル環境)

依存関係のインストールやTerraformの初期化を行う。

```bash
mise run setup
```

---

## 4. ローカル環境での動作確認

AWSへデプロイする前に、ローカルエミュレーターを用いて結合テストを行うことができる。

### 4.1 ローカルサーバーの起動
以下のコマンドで、バックエンド（API）とフロントエンドの開発サーバーを同時に起動できる。

```bash
mise run dev:all
```
※バックエンド単体を起動したい場合は `mise run dev:back`、フロント単体の場合は `mise run dev:front` を使用する。

### 4.2 環境変数 (.env) の設定
ローカルで動作させる場合、フロントエンドとバックエンドそれぞれのディレクトリに `.env` ファイルを作成（または修正）し、以下のように設定すること。

**`frontend/.env`** (バックエンドの接続先をローカルへ向ける)
```env
VITE_API_URL=http://localhost:9000/lambda-url
```

**`backend/.env`** (AIをダミーモードにしてAPI制限や課金を防ぐ)
```env
USE_MOCK_AI=true
```
> **📝 メモ: ダミーモードが作動する条件**
> 無駄なAPIトークンの消費を防ぐため、以下のいずれかに該当する場合は Gemini API と通信せず、固定のダミーテキストを返却する。
> 1. 上記のように `USE_MOCK_AI=true` 環境変数が設定されている場合
> 2. フロントエンドで入力したトピック名が `test` または `dummy` で始まる場合
> 3. AWS Systems Manager (SSM) からAPIキーが取得できない場合

ブラウザで `http://localhost:5173` にアクセスし、正常に動作するか確認すること。
（※確認が終わったら `frontend/.env` の `VITE_API_URL` を元のAWSエンドポイントに戻してください）

---

## 5. 本番環境 (AWS) へのデプロイ

動作確認が完了したら、実際にAWS環境へデプロイする。
本プロジェクトでは、インフラの適用（Terraform）とフロントエンドのビルド＆デプロイを以下のコマンド一発で全自動実行できる。

```bash
# ※デプロイ前に frontend/.env の VITE_API_URL が api_gateway_url になっているか確認
mise run deploy:all
```

> **各デプロイを個別に行いたい場合**
> - バックエンド・インフラのみ: `mise run deploy:infra`
> - フロントエンドのみ: `mise run deploy:front`

### 5.4 AWS環境の設定 (APIキー)
デプロイ後、AWSコンソールで Systems Manager (パラメーターストア) を開き、設定を行う。
1. **`/eng-app/gemini-api-key`**: ご自身の Gemini APIキー を設定して保存する。
2. **`/eng-app/api-key`**: フロントエンドから呼び出す際の共通パスワードとなる文字列（例: `my-super-secret-key-123`）を設定して保存する。

以上でデプロイは完了である！
出力された `cloudfront_url` にアクセスし、「ENG-APP」のログイン画面が表示されれば成功である。
