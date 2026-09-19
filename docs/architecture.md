# 全体アーキテクチャ設計書

## 1. システム構成図
```mermaid
graph TD
    Client["Browser / SPA<br/>(Vite + React)"]
    API_Gateway["Amazon API Gateway<br/>(HTTP API)"]
    Lambda["AWS Lambda<br/>(Rust)"]
    Tavily["Tavily AI Search API<br/>(最新ニュース検索)"]
    Gemini["Google Gemini API<br/>(gemini-3.5-flash-lite)"]
    DynamoDB["Amazon DynamoDB<br/>(トピック・単語帳)"]
    SSM["AWS SSM Parameter Store<br/>(APIキー管理)"]

    Client -- "HTTPS / REST<br/>(Header: x-api-key)" --> API_Gateway
    API_Gateway -- "invoke<br/>(API Key検証)" --> Lambda
    Lambda -. "Web検索ON時: ニュース取得" .-> Tavily
    Lambda -- "記事生成・構文解析" --> Gemini
    Lambda -- "単語帳・トピック保存" --> DynamoDB
    Lambda -- "起動時にAPIキー一括読込" --> SSM

    classDef aws fill:#FF9900,stroke:#232F3E,stroke-width:2px,color:black;
    classDef external fill:#4285F4,stroke:#0F9D58,stroke-width:2px,color:white;
    classDef client fill:#61DAFB,stroke:#282C34,stroke-width:2px,color:black;

    class API_Gateway,Lambda,DynamoDB,SSM aws;
    class Gemini,Tavily external;
    class Client client;
```

## 2. コンポーネント役割

| コンポーネント | 役割 |
|----------------|------|
| フロントエンド (React) | ユーザーUI。トピック選択、Web検索ON/OFF、生成記事の読解・構文解析表示・単語保存を行う。 |
| API Gateway | エンドポイントの提供、CORS制御、スロットリングによるレートリミット。 |
| Lambda (Rust) | メインロジック。起動時(`main`)にSSMからAPIキーをキャッシュし、Tavily/Gemini呼び出しやDB操作を超高速に処理。 |
| DynamoDB | シングルテーブル設計を用いたデータの永続化（トピック・単語帳）。 |
| Tavily AI Search API | 最新ニュース検索エンジン。トピックに関連する直近3日間のニュース記事とURLを取得。 |
| Gemini API | LLMエンジン (`gemini-3.5-flash-lite`)。英文記事生成および構文・単語解析を担当。 |
| SSM Parameter Store | `x-api-key`、`GEMINI_API_KEY`、`TAVILY_API_KEY` の安全な保持・管理。 |

## 3. ディレクトリ・モジュール構成方針

```
ai-english-learner/          # プロジェクトルート
├── .mise/tasks/             # タスクランナー定義
│   ├── dev/                 # ローカル開発用 (back, front)
│   ├── deploy/              # 本番デプロイ用 (infra, front, all)
│   ├── test/                # 自動テスト用 (back, front, e2e, all)
│   └── setup                # 初期環境構築スクリプト (Rust, Cargo Lambda, Playwright, Node, Terraform)
├── backend/                 # サーバー側リポジトリ (Rust)
│   ├── infra/               # Terraformコード (AWSリソース定義)
│   └── src/                 # Rustコード (lib.rs: 共通純粋関数, bin/: 各Lambda関数)
├── docs/                    # ドキュメントディレクトリ
│   ├── architecture.md      # 本ドキュメント
│   ├── design.md            # 基本設計書
│   ├── requirements.md      # 要件定義
│   ├── CONTRIBUTING.md      # 開発手順書
│   └── learning/            # 学習・設計判断の記録 (ADR等)
├── frontend/                # クライアント側リポジトリ (Vite + React 19)
│   ├── e2e/                 # 実ブラウザE2Eテスト (Playwright: mocks.ts, *.spec.ts)
│   ├── playwright.config.ts # Playwright 設定 (Vite 開発サーバー自動連動)
│   └── src/                 # Reactソースコード (components/, test/)
└── README.md                # プロジェクト概要
```

---

## 4. テストアーキテクチャ（3層モック分離）

本システムは、CI環境や手元開発で外部APIトークンを一切消費せず、安全・高速に実行可能な3層の自動テストアーキテクチャ（計69テスト）を採用しています。

```mermaid
graph TD
    Mise["mise run test:all<br/>(一括テストランナー)"]

    subgraph BackendTest ["1. バックエンド テスト (cargo-llvm-cov)"]
        Handlers["Lambda Handlers<br/>(generate_text, topics, vocab)"]
        PureFuncs["Pure Functions<br/>(lib.rs: プロンプト/JSON)"]
        WireMock["wiremock::MockServer<br/>(ローカルHTTPモック)"]
        
        Handlers --> PureFuncs
        Handlers <-->|注入されたモックURL| WireMock
    end

    subgraph FrontendTest ["2. フロントエンド単体テスト (Vitest + RTL)"]
        Components["UI Components<br/>(Login, TextGen, etc.)"]
        MockFetch["vi.spyOn(fetch)<br/>インメモリAPIモック"]
        Components <--> MockFetch
    end

    subgraph E2ETest ["3. フロントエンド E2E テスト (Playwright)"]
        Browser["Chromium Headless / Headed<br/>(実ブラウザ操作)"]
        RouteIntercept["page.route(RegExp)<br/>ネットワーク完全遮断モック"]
        Browser <-->|外部通信・トークン消費ゼロ| RouteIntercept
    end

    Mise --> BackendTest
    Mise --> FrontendTest
    Mise --> E2ETest
```

- **バックエンド (23件)**: `wiremock` によりローカルで HTTP モックサーバーを立ち上げ、`ApiEndpoints` を注入することで Tavily や Gemini との通信を完全シミュレート。`cargo-llvm-cov` で行カバレッジとHTMLレポートを出力。
- **フロントエンド単体・コンポーネント (40件)**: `Vitest` 上でブラウザの `fetch` を直接モック化。API Gateway やバックエンドを起動せずにコンポーネントの状態遷移を数秒で検証。
- **フロントエンド E2E (6件)**: `Playwright` により実ブラウザでログインから記事生成、スラッシュリーディング表示、単語帳登録までの実操作シナリオを検証。`page.route` によるネットワーク完全遮断でトークン消費ゼロを保証。

