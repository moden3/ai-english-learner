# 4. AI (Gemini API) 連携の組み込み方

本プロジェクトでは、Googleの `Gemini API (gemini-1.5-flash)` を用いて英語学習コンテンツ（英文生成・和訳・構文解説など）の生成を行っている。

## Gemini APIの利用準備
1. **APIキーの取得**: Google AI StudioからGemini APIのキーを発行し、AWS SSM Parameter Storeに保存する。
2. **エンドポイント**: REST APIとして以下のURLに対してHTTP POSTリクエストを送信する。
   `https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent?key=<API_KEY>`

## バックエンド(Rust)からの組み込み方法
AWS Lambda (Rust) からGemini APIを呼び出す具体的な実装手順。

- **HTTPクライアント**: `reqwest` クレートを使用して非同期HTTPリクエストを行う。
- **リクエストの構築**:
  - `system_instruction`: AIに「どのような役割として振る舞うか」「必ず指定したJSONスキーマで返すこと」などのシステム指示を設定。
  - `contents`: ユーザーの選択したレベルやトピックに基づくプロンプトを設定。
  - `response_mime_type`: 強制的にJSONで返答させるため `application/json` を指定。

- **実装イメージ**:
  ```rust
  // reqwestクライアントの生成
  let client = reqwest::Client::new();
  
  // Gemini APIへのリクエストボディ作成
  let request_body = json!({
      "system_instruction": {
          "parts": [{ "text": "あなたはプロの英語教師です。必ず指定されたJSONスキーマで返答してください。" }]
      },
      "contents": [{
          "parts": [{ "text": format!("トピック: {} について英語の長文を作ってください。", topic) }]
      }],
      "generationConfig": {
          "response_mime_type": "application/json"
      }
  });

  // APIリクエストの送信
  let res = client.post(api_url)
      .json(&request_body)
      .send()
      .await?;
      
  // レスポンスのJSONをパースして構造体にマッピング
  let response_data: GeminiResponse = res.json().await?;
  ```

- **データの流れ**: 
  API Gateway経由でフロントエンドから「トピック名」を受け取り、Rust内でGemini APIを呼び出す。得られた結果をDynamoDBに保存し、そのままフロントエンドにJSONとして返却して画面に表示させる。
