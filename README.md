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
├── frontend/      # Vite + React (UI画面)
├── backend/       # Rust Lambda + Terraform (APIロジックとAWSインフラ)
└── docs/          # 要件定義、アーキテクチャ図、API仕様、学習メモなどのドキュメント
```

## 🛠 技術スタック

| 領域 | 技術 |
|---|---|
| **フロントエンド** | React, TypeScript, Vite, Vanilla CSS |
| **バックエンド** | Rust, cargo-lambda, tokio |
| **AWS インフラ** | Terraform, Lambda, API Gateway, DynamoDB, S3, CloudFront, SSM |
| **AI (LLM)** | Google Gemini API (`gemini-3.5-flash-lite`) |
| **Web検索** | Tavily AI Search API |

## 📚 開発ドキュメント

開発に関する知見やAWSインフラの設計意図などは、`docs/learning/` フォルダ配下の勉強メモに詳細にまとめられている。
