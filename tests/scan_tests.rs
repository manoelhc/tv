mod common;

use tv::{scan_files, parse_scan_query, find_all_tf_files};

#[test]
fn test_scan_all_modules() {
    let files = vec![
        ("main.tf", common::SIMPLE_MODULE_TF),
        ("other.tf", common::REGISTRY_MODULE_TF),
    ];
    let temp_dir = common::create_test_dir_with_files(&files);
    
    let results = scan_files("module.*", temp_dir.path()).unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn test_scan_specific_module() {
    let files = vec![
        ("main.tf", common::SIMPLE_MODULE_TF),
        ("other.tf", common::REGISTRY_MODULE_TF),
    ];
    let temp_dir = common::create_test_dir_with_files(&files);
    
    let results = scan_files("module.vpc", temp_dir.path()).unwrap();
    assert_eq!(results.len(), 2); // Both have "vpc" module
}

#[test]
fn test_scan_module_with_source_attribute() {
    let files = vec![
        ("main.tf", common::SIMPLE_MODULE_TF),
        ("other.tf", common::REGISTRY_MODULE_TF),
    ];
    let temp_dir = common::create_test_dir_with_files(&files);
    
    let results = scan_files("module.*.source", temp_dir.path()).unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn test_scan_module_with_version_attribute() {
    let files = vec![
        ("main.tf", common::SIMPLE_MODULE_TF),
        ("other.tf", common::REGISTRY_MODULE_TF),
    ];
    let temp_dir = common::create_test_dir_with_files(&files);
    
    let results = scan_files("module.*.version", temp_dir.path()).unwrap();
    assert_eq!(results.len(), 1); // Only registry module has version
}

#[test]
fn test_scan_terraform_block() {
    let files = vec![
        ("main.tf", common::TERRAFORM_BLOCK_TF),
        ("other.tf", common::SIMPLE_MODULE_TF),
    ];
    let temp_dir = common::create_test_dir_with_files(&files);
    
    // Scan for terraform blocks - this should match files with terraform blocks
    let results = scan_files("terraform", temp_dir.path()).unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_scan_terraform_provider() {
    let files = vec![
        ("main.tf", common::TERRAFORM_BLOCK_TF),
        ("other.tf", common::SIMPLE_MODULE_TF),
    ];
    let temp_dir = common::create_test_dir_with_files(&files);
    
    let results = scan_files("terraform.required_providers.aws", temp_dir.path()).unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_scan_with_url_filter() {
    let files = vec![
        ("main.tf", common::SIMPLE_MODULE_TF),
        ("other.tf", common::MODULE_WITH_PATH_TF),
    ];
    let temp_dir = common::create_test_dir_with_files(&files);
    
    let results = scan_files(
        "module.*.source[url==\"git::https://github.com/terraform-aws-modules/terraform-aws-vpc.git\"]",
        temp_dir.path()
    ).unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_scan_with_ref_filter() {
    let files = vec![
        ("main.tf", common::SIMPLE_MODULE_TF),
        ("other.tf", common::MODULE_WITH_PATH_TF),
    ];
    let temp_dir = common::create_test_dir_with_files(&files);
    
    let results = scan_files(
        "module.*.source[ref==\"v5.0.0\"]",
        temp_dir.path()
    ).unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_scan_with_path_filter() {
    let files = vec![
        ("main.tf", common::SIMPLE_MODULE_TF),
        ("other.tf", common::MODULE_WITH_PATH_TF),
    ];
    let temp_dir = common::create_test_dir_with_files(&files);
    
    let results = scan_files(
        "module.*.source[path==\"modules/vpc\"]",
        temp_dir.path()
    ).unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_scan_nested_directories() {
    let files = vec![
        ("main.tf", common::SIMPLE_MODULE_TF),
        ("subdir/nested.tf", common::REGISTRY_MODULE_TF),
        ("subdir/deep/deep.tf", common::TERRAFORM_BLOCK_TF),
    ];
    let temp_dir = common::create_test_dir_with_files(&files);
    
    let results = scan_files("module.*", temp_dir.path()).unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn test_scan_no_matches() {
    let files = vec![
        ("main.tf", common::SIMPLE_MODULE_TF),
    ];
    let temp_dir = common::create_test_dir_with_files(&files);
    
    let results = scan_files("terraform.required_providers", temp_dir.path()).unwrap();
    assert_eq!(results.len(), 0);
}

#[test]
fn test_find_all_tf_files() {
    let files = vec![
        ("main.tf", common::SIMPLE_MODULE_TF),
        ("other.tf", common::REGISTRY_MODULE_TF),
        ("not_tf.txt", "not a terraform file"),
        ("subdir/nested.tf", common::TERRAFORM_BLOCK_TF),
    ];
    let temp_dir = common::create_test_dir_with_files(&files);
    
    let results = find_all_tf_files(temp_dir.path()).unwrap();
    assert_eq!(results.len(), 3); // Should find 3 .tf files
}

#[test]
fn test_parse_scan_query_simple_wildcard() {
    let query = parse_scan_query("module.*").unwrap();
    assert_eq!(query.block_type, "module");
    assert_eq!(query.block_label, None);
    assert_eq!(query.attribute, None);
}

#[test]
fn test_parse_scan_query_specific_module() {
    let query = parse_scan_query("module.vpc").unwrap();
    assert_eq!(query.block_type, "module");
    assert_eq!(query.block_label, Some("vpc".to_string()));
    assert_eq!(query.attribute, None);
}

#[test]
fn test_parse_scan_query_with_attribute() {
    let query = parse_scan_query("module.*.source").unwrap();
    assert_eq!(query.block_type, "module");
    assert_eq!(query.block_label, None);
    assert_eq!(query.attribute, Some("source".to_string()));
}

#[test]
fn test_parse_scan_query_terraform_nested() {
    let query = parse_scan_query("terraform.required_providers.aws").unwrap();
    assert_eq!(query.block_type, "terraform");
    assert_eq!(query.block_label, None);
    assert_eq!(query.nested_blocks, vec!["required_providers".to_string()]);
    assert_eq!(query.attribute, Some("aws".to_string()));
}

#[test]
fn test_parse_scan_query_with_filter() {
    let query = parse_scan_query("module.*.source[url==\"https://example.com\"]").unwrap();
    assert_eq!(query.block_type, "module");
    assert_eq!(query.attribute, Some("source".to_string()));
    assert!(query.filter.is_some());
    let filter = query.filter.unwrap();
    assert_eq!(filter.attribute, "url");
    assert_eq!(filter.value, "https://example.com");
}

#[test]
fn test_parse_scan_query_with_double_equals_filter() {
    let query = parse_scan_query("module.*.source[ref==\"v1.0.0\"]").unwrap();
    assert!(query.filter.is_some());
    let filter = query.filter.unwrap();
    assert_eq!(filter.attribute, "ref");
    assert_eq!(filter.value, "v1.0.0");
}

#[test]
fn test_scan_multiple_modules_in_one_file() {
    let files = vec![
        ("main.tf", common::MULTIPLE_MODULES_TF),
    ];
    let temp_dir = common::create_test_dir_with_files(&files);
    
    let results = scan_files("module.*", temp_dir.path()).unwrap();
    assert_eq!(results.len(), 2); // Two modules in one file
}

#[test]
fn test_scan_specific_module_in_multi_module_file() {
    let files = vec![
        ("main.tf", common::MULTIPLE_MODULES_TF),
    ];
    let temp_dir = common::create_test_dir_with_files(&files);
    
    let results = scan_files("module.eks", temp_dir.path()).unwrap();
    assert_eq!(results.len(), 1);
    
    let results_vpc = scan_files("module.vpc", temp_dir.path()).unwrap();
    assert_eq!(results_vpc.len(), 1);
}

#[test]
fn test_scan_returns_module_names() {
    let files = vec![
        ("main.tf", common::MULTIPLE_MODULES_TF),
    ];
    let temp_dir = common::create_test_dir_with_files(&files);
    
    let results = scan_files("module.*", temp_dir.path()).unwrap();
    assert_eq!(results.len(), 2);
    
    // Verify that module names are returned
    let module_names: Vec<String> = results.iter().map(|(_, name)| name.clone()).collect();
    assert!(module_names.contains(&"vpc".to_string()));
    assert!(module_names.contains(&"eks".to_string()));
}
