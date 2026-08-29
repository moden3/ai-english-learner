# 全体アーキテクチャ設計書

## 1. システム構成図
```mermaid
graph TD
    Client["Browser / SPA<br/>(Vite + React)"]
    API_Gateway["Amazon API Gateway<br/>(HTTP API)"]
    Lambda["AWS Lambda<br/>(Rust)"]
    Gemini["Google Gemini API<br/>(Flash)"]
    DynamoDB["Amazon DynamoDB"]
    SSM["AWS SSM Parameter Store"]

    Client -- "HTTPS / REST<br/>(Header: x-api-key)" --> API_Gateway
    API_Gateway -- "invoke<br/>(API Key検証)" --> Lambda
    Lambda -- "1リクエストで一括生成" --> Gemini
    Lambda -- "単語帳・トピック保存" --> DynamoDB
    Lambda -- "APIキー読込" --> SSM

    classDef aws fill:#FF9900,stroke:#232F3E,stroke-width:2px,color:black;
    classDef external fill:#4285F4,stroke:#0F9D58,stroke-width:2px,color:white;
    classDef client fill:#61DAFB,stroke:#282C34,stroke-width:2px,color:black;

    class API_Gateway,Lambda,DynamoDB,SSM aws;
    class Gemini external;
    class Client client;
```

## 2. コンポーネント役割

| コンポーネント | 役割 |
|----------------|------|
| フロントエンド (React) | ユーザーUI。APIへリクエストし、取得したJSONデータからインタラクティブな読解画面を構成する。 |
| API Gateway | エンドポイントの提供、CORS制御、スロットリングによるレートリミット。 |
| Lambda (Rust) | メインロジック。ルーティングはAPI Gatewayに任せ、機能ごとに1リソース1Lambdaで構成。超高速起動。 |
| DynamoDB | シングルテーブル設計を用いたデータの永続化。 |
| Gemini API | 英文・和訳・構文解説の自動生成AIエンジン。 |
| SSM Parameter Store | `x-api-key` （フロントとバックで共有する簡易認証キー）や `GEMINI_API_KEY` の保持。 |

## 3. ディレクトリ・モジュール構成方針

```
ai-english-learner/      # プロジェクトルート
├── backend/             # サーバー側リポジトリ
│   ├── infra/           # Terraformコード
│   └── src/             # Rustコード (binごとに各Lambda関数を定義)
├── docs/                # ドキュメントディレクトリ
│   ├── architecture.md  # 本ドキュメント
│   ├── requirements.md  # 要件定義
│   └── CONTRIBUTING.md  # 開発手順書
├── frontend/            # クライアント側リポジトリ
│   └── src/             # Vite + React のソースコード
└── README.md            # プロジェクト概要
```
