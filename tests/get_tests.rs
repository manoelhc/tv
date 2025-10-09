mod common;

use tv::{get_value, extract_param_from_source, extract_url_from_source, extract_path_from_source};

#[test]
fn test_get_simple_module_source() {
    let (_dir, file) = common::create_test_tf_file(common::SIMPLE_MODULE_TF);
    
    let result = get_value("module.vpc.source", Some(file.as_path())).unwrap();
    assert_eq!(result, Some("git::https://github.com/terraform-aws-modules/terraform-aws-vpc.git?ref=v5.0.0".to_string()));
}

#[test]
fn test_get_simple_module_name() {
    let (_dir, file) = common::create_test_tf_file(common::SIMPLE_MODULE_TF);
    
    let result = get_value("module.vpc.name", Some(file.as_path())).unwrap();
    assert_eq!(result, Some("my-vpc".to_string()));
}

#[test]
fn test_get_module_source_with_ref_index() {
    let (_dir, file) = common::create_test_tf_file(common::SIMPLE_MODULE_TF);
    
    let result = get_value("module.vpc.source[\"ref\"]", Some(file.as_path())).unwrap();
    assert_eq!(result, Some("v5.0.0".to_string()));
}

#[test]
fn test_get_module_source_with_url_index() {
    let (_dir, file) = common::create_test_tf_file(common::SIMPLE_MODULE_TF);
    
    let result = get_value("module.vpc.source[\"url\"]", Some(file.as_path())).unwrap();
    assert_eq!(result, Some("git::https://github.com/terraform-aws-modules/terraform-aws-vpc.git".to_string()));
}

#[test]
fn test_get_module_source_with_path_index() {
    let (_dir, file) = common::create_test_tf_file(common::MODULE_WITH_PATH_TF);
    
    let result = get_value("module.example.source[\"path\"]", Some(file.as_path())).unwrap();
    assert_eq!(result, Some("modules/vpc".to_string()));
}

#[test]
fn test_get_nonexistent_attribute() {
    let (_dir, file) = common::create_test_tf_file(common::SIMPLE_MODULE_TF);
    
    let result = get_value("module.vpc.nonexistent", Some(file.as_path())).unwrap();
    assert_eq!(result, None);
}

#[test]
fn test_get_nonexistent_module() {
    let (_dir, file) = common::create_test_tf_file(common::SIMPLE_MODULE_TF);
    
    let result = get_value("module.nonexistent.source", Some(file.as_path())).unwrap();
    assert_eq!(result, None);
}

#[test]
fn test_get_terraform_block_nested_attribute() {
    let (_dir, file) = common::create_test_tf_file(common::TERRAFORM_BLOCK_TF);
    
    let result = get_value("terraform.required_providers.aws.source", Some(file.as_path())).unwrap();
    assert_eq!(result, Some("hashicorp/aws".to_string()));
}

#[test]
fn test_get_terraform_block_nested_version() {
    let (_dir, file) = common::create_test_tf_file(common::TERRAFORM_BLOCK_TF);
    
    let result = get_value("terraform.required_providers.aws.version", Some(file.as_path())).unwrap();
    assert_eq!(result, Some("6.15.0".to_string()));
}

#[test]
fn test_get_registry_module_version() {
    let (_dir, file) = common::create_test_tf_file(common::REGISTRY_MODULE_TF);
    
    let result = get_value("module.vpc.version", Some(file.as_path())).unwrap();
    assert_eq!(result, Some("5.0.0".to_string()));
}

#[test]
fn test_extract_param_ref() {
    let source = "git::https://github.com/org/repo.git?ref=v1.0.0";
    let result = extract_param_from_source(source, "ref").unwrap();
    assert_eq!(result, Some("v1.0.0".to_string()));
}

#[test]
fn test_extract_param_ref_with_path() {
    let source = "git::https://github.com/org/repo.git//modules/vpc?ref=v1.0.0";
    let result = extract_param_from_source(source, "ref").unwrap();
    assert_eq!(result, Some("v1.0.0".to_string()));
}

#[test]
fn test_extract_param_nonexistent() {
    let source = "git::https://github.com/org/repo.git?ref=v1.0.0";
    let result = extract_param_from_source(source, "nonexistent").unwrap();
    assert_eq!(result, None);
}

#[test]
fn test_extract_url_with_git_prefix() {
    let source = "git::https://github.com/org/repo.git?ref=v1.0.0";
    let result = extract_url_from_source(source);
    assert_eq!(result, "git::https://github.com/org/repo.git");
}

#[test]
fn test_extract_url_with_path() {
    let source = "git::https://github.com/org/repo.git//modules/vpc?ref=v1.0.0";
    let result = extract_url_from_source(source);
    assert_eq!(result, "git::https://github.com/org/repo.git");
}

#[test]
fn test_extract_url_without_git_prefix() {
    let source = "https://github.com/org/repo.git?ref=v1.0.0";
    let result = extract_url_from_source(source);
    assert_eq!(result, "https://github.com/org/repo.git");
}

#[test]
fn test_extract_path_with_path() {
    let source = "git::https://github.com/org/repo.git//modules/vpc?ref=v1.0.0";
    let result = extract_path_from_source(source);
    assert_eq!(result, Some("modules/vpc".to_string()));
}

#[test]
fn test_extract_path_without_path() {
    let source = "git::https://github.com/org/repo.git?ref=v1.0.0";
    let result = extract_path_from_source(source);
    assert_eq!(result, None);
}

#[test]
fn test_extract_path_with_query() {
    let source = "git::https://github.com/org/repo.git//path/to/module?ref=v1.0.0";
    let result = extract_path_from_source(source);
    assert_eq!(result, Some("path/to/module".to_string()));
}
