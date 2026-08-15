terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
  # 将来的にS3バックエンド等に変更する場合はここに追記
  # backend "s3" {}
}

provider "aws" {
  region  = var.aws_region

  default_tags {
    tags = {
      Project     = "eng-app"
      Environment = "dev"
      ManagedBy   = "Terraform"
    }
  }
}

variable "aws_region" {
  type    = string
  default = "ap-northeast-1"
}
