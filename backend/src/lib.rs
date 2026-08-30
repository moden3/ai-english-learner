use aws_sdk_ssm::Client as SsmClient;
use lambda_http::Request;

pub async fn get_api_key(ssm_client: &SsmClient) -> String {
    // 1. 環境変数 (ローカル開発用)
    if let Ok(val) = std::env::var("APP_API_KEY") {
        return val;
    }

    // 2. AWS SSM Parameter Store
    let ssm_res = ssm_client
        .get_parameter()
        .name("/eng-app/api-key")
        .with_decryption(true)
        .send()
        .await;

    match ssm_res {
        Ok(out) => out.parameter.unwrap().value.unwrap(),
        Err(e) => {
            println!("SSM error (could not read API Key): {:?}", e);
            "CHANGE_ME_INITIAL_VALUE".to_string()
        }
    }
}

pub fn validate_api_key(req: &Request, expected_key: &str) -> bool {
    let provided_key = req.headers().get("x-api-key")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    
    // 定数時間比較(Constant time comparison)が理想ですが、まずは単純比較
    provided_key == expected_key
}
