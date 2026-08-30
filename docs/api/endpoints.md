# Web API エンドポイント仕様一覧

本プロジェクト（AI英語学習アプリ）のバックエンド（Rust + AWS API Gateway）に実装されているHTTP APIの一覧です。

## 共通仕様
- **認証**: すべてのエンドポイントは、リクエストヘッダーに `x-api-key` を付与する必要があります。
- **ベースURL**: 環境によって異なります。
  - ローカル開発環境: `http://localhost:9000/lambda-url`
  - 本番環境 (AWS): API GatewayのエンドポイントURL

---

## 1. トピック管理API (`/topics`)
実装ファイル: `backend/src/bin/topics.rs`

| メソッド | パス | 用途 | リクエストボディ | レスポンス例 |
| :--- | :--- | :--- | :--- | :--- |
| `GET` | `/topics` | 保存済みのトピック一覧を取得する。 | なし | `[{"id": "...", "name": "Technology", ...}]` |
| `POST` | `/topics` | 新しいトピックを作成（DBに保存）する。 | `{"name": "トピック名"}` | 200 OK |
| `DELETE` | `/topics` | 指定したトピックを削除する。 | `{"id": "トピックID"}` | 200 OK |

---

## 2. 単語帳API (`/vocabulary`)
実装ファイル: `backend/src/bin/vocabulary.rs`

| メソッド | パス | 用途 | リクエストボディ | レスポンス例 |
| :--- | :--- | :--- | :--- | :--- |
| `GET` | `/vocabulary` | 保存済みの英単語一覧を取得する。 | なし | `[{"id": "...", "word": "apple", ...}]` |
| `POST` | `/vocabulary` | 新しい英単語を単語帳に登録する。 | `{"word": "単語", "meaning": "意味"}` | 200 OK |
| `DELETE` | `/vocabulary` | 単語帳から指定した単語を削除する。 | `{"id": "単語ID"}` | 200 OK |

---

## 3. AIテキスト生成・解析API (`/generate_text`)
実装ファイル: `backend/src/bin/generate_text.rs`

| メソッド | パス | 用途 | リクエストボディ |
| :--- | :--- | :--- | :--- |
| `POST` | `/generate_text` | Gemini APIを用いて、トピックに応じたテキストの生成、または構文解析・単語抽出を行う。 | `{"topic": "...", "action": "generate" 又は "analyze"}` |

※ `action: "generate"` の場合は長文の英語記事が返却され、`action: "analyze"` の場合はスラッシュリーディング用のセグメントと重要単語のリストが返却されます。
