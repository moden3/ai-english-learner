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

| 機能ID | メソッド | パス | 用途 | リクエストボディ | レスポンス例 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| F-004 | `GET` | `/topics` | 保存済みのトピック一覧を取得する。 | なし | `[{"id": "...", "name": "Technology", ...}]` |
| F-006 | `POST` | `/topics` | 新しいトピックを作成（DBに保存）する。 | `{"name": "トピック名"}` | 200 OK |
| F-007 | `DELETE` | `/topics` | 指定したトピックを削除する。 | `{"id": "トピックID"}` | 200 OK |

---

## 2. 単語帳API (`/vocabulary`)
実装ファイル: `backend/src/bin/vocabulary.rs`

| 機能ID | メソッド | パス | 用途 | リクエストボディ | レスポンス例 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| F-009 | `GET` | `/vocabulary` | 保存済みの英単語一覧を取得する。 | なし | `[{"id": "...", "word": "apple", ...}]` |
| F-008 | `POST` | `/vocabulary` | 新しい英単語を単語帳に登録する。 | `{"word": "単語", "meaning": "意味"}` | 200 OK |
| F-010 | `DELETE` | `/vocabulary` | 単語帳から指定した単語を削除する。 | `{"id": "単語ID"}` | 200 OK |

---

## 3. AIテキスト生成・解析API (`/generate_text`)
実装ファイル: `backend/src/bin/generate_text.rs`

| 機能ID | メソッド | パス | 用途 |
| :--- | :--- | :--- | :--- |
| F-001<br>F-002<br>F-003 | `POST` | `/generate_text` | Gemini 3.5 Flash-liteおよびTavily AI Searchを用いて、英文記事の生成（最新ニュース検索対応）または構文解析・重要単語抽出を行う。 |

### 3.1 英文記事生成 (`action: "generate"`)
トピックに応じた150〜250語のビジネス英語記事を生成します。

**リクエストボディ**:
```json
{
  "action": "generate",
  "topic_name": "Artificial Intelligence in Finance",
  "use_web_search": true
}
```
- `topic_name` (string, 任意): 生成したい記事のトピック。未指定の場合はデフォルトトピック。
- `use_web_search` (boolean, 任意, デフォルト: `false`): `true` にすると、Tavily AI Search API で直近3日間のニュースを検索し、その内容を踏まえた記事と参照URLを返却します。

**レスポンス例 (200 OK)**:
```json
{
  "text": "Recent advancements in artificial intelligence are reshaping financial services...",
  "source_url": "https://www.example.com/news/ai-finance"
}
```
※ `use_web_search: false` の場合、`source_url` は `null` になります。

### 3.2 構文解析・単語抽出 (`action: "analyze"`)
生成された英文をスラッシュリーディング用の意味の塊（セグメント）に分割し、和訳・文法注記・上級ビジネス英単語（CEFR C1レベル）を抽出します。

**リクエストボディ**:
```json
{
  "action": "analyze",
  "text": "Recent advancements in artificial intelligence are reshaping financial services..."
}
```

**レスポンス例 (200 OK)**:
```json
{
  "segments": [
    { "id": 1, "text": "Recent advancements", "translation": "最近の進展は", "grammar_note": "主語(S)" },
    { "id": 2, "text": "in artificial intelligence", "translation": "人工知能における", "grammar_note": "前置詞句(修飾語)" },
    { "id": 3, "text": "are reshaping financial services.", "translation": "金融サービスを再構築している。", "grammar_note": "動詞(V) + 目的語(O)" }
  ],
  "keywords": [
    {
      "word": "reshaping",
      "meaning": "再構築している、形を変えている",
      "part_of_speech": "verb",
      "example": "AI is reshaping the financial industry."
    }
  ]
}
```

### 3.3 主なHTTPエラーステータス
| ステータス | 原因 | メッセージ / 挙動 |
| :--- | :--- | :--- |
| `400 Bad Request` | リクエストボディが不正 | `"Invalid Request Body"` |
| `401 Unauthorized` | `x-api-key` ヘッダーが未指定または不一致 | `"Unauthorized"` |
| `405 Method Not Allowed` | POST 以外のHTTPメソッド | `"Method Not Allowed"` |
| `503 Service Unavailable` | `use_web_search: true` 時にTavily検索が失敗（エラーまたは結果0件） | `"Web search failed. Please try again or disable web search."` |
| `500 Internal Server Error` | Gemini APIエラーまたは通信エラー | `"Gemini API returned an error"` |
