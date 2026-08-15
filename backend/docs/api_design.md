# バックエンド API & DB 設計書

## 1. DynamoDB 設計 (シングルテーブル設計)

**テーブル名**: `eng-app-table`  
**パーティションキー**: `PK` (String)  
**ソートキー**: `SK` (String)

| エンティティ | PK | SK | 属性 (データ型) |
|------------|----|----|-----------------|
| トピック | `TOPIC` | `TOPIC#<uuid>` | `name` (S), `name_en` (S), `created_at` (S) |
| 単語帳 | `VOCAB` | `WORD#<uuid>` | `word` (S), `meaning` (S), `grammar_note` (S), `created_at` (S) |

## 2. Lambda関数とエンドポイントのマッピング

| メソッド | エンドポイント | 担当Lambda関数 (Rust bin) | 用途 |
|----------|----------------|-------------------------|------|
| POST | `/texts/generate` | `GenerateTextFunction` | AIによるテキスト・解析データの一括生成 |
| GET | `/topics` | `TopicsFunction` | トピック一覧取得 |
| POST | `/topics` | `TopicsFunction` | トピック追加 |
| DELETE | `/topics/{id}` | `TopicsFunction` | トピック削除 |
| GET | `/vocabulary`| `VocabularyFunction` | 単語帳一覧取得 |
| POST | `/vocabulary`| `VocabularyFunction` | 単語帳追加 |
| DELETE | `/vocabulary/{id}`| `VocabularyFunction` | 単語帳削除 |

## 3. 認証要件
全てのリクエストにおいて、ヘッダーに `x-api-key: <Secret Key>` を含めること。
Lambdaの初期処理にてSSMパラメータから取得したキーと照合し、不一致時は即座に `401 Unauthorized` を返却。

## 4. API 仕様詳細

### 4.1 POST `/texts/generate`
*リクエスト:*
```json
{
  "level": "intermediate",
  "topic": "business"
}
```
*レスポンス:*
```json
{
  "text_id": "uuid-xxxx",
  "level": "intermediate",
  "topic": "business",
  "title": "Effective Remote Communication",
  "full_text": "The quick brown fox...",
  "segments": [
    {
      "id": 1,
      "text": "The quick brown fox",
      "translation": "素早い茶色のキツネが",
      "grammar_note": "主語(S)。形容詞quickとbrownが名詞foxを修飾。"
    }
  ],
  "keywords": [
    {
      "word": "lazy",
      "meaning": "怠惰な、のんびりした",
      "part_of_speech": "adjective",
      "example": "He is a lazy dog."
    }
  ],
  "created_at": "2026-08-12T12:00:00Z"
}
```

### 4.2 トピック管理 API
* GET `/topics` -> トピックのJSON配列を返す
* POST `/topics` -> `{ "name": "...", "name_en": "..." }` を受け取り保存

### 4.3 単語帳 API
* GET `/vocabulary` -> 単語帳のJSON配列を返す
* POST `/vocabulary` -> 以下のJSONを受け取り保存
```json
{
  "word": "lazy",
  "meaning": "怠惰な",
  "part_of_speech": "adjective",
  "example": "He is a lazy dog.",
  "source_text_id": "uuid-xxxx"
}
```
