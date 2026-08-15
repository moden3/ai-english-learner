# API Key (バックエンドとフロントエンドで共有する簡易認証キー)
resource "aws_ssm_parameter" "api_key" {
  name        = "/eng-app/api-key"
  description = "Simple API Key for authenticating client requests"
  type        = "SecureString"
  value       = "CHANGE_ME_INITIAL_VALUE" # デプロイ後にAWSコンソール等から安全な値に変更する想定

  lifecycle {
    ignore_changes = [value] # Terraform実行のたびに値が上書きされるのを防ぐ
  }
}

# Google Gemini API Key
resource "aws_ssm_parameter" "gemini_api_key" {
  name        = "/eng-app/gemini-api-key"
  description = "API Key for Google Gemini API"
  type        = "SecureString"
  value       = "CHANGE_ME_GEMINI_KEY" # デプロイ後にAWSコンソール等から実際のキーに変更する想定

  lifecycle {
    ignore_changes = [value]
  }
}
