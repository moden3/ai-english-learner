# AI English Learner (ENG-APP) 🚀

AI（Google Gemini 1.5 Flash）を活用した、サーバーレスな次世代英語学習アプリケーションです。
長文読解、構文解説、重要単語の抽出などを一瞬で生成し、学習を強力にサポートします。

## 🌟 特徴

- **フルサーバーレス & 月額$0運用**: AWS Lambda, API Gateway, DynamoDB, S3, CloudFrontによる完全サーバーレス構成。個人利用において無料枠内で収まるコスト最適化設計。
- **高速なバックエンド**: バックエンド(Lambda)には **Rust** を採用し、コールドスタートの遅延を極限まで排除。
- **モダンなUI/UX**: Vite + React と Vanilla CSS による **Glassmorphism（グラスモーフィズム）** デザイン。
- **セキュアな設計**: ソースコードにAPIキーを含めないランタイム認証と、SSM Parameter Storeによるシークレット管理。

## 📁 ディレクトリ構造 (Monorepo)

```text
.
├── frontend/      # Vite + React (UI画面)
├── backend/       # Rust Lambda + Terraform (APIロジックとAWSインフラ)
└── docs/          # アーキテクチャ図や学習メモなどのドキュメント
```

## 🛠 技術スタック

| 領域 | 技術 |
|---|---|
| **フロントエンド** | React, TypeScript, Vite, Vanilla CSS |
| **バックエンド** | Rust, cargo-lambda, tokio |
| **AWS インフラ** | Terraform, Lambda, API Gateway, DynamoDB, S3, CloudFront, SSM |
| **AI (LLM)** | Google Gemini API (gemini-1.5-flash) |

## 📚 開発ドキュメント

開発に関する知見やAWSインフラの設計意図などは、`docs/learning/` フォルダ配下の勉強メモに詳細にまとめられています。

- [00_index.md](./docs/learning/00_index.md) - システム概要とアーキテクチャ図
- [01_terraform_infrastructure.md](./docs/learning/01_terraform_infrastructure.md) - AWSインフラとIaC
- [02_rust_backend.md](./docs/learning/02_rust_backend.md) - Rust実装とDynamoDB設計
- [03_frontend_react.md](./docs/learning/03_frontend_react.md) - UIデザインとセキュリティ
- [04_ai_integration.md](./docs/learning/04_ai_integration.md) - AI連携の組み込み
- [05_cost_optimization.md](./docs/learning/05_cost_optimization.md) - 月額無料化戦略
