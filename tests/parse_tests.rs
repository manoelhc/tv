use tv::{parse_query, parse_scan_query, parse_attribute_filter};

#[test]
fn test_parse_query_simple_module() {
    let query = parse_query("module.vpc.source").unwrap();
    assert_eq!(query.block_type, "module");
    assert_eq!(query.block_label, Some("vpc".to_string()));
    assert_eq!(query.attribute, "source");
    assert_eq!(query.index, None);
}

#[test]
fn test_parse_query_with_index() {
    let query = parse_query("module.vpc.source[\"ref\"]").unwrap();
    assert_eq!(query.block_type, "module");
    assert_eq!(query.block_label, Some("vpc".to_string()));
    assert_eq!(query.attribute, "source");
    assert_eq!(query.index, Some("ref".to_string()));
}

#[test]
fn test_parse_query_with_index_no_quotes() {
    let query = parse_query("module.vpc.source[ref]").unwrap();
    assert_eq!(query.block_type, "module");
    assert_eq!(query.block_label, Some("vpc".to_string()));
    assert_eq!(query.attribute, "source");
    assert_eq!(query.index, Some("ref".to_string()));
}

#[test]
fn test_parse_query_terraform_nested() {
    let query = parse_query("terraform.required_providers.aws.source").unwrap();
    assert_eq!(query.block_type, "terraform");
    assert_eq!(query.block_label, None);
    assert_eq!(query.nested_blocks, vec!["required_providers".to_string(), "aws".to_string()]);
    assert_eq!(query.attribute, "source");
}

#[test]
fn test_parse_query_terraform_simple() {
    let query = parse_query("terraform.backend").unwrap();
    assert_eq!(query.block_type, "terraform");
    assert_eq!(query.block_label, None);
    assert_eq!(query.attribute, "backend");
}

#[test]
fn test_parse_query_too_short() {
    let result = parse_query("module");
    assert!(result.is_err());
}

#[test]
fn test_parse_query_unclosed_bracket() {
    let result = parse_query("module.vpc.source[ref");
    assert!(result.is_err());
}

#[test]
fn test_parse_scan_query_module_wildcard() {
    let query = parse_scan_query("module.*").unwrap();
    assert_eq!(query.block_type, "module");
    assert_eq!(query.block_label, None);
    assert_eq!(query.attribute, None);
    assert!(query.filter.is_none());
}

#[test]
fn test_parse_scan_query_module_specific() {
    let query = parse_scan_query("module.vpc").unwrap();
    assert_eq!(query.block_type, "module");
    assert_eq!(query.block_label, Some("vpc".to_string()));
    assert_eq!(query.attribute, None);
}

#[test]
fn test_parse_scan_query_module_with_attribute() {
    let query = parse_scan_query("module.vpc.source").unwrap();
    assert_eq!(query.block_type, "module");
    assert_eq!(query.block_label, Some("vpc".to_string()));
    assert_eq!(query.attribute, Some("source".to_string()));
}

#[test]
fn test_parse_scan_query_wildcard_with_attribute() {
    let query = parse_scan_query("module.*.source").unwrap();
    assert_eq!(query.block_type, "module");
    assert_eq!(query.block_label, None);
    assert_eq!(query.attribute, Some("source".to_string()));
}

#[test]
fn test_parse_scan_query_terraform_no_label() {
    let query = parse_scan_query("terraform.required_providers").unwrap();
    assert_eq!(query.block_type, "terraform");
    assert_eq!(query.block_label, None);
    assert_eq!(query.nested_blocks, vec![] as Vec<String>);
    assert_eq!(query.attribute, Some("required_providers".to_string()));
}

#[test]
fn test_parse_scan_query_with_filter() {
    let query = parse_scan_query("module.*.source[url==\"https://example.com\"]").unwrap();
    assert_eq!(query.block_type, "module");
    assert_eq!(query.attribute, Some("source".to_string()));
    assert!(query.filter.is_some());
}

#[test]
fn test_parse_scan_query_unclosed_bracket() {
    let result = parse_scan_query("module.*.source[url==\"test\"");
    assert!(result.is_err());
}

#[test]
fn test_parse_attribute_filter_double_equals() {
    let filter = parse_attribute_filter("url==\"https://example.com\"").unwrap();
    assert_eq!(filter.attribute, "url");
    assert_eq!(filter.value, "https://example.com");
}

#[test]
fn test_parse_attribute_filter_single_equals() {
    let filter = parse_attribute_filter("ref=\"v1.0.0\"").unwrap();
    assert_eq!(filter.attribute, "ref");
    assert_eq!(filter.value, "v1.0.0");
}

#[test]
fn test_parse_attribute_filter_no_quotes() {
    let filter = parse_attribute_filter("ref==v1.0.0").unwrap();
    assert_eq!(filter.attribute, "ref");
    assert_eq!(filter.value, "v1.0.0");
}

#[test]
fn test_parse_attribute_filter_with_spaces() {
    let filter = parse_attribute_filter("url == \"https://example.com\"").unwrap();
    assert_eq!(filter.attribute, "url");
    assert_eq!(filter.value, "https://example.com");
}

#[test]
fn test_parse_attribute_filter_invalid() {
    let result = parse_attribute_filter("invalid");
    assert!(result.is_err());
}

#[test]
fn test_parse_query_multiple_nested() {
    let query = parse_query("terraform.required_providers.aws.version").unwrap();
    assert_eq!(query.block_type, "terraform");
    assert_eq!(query.nested_blocks, vec!["required_providers".to_string(), "aws".to_string()]);
    assert_eq!(query.attribute, "version");
}

#[test]
fn test_parse_query_with_url_index() {
    let query = parse_query("module.vpc.source[\"url\"]").unwrap();
    assert_eq!(query.index, Some("url".to_string()));
}

#[test]
fn test_parse_query_with_path_index() {
    let query = parse_query("module.vpc.source[\"path\"]").unwrap();
    assert_eq!(query.index, Some("path".to_string()));
}
