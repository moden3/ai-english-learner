# 1. /texts/generate 用のLambda関数 (GenerateTextFunction)
data "archive_file" "generate_text_zip" {
  type        = "zip"
  source_file = "${path.module}/../target/lambda/generate_text/bootstrap"
  output_path = "${path.module}/generate_text.zip"
}

resource "aws_lambda_function" "generate_text" {
  function_name    = "eng-app-generate-text"
  role             = aws_iam_role.lambda_exec_role.arn
  handler          = "bootstrap"
  runtime          = "provided.al2023" 
  filename         = data.archive_file.generate_text_zip.output_path
  source_code_hash = data.archive_file.generate_text_zip.output_base64sha256
  timeout          = 30 # AIの呼び出しがあるため少し長めに設定
}

# 2. /topics 用のLambda関数 (TopicsFunction)
data "archive_file" "topics_zip" {
  type        = "zip"
  source_file = "${path.module}/../target/lambda/topics/bootstrap"
  output_path = "${path.module}/topics.zip"
}

resource "aws_lambda_function" "topics" {
  function_name    = "eng-app-topics"
  role             = aws_iam_role.lambda_exec_role.arn
  handler          = "bootstrap"
  runtime          = "provided.al2023"
  filename         = data.archive_file.topics_zip.output_path
  source_code_hash = data.archive_file.topics_zip.output_base64sha256
}

# 3. /vocabulary 用のLambda関数 (VocabularyFunction)
data "archive_file" "vocabulary_zip" {
  type        = "zip"
  source_file = "${path.module}/../target/lambda/vocabulary/bootstrap"
  output_path = "${path.module}/vocabulary.zip"
}

resource "aws_lambda_function" "vocabulary" {
  function_name    = "eng-app-vocabulary"
  role             = aws_iam_role.lambda_exec_role.arn
  handler          = "bootstrap"
  runtime          = "provided.al2023"
  filename         = data.archive_file.vocabulary_zip.output_path
  source_code_hash = data.archive_file.vocabulary_zip.output_base64sha256
}

# ========== API Gatewayからの呼び出し許可 ==========

resource "aws_lambda_permission" "apigw_generate_text" {
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.generate_text.function_name
  principal     = "apigateway.amazonaws.com"
  source_arn    = "${aws_apigatewayv2_api.main.execution_arn}/*/*"
}

resource "aws_lambda_permission" "apigw_topics" {
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.topics.function_name
  principal     = "apigateway.amazonaws.com"
  source_arn    = "${aws_apigatewayv2_api.main.execution_arn}/*/*"
}

resource "aws_lambda_permission" "apigw_vocabulary" {
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.vocabulary.function_name
  principal     = "apigateway.amazonaws.com"
  source_arn    = "${aws_apigatewayv2_api.main.execution_arn}/*/*"
}
