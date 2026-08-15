resource "aws_dynamodb_table" "main" {
  name         = "eng-app-table"
  billing_mode = "PAY_PER_REQUEST" # 無料枠活用・オンデマンドキャパシティ

  hash_key  = "PK"
  range_key = "SK"

  attribute {
    name = "PK"
    type = "S"
  }

  attribute {
    name = "SK"
    type = "S"
  }

  # シングルテーブル設計のため、PK/SK以外の属性は定義不要
}
