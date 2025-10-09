use std::fs;
use tempfile::TempDir;

pub fn create_test_tf_file(content: &str) -> (TempDir, std::path::PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.tf");
    fs::write(&file_path, content).unwrap();
    (temp_dir, file_path)
}

pub fn create_test_dir_with_files(files: &[(&str, &str)]) -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    for (name, content) in files {
        let file_path = temp_dir.path().join(name);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(file_path, content).unwrap();
    }
    temp_dir
}

pub const SIMPLE_MODULE_TF: &str = r#"module "vpc" {
  source = "git::https://github.com/terraform-aws-modules/terraform-aws-vpc.git?ref=v5.0.0"
  
  name = "my-vpc"
  cidr = "10.0.0.0/16"
}
"#;

pub const MODULE_WITH_PATH_TF: &str = r#"module "example" {
  source = "git::https://github.com/org/repo.git//modules/vpc?ref=v1.0.0"
}
"#;

pub const TERRAFORM_BLOCK_TF: &str = r#"terraform {
  required_providers {
    aws = {
      source = "hashicorp/aws"
      version = "6.15.0"
    }
  }
}
"#;

pub const REGISTRY_MODULE_TF: &str = r#"module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "5.0.0"
}
"#;

pub const MULTIPLE_MODULES_TF: &str = r#"module "vpc" {
  source = "git::https://github.com/terraform-aws-modules/terraform-aws-vpc.git?ref=v5.0.0"
}

module "eks" {
  source = "git::https://github.com/terraform-aws-modules/terraform-aws-eks.git?ref=v18.0.0"
}
"#;
