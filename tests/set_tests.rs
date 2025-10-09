mod common;

use tv::{set_value, get_value, update_param_in_source, update_url_in_source, update_path_in_source};

#[test]
fn test_set_simple_attribute() {
    let (_dir, file) = common::create_test_tf_file(common::SIMPLE_MODULE_TF);
    
    set_value("module.vpc.name", "new-vpc", Some(file.as_path())).unwrap();
    let result = get_value("module.vpc.name", Some(file.as_path())).unwrap();
    assert_eq!(result, Some("new-vpc".to_string()));
}

#[test]
fn test_set_module_source() {
    let (_dir, file) = common::create_test_tf_file(common::SIMPLE_MODULE_TF);
    
    set_value("module.vpc.source", "git::https://github.com/new/repo.git?ref=v2.0.0", Some(file.as_path())).unwrap();
    let result = get_value("module.vpc.source", Some(file.as_path())).unwrap();
    assert_eq!(result, Some("git::https://github.com/new/repo.git?ref=v2.0.0".to_string()));
}

#[test]
fn test_set_module_source_ref() {
    let (_dir, file) = common::create_test_tf_file(common::SIMPLE_MODULE_TF);
    
    set_value("module.vpc.source[\"ref\"]", "v6.0.0", Some(file.as_path())).unwrap();
    let result = get_value("module.vpc.source[\"ref\"]", Some(file.as_path())).unwrap();
    assert_eq!(result, Some("v6.0.0".to_string()));
    
    // Verify the URL is preserved
    let source = get_value("module.vpc.source[\"url\"]", Some(file.as_path())).unwrap();
    assert_eq!(source, Some("git::https://github.com/terraform-aws-modules/terraform-aws-vpc.git".to_string()));
}

#[test]
fn test_set_module_source_url() {
    let (_dir, file) = common::create_test_tf_file(common::SIMPLE_MODULE_TF);
    
    set_value("module.vpc.source[\"url\"]", "git::https://github.com/myorg/myvpc.git", Some(file.as_path())).unwrap();
    let result = get_value("module.vpc.source[\"url\"]", Some(file.as_path())).unwrap();
    assert_eq!(result, Some("git::https://github.com/myorg/myvpc.git".to_string()));
    
    // Verify the ref is preserved
    let ref_val = get_value("module.vpc.source[\"ref\"]", Some(file.as_path())).unwrap();
    assert_eq!(ref_val, Some("v5.0.0".to_string()));
}

#[test]
fn test_set_module_source_path() {
    let (_dir, file) = common::create_test_tf_file(common::MODULE_WITH_PATH_TF);
    
    set_value("module.example.source[\"path\"]", "modules/new-vpc", Some(file.as_path())).unwrap();
    let result = get_value("module.example.source[\"path\"]", Some(file.as_path())).unwrap();
    assert_eq!(result, Some("modules/new-vpc".to_string()));
    
    // Verify URL is preserved
    let url = get_value("module.example.source[\"url\"]", Some(file.as_path())).unwrap();
    assert_eq!(url, Some("git::https://github.com/org/repo.git".to_string()));
}

#[test]
fn test_set_terraform_nested_attribute() {
    let (_dir, file) = common::create_test_tf_file(common::TERRAFORM_BLOCK_TF);
    
    set_value("terraform.required_providers.aws.version", "7.0.0", Some(file.as_path())).unwrap();
    let result = get_value("terraform.required_providers.aws.version", Some(file.as_path())).unwrap();
    assert_eq!(result, Some("7.0.0".to_string()));
}

#[test]
fn test_set_registry_module_version() {
    let (_dir, file) = common::create_test_tf_file(common::REGISTRY_MODULE_TF);
    
    set_value("module.vpc.version", "6.0.0", Some(file.as_path())).unwrap();
    let result = get_value("module.vpc.version", Some(file.as_path())).unwrap();
    assert_eq!(result, Some("6.0.0".to_string()));
}

#[test]
fn test_update_param_ref() {
    let source = "\"git::https://github.com/org/repo.git?ref=v1.0.0\"";
    let result = update_param_in_source(source, "ref", "v2.0.0").unwrap();
    assert_eq!(result, "\"git::https://github.com/org/repo.git?ref=v2.0.0\"");
}

#[test]
fn test_update_param_add_new() {
    let source = "\"git::https://github.com/org/repo.git\"";
    let result = update_param_in_source(source, "ref", "v1.0.0").unwrap();
    assert_eq!(result, "\"git::https://github.com/org/repo.git?ref=v1.0.0\"");
}

#[test]
fn test_update_param_multiple_params() {
    let source = "\"git::https://github.com/org/repo.git?ref=v1.0.0&depth=1\"";
    let result = update_param_in_source(source, "ref", "v2.0.0").unwrap();
    assert_eq!(result, "\"git::https://github.com/org/repo.git?ref=v2.0.0&depth=1\"");
}

#[test]
fn test_update_url_preserves_query() {
    let source = "git::https://github.com/org/repo.git?ref=v1.0.0";
    let result = update_url_in_source(source, "git::https://github.com/neworg/newrepo.git");
    assert_eq!(result, "git::https://github.com/neworg/newrepo.git?ref=v1.0.0");
}

#[test]
fn test_update_url_preserves_path_and_query() {
    let source = "git::https://github.com/org/repo.git//modules/vpc?ref=v1.0.0";
    let result = update_url_in_source(source, "git::https://github.com/neworg/newrepo.git");
    assert_eq!(result, "git::https://github.com/neworg/newrepo.git//modules/vpc?ref=v1.0.0");
}

#[test]
fn test_update_url_simple() {
    let source = "git::https://github.com/org/repo.git";
    let result = update_url_in_source(source, "git::https://github.com/neworg/newrepo.git");
    assert_eq!(result, "git::https://github.com/neworg/newrepo.git");
}

#[test]
fn test_update_path_new_path() {
    let source = "git::https://github.com/org/repo.git?ref=v1.0.0";
    let result = update_path_in_source(source, "modules/vpc");
    assert_eq!(result, "git::https://github.com/org/repo.git//modules/vpc?ref=v1.0.0");
}

#[test]
fn test_update_path_replace_path() {
    let source = "git::https://github.com/org/repo.git//old/path?ref=v1.0.0";
    let result = update_path_in_source(source, "new/path");
    assert_eq!(result, "git::https://github.com/org/repo.git//new/path?ref=v1.0.0");
}

#[test]
fn test_update_path_remove_path() {
    let source = "git::https://github.com/org/repo.git//old/path?ref=v1.0.0";
    let result = update_path_in_source(source, "");
    assert_eq!(result, "git::https://github.com/org/repo.git?ref=v1.0.0");
}

#[test]
fn test_update_path_with_leading_slash() {
    let source = "git::https://github.com/org/repo.git?ref=v1.0.0";
    let result = update_path_in_source(source, "/modules/vpc");
    assert_eq!(result, "git::https://github.com/org/repo.git//modules/vpc?ref=v1.0.0");
}
