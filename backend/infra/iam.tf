# Lambda関数が引き受けるIAMロールの定義
resource "aws_iam_role" "lambda_exec_role" {
  name = "eng-app-lambda-role"

  # どのAWSサービスがこのロールを使えるか（ここではLambda）を定義
  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Action = "sts:AssumeRole"
      Effect = "Allow"
      Principal = {
        Service = "lambda.amazonaws.com"
      }
    }]
  })
}

# CloudWatchへのログ書き込み権限（AWS管理ポリシー）をアタッチ
resource "aws_iam_role_policy_attachment" "lambda_basic_execution" {
  role       = aws_iam_role.lambda_exec_role.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole"
}

# DynamoDBとSSM Parameter Storeへのアクセス権限（インラインポリシー）を追加
resource "aws_iam_role_policy" "lambda_app_policy" {
  name = "eng-app-lambda-policy"
  role = aws_iam_role.lambda_exec_role.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        # DynamoDBに対するCRUD操作の許可
        Effect = "Allow"
        Action = [
          "dynamodb:PutItem",
          "dynamodb:GetItem",
          "dynamodb:Scan",
          "dynamodb:Query",
          "dynamodb:DeleteItem",
          "dynamodb:UpdateItem"
        ]
        # 先に定義した dynamodb.tf のテーブルARNを参照
        Resource = aws_dynamodb_table.main.arn
      },
      {
        # SSM Parameter StoreからAPIキーを取得する許可
        Effect = "Allow"
        Action = [
          "ssm:GetParameter"
        ]
        Resource = "arn:aws:ssm:${var.aws_region}:${data.aws_caller_identity.current.account_id}:parameter/eng-app/*"
      }
    ]
  })
}

# AWSアカウントIDを取得するためのデータソース（ARN構築に使用）
data "aws_caller_identity" "current" {}
