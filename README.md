# AI English Learner (ENG-APP) 🚀

AI（Google Gemini 3.5 Flash-lite）および検索特化エンジン（Tavily AI Search）を活用した、サーバーレスな次世代英語学習アプリケーションである。
最新ニュース記事の自動収集、プロフェッショナルなビジネス英文生成（150〜250語）、構文解説、重要単語の抽出などを一瞬で行い、学習を強力にサポートする。

## 🌟 特徴

- **フルサーバーレス & 月額$0運用**: AWS Lambda, API Gateway, DynamoDB, S3, CloudFrontによる完全サーバーレス構成。個人利用において無料枠内で収まるコスト最適化設計。
- **高速なバックエンド**: バックエンド(Lambda)には **Rust** を採用し、コールドスタート時にシークレットを一括キャッシュして遅延を極限まで排除。
- **最新ニュース連携 (Tavily AI Search)**: Web検索とLLM生成の責務を分離し、直近3日間のファクトに基づいた英語記事と正確な出典URLを提供。
- **モダンなUI/UX**: Vite + React と Vanilla CSS による **Glassmorphism（グラスモーフィズム）** デザイン。
- **セキュアな設計**: ソースコードにAPIキーを含めないランタイム認証と、SSM Parameter Storeによるシークレット管理。

## 📁 ディレクトリ構造 (Monorepo)

```text
.
├── .github/workflows/ # GitHub Actions CI & 本番自動デプロイ定義 (ci.yml, deploy.yml)
├── .githooks/         # Git pre-commit 自動品質チェックフック
├── .mise/tasks/       # 開発・ビルド・テスト・フォーマット・CI・デプロイの一括タスク定義
├── frontend/          # Vite + React (UI画面, Vitest単体テスト, Playwright E2Eテスト)
├── backend/           # Rust Lambda + Terraform (APIロジックとAWSインフラ定義)
└── docs/              # 要件定義、アーキテクチャ図、API仕様、学習ナレッジ(docs/learning/)
```

## 🛠 技術スタック

| 領域                | 技術                                                                         |
| ------------------- | ---------------------------------------------------------------------------- |
| **フロントエンド**  | React 19, TypeScript, Vite, Vanilla CSS, Vitest (`v8` coverage), Playwright  |
| **バックエンド**    | Rust (Edition 2024), cargo-lambda, tokio, wiremock, cargo-llvm-cov           |
| **AWS インフラ**    | Terraform (S3 Remote Backend), Lambda, API Gateway, DynamoDB, S3, CloudFront |
| **AI / Web検索**    | Google Gemini API (`gemini-3.5-flash-lite`), Tavily AI Search API            |
| **開発基盤・CI/CD** | mise (タスク・環境管理), Prettier, ESLint, Clippy, Git Hooks, GitHub Actions |

## 🚀 クイックスタート (主要コマンド)

開発環境の構築からフォーマット、テスト、CI検証、本番デプロイまで、すべて `mise` タスクに集約されている（詳細は [docs/CONTRIBUTING.md](./docs/CONTRIBUTING.md) を参照）。

```bash
mise run setup        # 初期環境構築（Rust, Cargo Lambda, Playwright, npm, Terraform, Git Hooks）
mise run fmt          # コード・インフラ・Markdownドキュメントの一括フォーマット整形
mise run pre-commit   # コミット前の品質・セキュリティチェック（git commit時に自動実行）
mise run test:all     # 全3層（Rust + Vitest + Playwright E2E 計90件）の自動テスト＆カバレッジ出力
mise run build:all    # バックエンド＆フロントエンドの本番並列ビルド
mise run ci           # ローカルCIパイプライン（pre-commit + test:all + build:all）
mise run deploy:all   # AWS本番環境（Terraform + S3/CloudFront）への一括デプロイ
```

## 📚 開発ドキュメント・学習ナレッジ

設計書および開発に関する実践的な知見・ノウハウは `docs/` 配下に体系的にまとめられている。

- **設計・運用ドキュメント**:
  - [システム要件定義書 (docs/requirements.md)](./docs/requirements.md)
  - [全体アーキテクチャ設計書 (docs/architecture.md)](./docs/architecture.md)
  - [システム基本設計書 (docs/design.md)](./docs/design.md)
  - [Web API エンドポイント仕様一覧 (docs/api/endpoints.md)](./docs/api/endpoints.md)
  - [開発者向けセットアップ・デプロイ手順書 (docs/CONTRIBUTING.md)](./docs/CONTRIBUTING.md)
- **Web開発ナレッジ集 (`docs/learning/`)**:
  - [01. アーキテクチャ・デシジョン・レコード (ADR)](./docs/learning/01_architecture_decisions.md)
  - [02. TerraformとAWSインフラ基盤](./docs/learning/02_terraform_infrastructure.md)
  - [03. Rustによるサーバーレスバックエンドとモック戦略](./docs/learning/03_rust_backend.md)
  - [04. モダンフロントエンド開発・セキュリティ・テスト戦略](./docs/learning/04_frontend_react.md)
  - [05. AI (Gemini API) & Web検索 (Tavily) 連携とコスト最適化](./docs/learning/05_ai_integration.md)
  - [06. ローカル開発戦略・Miseタスク集約・CI/CD自動デプロイ基盤](./docs/learning/06_local_dev_strategy.md)
