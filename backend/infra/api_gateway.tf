# API Gateway (HTTP API) の定義
resource "aws_apigatewayv2_api" "main" {
  name          = "eng-app-api"
  protocol_type = "HTTP"
  
  cors_configuration {
    allow_origins = [
      "http://localhost:5173",
      "https://${aws_cloudfront_distribution.frontend.domain_name}"
    ]
    allow_methods = ["GET", "POST", "DELETE", "OPTIONS"]
    allow_headers = ["content-type", "x-api-key"]
  }
}

# デフォルトステージ（自動デプロイを有効化）
resource "aws_apigatewayv2_stage" "default" {
  api_id      = aws_apigatewayv2_api.main.id
  name        = "$default"
  auto_deploy = true
}

# ========== Integrations (Lambdaとの接続) ==========

resource "aws_apigatewayv2_integration" "generate_text" {
  api_id                 = aws_apigatewayv2_api.main.id
  integration_type       = "AWS_PROXY"
  integration_uri        = aws_lambda_function.generate_text.invoke_arn
  payload_format_version = "2.0"
}

resource "aws_apigatewayv2_integration" "topics" {
  api_id                 = aws_apigatewayv2_api.main.id
  integration_type       = "AWS_PROXY"
  integration_uri        = aws_lambda_function.topics.invoke_arn
  payload_format_version = "2.0"
}

resource "aws_apigatewayv2_integration" "vocabulary" {
  api_id                 = aws_apigatewayv2_api.main.id
  integration_type       = "AWS_PROXY"
  integration_uri        = aws_lambda_function.vocabulary.invoke_arn
  payload_format_version = "2.0"
}

# ========== Routes (エンドポイントとIntegrationのマッピング) ==========

# /generate_text
resource "aws_apigatewayv2_route" "post_generate_text" {
  api_id    = aws_apigatewayv2_api.main.id
  route_key = "POST /generate_text"
  target    = "integrations/${aws_apigatewayv2_integration.generate_text.id}"
}

# /topics
resource "aws_apigatewayv2_route" "get_topics" {
  api_id    = aws_apigatewayv2_api.main.id
  route_key = "GET /topics"
  target    = "integrations/${aws_apigatewayv2_integration.topics.id}"
}

resource "aws_apigatewayv2_route" "post_topics" {
  api_id    = aws_apigatewayv2_api.main.id
  route_key = "POST /topics"
  target    = "integrations/${aws_apigatewayv2_integration.topics.id}"
}

resource "aws_apigatewayv2_route" "delete_topic" {
  api_id    = aws_apigatewayv2_api.main.id
  route_key = "DELETE /topics/{id}"
  target    = "integrations/${aws_apigatewayv2_integration.topics.id}"
}

# /vocabulary
resource "aws_apigatewayv2_route" "get_vocabulary" {
  api_id    = aws_apigatewayv2_api.main.id
  route_key = "GET /vocabulary"
  target    = "integrations/${aws_apigatewayv2_integration.vocabulary.id}"
}

resource "aws_apigatewayv2_route" "post_vocabulary" {
  api_id    = aws_apigatewayv2_api.main.id
  route_key = "POST /vocabulary"
  target    = "integrations/${aws_apigatewayv2_integration.vocabulary.id}"
}

resource "aws_apigatewayv2_route" "delete_vocabulary" {
  api_id    = aws_apigatewayv2_api.main.id
  route_key = "DELETE /vocabulary/{id}"
  target    = "integrations/${aws_apigatewayv2_integration.vocabulary.id}"
}
