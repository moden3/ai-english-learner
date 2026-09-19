# 開発者向けセットアップ・デプロイ手順書 (Developer Guide)

このドキュメントは、本プロジェクトの開発環境構築、およびAWSへのデプロイ手順をまとめたものである。
以下の手順に従って環境をセットアップしてください。

## 1. 前提ツールのインストール

開発に必要な各種ランタイムやツール（Node.js, Terraform, Rust, Cargo Lambda 等）は、`mise` および本プロジェクトのセットアップタスクによってすべて自動管理・導入される。
したがって、**事前に開発者が手動でインストールする必要があるのは `mise` のみ** である。

### 1.1 mise (タスク・環境管理ツール)
以下のコマンドでインストールすること。
```bash
curl https://mise.run | sh
# Macの場合は brew install mise も可能
```
> インストール後、`mise` コマンドが認識されない場合は、シェルの設定ファイル（`.bashrc` や `.zshrc` など）にパスを追加するか、ターミナルを再起動してください。
> 参考: `echo 'eval "$(~/.local/bin/mise activate bash)"' >> ~/.bashrc`

※ **Node.js**, **Terraform**, **Rust**, **Cargo Lambda**, **Playwright** 等はすべて次節の `mise run setup` で全自動でセットアップされるため、手動インストールや個別の PATH 設定は一切不要である。

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

以下のコマンドで、本プロジェクトの開発に必要な言語・ツール・ライブラリ・インフラの初期化を一括実行する。

```bash
mise run setup
```

> **`mise run setup` で自動実行される処理:**
> 1. 🦀 **Rust ツールチェーンのセットアップ** (未インストール時は `rustup` および stable ツールチェーンを自動導入)
> 2. ⚡ **Cargo Lambda のセットアップ** (Lambda 関数のローカル実行・ビルドツールを自動導入)
> 3. 🧪 **バックエンドのテスト・カバレッジツール導入** (`cargo-llvm-cov`, `llvm-tools-preview`)
> 4. 📦 **フロントエンド依存パッケージの導入** (`npm install`)
> 5. 🎭 **Playwright ヘッドレスブラウザ・Linux 共有ライブラリのセットアップ**
> 6. ☁️ **Terraform の初期化** (`terraform init`)
>
> ※ `mise.toml` の環境変数設定により、`~/.local/bin` や `~/.cargo/bin` への PATH も mise タスク内で自動的に解決されるため、環境変数 PATH の手動設定も不要です。

---

## 4. ローカル開発環境の起動とテスト

### 4.1 バックエンド（Lambda）の起動
```bash
mise run dev:back
```
> `http://127.0.0.1:9000/lambda-url` でローカルAPIがリッスン状態になる。

### 4.2 フロントエンド（Vite）の起動
```bash
mise run dev:front
```
> `http://localhost:5173` で開発サーバーが起動する。

**環境変数ファイルの設定 (`.env`)**
ローカルでバックエンドとフロントエンドを連携させる場合、各ディレクトリに `.env` を作成する。

**`frontend/.env`** (ローカル開発用設定)
```env
VITE_API_URL=http://localhost:9000/lambda-url
```

**`backend/.env`** (ローカル開発用設定)
```env
APP_API_KEY=your-local-api-key

# ローカルで実際のAI / Web検索をテストする場合
# (設定するとAWS SSMにアクセスせず直接APIを利用できます)
#GEMINI_API_KEY=AIzaSy-xxxxxxxxxxxxxxxx
#TAVILY_API_KEY=tvly-xxxxxxxxxxxxxxxx
```
> **📝 メモ: ダミーモードが作動する条件**
> 無駄なAPIトークンの消費を防ぐため、以下のいずれかに該当する場合は Gemini / Tavily API と通信せず、固定のダミーテキストを返却する。
> 1. フロントエンドで選択・入力したトピック名が `test` または `dummy` で始まる場合
> 2. Gemini APIキーが未設定（空文字、または初期値 `CHANGE_ME_GEMINI_KEY`）の場合

ブラウザで `http://localhost:5173` にアクセスし、正常に動作するか確認すること。
（※確認が終わったら `frontend/.env` の `VITE_API_URL` を元のAWSエンドポイントに戻してください）

### 4.3 自動テストとコードカバレッジの実行 (Unit, Mock & E2E Tests)
本プロジェクトでは、外部ネットワーク通信を行わず安全かつ高速にバックエンド（Rust）、フロントエンド単体・コンポーネント（Vitest）、および実ブラウザによるE2E（Playwright）の全テストを一括実行できる。

バックエンド側では **ターミナルへのカバレッジサマリー出力** と **HTMLカバレッジレポートの自動生成** が行われる。
E2Eテストでは、Playwright のネットワークインターセプトにより外部API（Gemini / Tavily）のトークン消費ゼロ・バックエンド未起動でも実ブラウザ操作を完全自動検証する。

```bash
# バックエンド ＋ フロントエンド（単体・コンポーネント ＋ E2E）の全テストを一括実行
mise run test:all

# 個別実行したい場合
mise run test:back    # バックエンド（Rust）カバレッジテストのみ
mise run test:front   # フロントエンド（Vitest）単体・コンポーネントテストのみ
mise run test:e2e     # フロントエンド（Playwright）E2Eテストのみ
```

> **バックエンド HTMLカバレッジレポートのブラウザ閲覧**
> 生成されたレポート（行ごとの網羅状況・カラーハイライト）は以下で直接開くことができる：
> ```bash
> xdg-open backend/target/llvm-cov/html/index.html
> ```

> **フロントエンドのウォッチモード起動**
> フロントエンド開発中に単体テストを自動再実行したい場合：
> ```bash
> cd frontend && npm run test:watch
> ```

#### 🎭 Playwright E2E テストの画面表示・デバッグ実行
Playwright では、画面を表示しながらのテストやステップ実行が簡単に行える。

1. **インタラクティブ UI モード（推奨 🌟）**
   GUI ダッシュボードが立ち上がり、タイムトラベル（各ステップのDOM・画面遷移・通信の再現）やテストの個別再生が可能：
   ```bash
   mise run test:e2e --ui
   ```

2. **Headed モード（実際のブラウザ画面を眺める）**
   実際の Chromium ウィンドウが開き、自動入力・クリックの様子がリアルタイムに描画される：
   ```bash
   mise run test:e2e --headed

   # 4並列ではなく1つずつゆっくり確認したい場合（1ワーカー）
   mise run test:e2e --headed --workers=1

   # 特定のテストシナリオだけを画面表示したい場合
   mise run test:e2e e2e/generate_and_analysis.spec.ts --headed
   ```

3. **デバッグモード（1行ずつステップ実行）**
   Playwright Inspector が起動し、「Step Over」で1行ずつ進めながらセレクター調査や挙動確認ができる：
   ```bash
   mise run test:e2e --debug
   # または特定のテストファイルのみ
   mise run test:e2e e2e/auth.spec.ts --debug
   ```

> ※ WSL2 環境では、GUI サポート（WSLg）により自動的に Windows 側にブラウザやダッシュボード画面が表示されます。終了時はウィンドウを閉じるか、ターミナルで `Ctrl + C` を押してください。

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

### 5.4 AWS環境の設定 (APIキー・シークレット)
デプロイ後、AWSコンソールで Systems Manager (パラメーターストア) を開き、各パラメータの値を本番用の値に変更して保存する。
1. **`/eng-app/api-key`**: フロントエンドから呼び出す際の共通パスワードとなる文字列（例: `my-super-secret-key-123`）を設定。
2. **`/eng-app/gemini-api-key`**: Google AI Studio で取得した Gemini APIキー を設定。
3. **`/eng-app/tavily-api-key`**: [Tavily](https://app.tavily.com/) で無料取得した Tavily APIキー（Web検索機能用）を設定。

以上でデプロイは完了。
出力された `cloudfront_url` にアクセスし、「ENG-APP」のログイン画面が表示されれば成功となる。
