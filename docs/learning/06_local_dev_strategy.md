# 6. サーバーレス＆AIアプリのローカル開発手法 (Local Dev Strategy)

クラウドインフラや外部のAI APIに依存するアプリケーションを開発する際、「テストのたびにクラウドへデプロイが必要になる」「テストのたびにAI APIの課金枠（制限枠）を消費してしまう」という2つの大きな課題が発生する。

本プロジェクトでは、この課題を解決し、**「クラウド課金ゼロ」「API制限ゼロ」でフロントエンドの高速な反復開発（イテレーション）を実現する仕組み** を取り入れている。

## 1. `cargo lambda watch` によるローカルサーバーレス環境
AWS Lambda (Rust) のコードをテストするたびにAWSへZIPデプロイするのは非効率である。
本プロジェクトでは `cargo lambda watch` を利用し、ローカルマシン上に擬似的なLambda環境を立ち上げることで、API Gateway + Lambda の挙動を完全にローカルでシミュレートしている。

```bash
# .envファイルを読み込ませつつ、ローカルでポート9000番にLambda環境を立ち上げる
$ cargo lambda watch --env-file .env
```
これにより、フロントエンドからは `http://localhost:9000/lambda-url/generate_text` へリクエストを送るだけで、本番と同じようにバックエンドの処理をテストできる。

## 2. `.env` を活用した AI ダミーモード（モック）の仕組み
Gemini API のような外部APIを組み込む際、UIの微調整やアニメーションのテストのたびに本物のAIを呼び出していると、あっという間に「1日の利用上限（レートリミット）」に引っかかってしまう。

これを防ぐため、バックエンド側に「ダミーモード（モック）」を実装している。

### 実装のポイント
バックエンド (`generate_text.rs`) では、以下のいずれかの条件を満たす場合に、Gemini APIへの通信をバイパスし、即座に「固定のJSONデータ」を返すように分岐処理を入れている。

1. **環境変数による制御**: バックエンド側の `.env` ファイルに `USE_MOCK_AI=true` が設定されている場合。
2. **マジックワードによる制御**: ユーザーが入力したトピック名が `test` や `dummy` で始まる場合。
3. **APIキーのフォールバック**: AWS SSMからAPIキーが取得できず、初期値（`DUMMY_KEY_FOR_TESTING`）のままの場合。

```rust
// generate_text.rs のダミー判定ロジック
let is_dummy_mode = std::env::var("USE_MOCK_AI").is_ok() 
    || topic_name.to_lowercase().starts_with("test")
    || topic_name.to_lowercase().starts_with("dummy")
    || api_key == "DUMMY_KEY_FOR_TESTING";

if is_dummy_mode {
    // 外部APIを叩かずに即座にダミーのJSONを返す
    let dummy_res = if action == "analyze" {
        json!({
            "segments": [...],
            "keywords": [...]
        })
    } else { ... };
    
    return Ok(Response::builder().body(...));
}
```

### 開発体験 (DX) への寄与
この仕組みにより、フロントエンドエンジニアは「APIの制限」や「AIの応答待ち時間」を気にすることなく、UIのスタイル調整や状態遷移のテストを**ローカルで瞬時**に行うことができるようになる。
サーバーレス＆AIアプリをストレスなく個人開発するための非常に重要なアプローチである。
