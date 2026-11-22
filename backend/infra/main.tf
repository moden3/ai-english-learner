terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
  # S3バックエンドによるリモート状態（tfstate）の同期管理
  # ローカル開発環境とGitHub Actionsの両方で同じAWSリソース状態を共有する
  backend "s3" {
    bucket = "eng-app-tfstate-538884318891"
    key    = "eng-app/terraform.tfstate"
    region = "ap-northeast-1"
  }
}

provider "aws" {
  region = var.aws_region

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
