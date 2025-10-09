use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use hcl_edit::Ident;
use hcl_edit::expr::Expression;
use hcl_edit::structure::{Attribute, Body};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "tv")]
#[command(about = "Terraform Version control - manage module versions in .tf files", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get a value from a .tf file
    Get {
        /// Query path (e.g., module.name.source["ref"])
        query: String,
        /// Default value if not found
        #[arg(default_value = "")]
        default: String,
        /// Path to .tf file (defaults to current directory)
        #[arg(short, long)]
        file: Option<PathBuf>,
    },
    /// Set a value in a .tf file
    Set {
        /// Query path (e.g., module.name.source["ref"])
        query: String,
        /// Value to set
        value: String,
        /// Path to .tf file (defaults to current directory)
        #[arg(short, long)]
        file: Option<PathBuf>,
    },
    /// Scan for .tf files that match a query pattern
    Scan {
        /// Query pattern (e.g., module.*, terraform.required_providers.aws)
        query: String,
        /// Directory to scan (defaults to current directory)
        #[arg(short, long, default_value = ".")]
        dir: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Get {
            query,
            default,
            file,
        } => {
            let result = get_value(&query, file.as_deref())?;
            println!("{}", result.unwrap_or(default));
        }
        Commands::Set { query, value, file } => {
            set_value(&query, &value, file.as_deref())?;
        }
        Commands::Scan { query, dir } => {
            let files = scan_files(&query, &dir)?;
            for file in files {
                println!("{}", file.display());
            }
        }
    }

    Ok(())
}

#[derive(Debug)]
struct Query {
    block_type: String,
    block_label: Option<String>,
    nested_blocks: Vec<String>,
    attribute: String,
    index: Option<String>,
}

fn parse_query(query: &str) -> Result<Query> {
    // Expected formats:
    // - module.name.attribute (simple: block with label)
    // - module.name.source["ref"] (simple with index)
    // - terraform.required_providers.aws.source (nested: terraform block -> required_providers block -> aws object attr -> source field)

    let parts: Vec<&str> = query.split('.').collect();
    if parts.len() < 2 {
        return Err(anyhow!(
            "Query must have at least 2 parts: block_type.attribute or block_type.label.attribute"
        ));
    }

    let block_type = parts[0].to_string();
    
    // Parse the rest - could be label.attribute or nested.blocks.attribute
    // We need to figure out the last part with optional index as the attribute
    let rest = parts[1..].join(".");
    let (rest_without_index, index) = if let Some(bracket_start) = rest.find('[') {
        let bracket_end = rest
            .find(']')
            .ok_or_else(|| anyhow!("Unclosed bracket in query"))?;
        let rest_part = rest[..bracket_start].to_string();
        let idx = rest[bracket_start + 1..bracket_end]
            .trim_matches('"')
            .to_string();
        (rest_part, Some(idx))
    } else {
        (rest, None)
    };

    // Split the rest into parts
    let remaining_parts: Vec<&str> = rest_without_index.split('.').collect();
    
    if remaining_parts.is_empty() {
        return Err(anyhow!("Query must include an attribute"));
    }
    
    // The last part is always the attribute
    let attribute = remaining_parts.last().unwrap().to_string();
    
    // Everything in between is either a label or nested blocks
    let middle_parts: Vec<String> = remaining_parts[..remaining_parts.len() - 1]
        .iter()
        .map(|s| s.to_string())
        .collect();
    
    // Determine if we have a simple block_type.label.attribute pattern
    // or a nested block pattern
    let (block_label, nested_blocks) = if middle_parts.len() == 1 {
        // Simple pattern: module.vpc.source -> label is "vpc"
        (Some(middle_parts[0].clone()), vec![])
    } else if middle_parts.is_empty() {
        // Pattern: terraform.attribute -> no label
        (None, vec![])
    } else {
        // Nested pattern: terraform.required_providers.aws.source
        // Need to determine which parts are blocks vs attributes
        // For now, we'll assume all middle parts could be either blocks or attributes
        // and handle them dynamically
        (None, middle_parts.clone())
    };

    Ok(Query {
        block_type,
        block_label,
        nested_blocks,
        attribute,
        index,
    })
}

fn find_tf_file(path: Option<&std::path::Path>) -> Result<PathBuf> {
    if let Some(p) = path {
        if p.is_file() {
            return Ok(p.to_path_buf());
        }
        if p.is_dir() {
            // Find .tf files in directory
            let entries = fs::read_dir(p)?;
            for entry in entries {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("tf") {
                    return Ok(path);
                }
            }
            return Err(anyhow!("No .tf files found in directory"));
        }
        return Err(anyhow!("Invalid path: {:?}", p));
    }

    // Default: look in current directory
    let current_dir = std::env::current_dir()?;
    find_tf_file(Some(&current_dir))
}

fn get_value(query: &str, file: Option<&std::path::Path>) -> Result<Option<String>> {
    let parsed_query = parse_query(query)?;
    let file_path = find_tf_file(file)?;

    let content = fs::read_to_string(&file_path)
        .with_context(|| format!("Failed to read file: {:?}", file_path))?;

    let body: Body = content
        .parse()
        .with_context(|| format!("Failed to parse HCL: {:?}", file_path))?;

    // Find the block
    for structure in body.iter() {
        if let Some(block) = structure.as_block()
            && block.ident.as_str() == parsed_query.block_type
        {
            // Check labels if we expect one
            if let Some(ref expected_label) = parsed_query.block_label {
                let labels: Vec<String> = block
                    .labels
                    .iter()
                    .map(|l| l.as_str())
                    .map(|s| s.to_string())
                    .collect();

                if labels.first().map(|s| s.as_str()) != Some(expected_label.as_str()) {
                    continue;
                }
            }
            
            // Navigate through nested blocks if any
            let mut current_body = &block.body;
            let mut attr_path = vec![];
            
            for (idx, nested_name) in parsed_query.nested_blocks.iter().enumerate() {
                let mut found_as_block = false;
                
                // Try to find as a nested block first
                for item in current_body.iter() {
                    if let Some(nested_block) = item.as_block() {
                        let nested_ident = nested_block.ident.as_str();
                        let nested_labels: Vec<String> = nested_block
                            .labels
                            .iter()
                            .map(|l| l.as_str())
                            .map(|s| s.to_string())
                            .collect();
                        
                        if nested_ident == nested_name 
                            || nested_labels.first().map(|s| s.as_str()) == Some(nested_name) {
                            current_body = &nested_block.body;
                            found_as_block = true;
                            break;
                        }
                    }
                }
                
                // If not found as a block, treat remaining parts as attribute path
                if !found_as_block {
                    attr_path = parsed_query.nested_blocks[idx..].to_vec();
                    attr_path.push(parsed_query.attribute.clone());
                    break;
                }
            }
            
            // If we have an attribute path, navigate through object attributes
            if !attr_path.is_empty() {
                return navigate_object_attributes(current_body, &attr_path, parsed_query.index.as_deref());
            }
            
            // Find the attribute in the final body
            for attr_item in current_body.iter() {
                if let Some(attr) = attr_item.as_attribute()
                    && attr.key.as_str() == parsed_query.attribute
                {
                    let value_str = attr.value.to_string();

                    if let Some(ref index_key) = parsed_query.index {
                        return extract_param_from_source(&value_str, index_key);
                    }

                    return Ok(Some(value_str.trim().trim_matches('"').to_string()));
                }
            }
        }
    }

    Ok(None)
}

fn navigate_object_attributes(
    body: &hcl_edit::structure::Body,
    attr_path: &[String],
    index: Option<&str>,
) -> Result<Option<String>> {
    if attr_path.is_empty() {
        return Ok(None);
    }
    
    let first_attr = &attr_path[0];
    
    // Find the first attribute in the body
    for item in body.iter() {
        if let Some(attr) = item.as_attribute() {
            if attr.key.as_str() == first_attr {
                // Get the value and navigate deeper if needed
                let value_str = attr.value.to_string();
                
                if attr_path.len() == 1 {
                    // This is the final attribute
                    if let Some(index_key) = index {
                        return extract_param_from_source(&value_str, index_key);
                    }
                    return Ok(Some(value_str.trim().trim_matches('"').to_string()));
                } else {
                    // Need to navigate deeper into the object
                    return extract_from_object_string(&value_str, &attr_path[1..], index);
                }
            }
        }
    }
    
    Ok(None)
}

fn extract_from_object_string(
    object_str: &str,
    attr_path: &[String],
    index: Option<&str>,
) -> Result<Option<String>> {
    // Parse the object string to extract nested attribute value
    // object_str looks like: {source = "hashicorp/aws", version = "6.15.0"}
    // or multi-line:
    // {
    //   source = "hashicorp/aws"
    //   version = "6.15.0"
    // }
    
    if attr_path.is_empty() {
        return Ok(None);
    }
    
    let target_attr = &attr_path[0];
    
    // Clean up the object string - remove braces and whitespace
    let cleaned = object_str.trim().trim_matches(|c| c == '{' || c == '}').trim();
    
    // Parse line by line or by looking for the pattern
    // Look for pattern: attr_name = "value" or attr_name = value
    let pattern = format!("{} =", target_attr);
    if let Some(start_idx) = cleaned.find(&pattern) {
        let after_equals = &cleaned[start_idx + pattern.len()..].trim_start();
        
        // Extract the value - could be quoted or unquoted
        // Value ends at newline or comma or closing brace
        let value_end = after_equals
            .find(&[',', '\n', '}'][..])
            .unwrap_or(after_equals.len());
        let value = after_equals[..value_end].trim().trim_matches('"').to_string();
        
        if attr_path.len() == 1 {
            if let Some(index_key) = index {
                return extract_param_from_source(&format!("\"{}\"", value), index_key);
            }
            return Ok(Some(value));
        } else {
            // More nesting - recursively extract
            return extract_from_object_string(&value, &attr_path[1..], index);
        }
    }
    
    Ok(None)
}

fn extract_param_from_source(source: &str, param_name: &str) -> Result<Option<String>> {
    // Remove quotes from source string
    let source = source.trim().trim_matches('"');

    // Handle special cases for "url" and "path"
    if param_name == "url" {
        return Ok(Some(extract_url_from_source(source)));
    } else if param_name == "path" {
        return Ok(extract_path_from_source(source));
    }

    // Look for param_name=value pattern in query string
    if let Some(param_start) = source.find(&format!("{}=", param_name)) {
        let value_start = param_start + param_name.len() + 1;
        let remaining = &source[value_start..];

        // Value goes until end of string or next parameter
        let value_end = remaining.find('&').unwrap_or(remaining.len());
        let value = &remaining[..value_end];

        return Ok(Some(value.to_string()));
    }

    Ok(None)
}

fn extract_url_from_source(source: &str) -> String {
    // Extract URL from various source formats
    // Format: git::https://github.com/org/repo.git//path?ref=version
    // or: github.com/org/repo.git//path?ref=version
    // or: terraform-aws-modules/vpc/aws (registry)
    // or: ./modules/vpc (local)

    // Keep the git:: prefix if present
    let url_start = source;
    let search_start = if source.starts_with("git::") {
        // Skip the git:: prefix for searching but include it in result
        5
    } else {
        0
    };

    let mut url_end = source.len();

    // Remove path component (starts with // but not part of https://)
    // We need to find // that's NOT part of the protocol
    if let Some(protocol_end) = source[search_start..].find("://") {
        // Look for // after the protocol
        let absolute_protocol_end = search_start + protocol_end + 3;
        let after_protocol = &source[absolute_protocol_end..];
        if let Some(path_idx) = after_protocol.find("//") {
            // Found path delimiter after protocol
            url_end = absolute_protocol_end + path_idx;
        }
    } else {
        // No protocol, just look for //
        if let Some(path_idx) = source[search_start..].find("//") {
            url_end = search_start + path_idx;
        }
    }

    // Check if there's a query string before the path delimiter
    if let Some(query_idx) = source[..url_end].find('?') {
        url_end = query_idx;
    }

    url_start[..url_end].to_string()
}

fn extract_path_from_source(source: &str) -> Option<String> {
    // Extract path from git sources
    // Format: git::https://github.com/org/repo.git//path?ref=version
    // Path starts after // (but not the // in https://) and ends at ? or end of string

    // First, skip past any protocol (like https://)
    let search_start = if let Some(protocol_end) = source.find("://") {
        protocol_end + 3
    } else {
        0
    };

    if let Some(path_start) = source[search_start..].find("//") {
        let path_begin = search_start + path_start + 2;
        let remaining = &source[path_begin..];

        // Path ends at query string or end of string
        let path_end = remaining.find('?').unwrap_or(remaining.len());
        let path = &remaining[..path_end];

        if !path.is_empty() {
            return Some(path.to_string());
        }
    }

    None
}

fn navigate_to_nested_body_mut<'a>(
    mut body: &'a mut hcl_edit::structure::Body,
    nested_blocks: &[String],
) -> Result<&'a mut hcl_edit::structure::Body> {
    for nested_block_name in nested_blocks {
        let mut found = false;
        let mut idx = 0;
        
        // Find the index of the nested block
        for (i, item) in body.iter().enumerate() {
            if let Some(nested_block) = item.as_block() {
                let nested_ident = nested_block.ident.as_str();
                let nested_labels: Vec<String> = nested_block
                    .labels
                    .iter()
                    .map(|l| l.as_str())
                    .map(|s| s.to_string())
                    .collect();
                
                if nested_ident == nested_block_name 
                    || nested_labels.first().map(|s| s.as_str()) == Some(nested_block_name.as_str()) {
                    found = true;
                    idx = i;
                    break;
                }
            }
        }
        
        if !found {
            return Err(anyhow!("Nested block '{}' not found", nested_block_name));
        }
        
        // Navigate to the nested block's body
        if let Some(item) = body.get_mut(idx) {
            if let Some(nested_block) = item.as_block_mut() {
                body = &mut nested_block.body;
            } else {
                return Err(anyhow!("Expected block at index {}", idx));
            }
        } else {
            return Err(anyhow!("Could not get mutable reference at index {}", idx));
        }
    }
    
    Ok(body)
}

fn set_value(query: &str, value: &str, file: Option<&std::path::Path>) -> Result<()> {
    let parsed_query = parse_query(query)?;
    let file_path = find_tf_file(file)?;

    let content = fs::read_to_string(&file_path)
        .with_context(|| format!("Failed to read file: {:?}", file_path))?;

    let mut body: Body = content
        .parse()
        .with_context(|| format!("Failed to parse HCL: {:?}", file_path))?;

    // Find the block
    let mut found = false;
    for mut structure in body.iter_mut() {
        if let Some(block) = structure.as_block_mut()
            && block.ident.as_str() == parsed_query.block_type
        {
            // Check labels if we expect one
            if let Some(ref expected_label) = parsed_query.block_label {
                let labels: Vec<String> = block
                    .labels
                    .iter()
                    .map(|l| l.as_str())
                    .map(|s| s.to_string())
                    .collect();

                if labels.first().map(|s| s.as_str()) != Some(expected_label.as_str()) {
                    continue;
                }
            }
            
            // Navigate through nested blocks and determine if we need to handle object attributes
            let mut current_body = &mut block.body;
            let mut attr_path = vec![];
            let mut navigated_blocks = 0;
            
            for (idx, nested_name) in parsed_query.nested_blocks.iter().enumerate() {
                let mut found_as_block = false;
                
                // Try to find as a nested block first  
                // We need to check without borrowing mutably yet
                for item in current_body.iter() {
                    if let Some(nested_block) = item.as_block() {
                        let nested_ident = nested_block.ident.as_str();
                        let nested_labels: Vec<String> = nested_block
                            .labels
                            .iter()
                            .map(|l| l.as_str())
                            .map(|s| s.to_string())
                            .collect();
                        
                        if nested_ident == nested_name 
                            || nested_labels.first().map(|s| s.as_str()) == Some(nested_name) {
                            found_as_block = true;
                            break;
                        }
                    }
                }
                
                if found_as_block {
                    // Navigate using the helper function for the blocks we found
                    navigated_blocks = idx + 1;
                } else {
                    // Rest are object attributes
                    attr_path = parsed_query.nested_blocks[idx..].to_vec();
                    attr_path.push(parsed_query.attribute.clone());
                    break;
                }
            }
            
            // Navigate to the deepest block level
            if navigated_blocks > 0 {
                current_body = navigate_to_nested_body_mut(current_body, &parsed_query.nested_blocks[..navigated_blocks])?;
            }
            
            // If we have an attribute path, we need to update within an object
            if !attr_path.is_empty() {
                update_object_attribute(current_body, &attr_path, value, parsed_query.index.as_deref())?;
                found = true;
                break;
            }
            
            // Otherwise, handle as a direct attribute
            let pos = current_body.iter().position(|s| {
                s.as_attribute()
                    .map(|a| a.key.as_str() == parsed_query.attribute)
                    .unwrap_or(false)
            });

            if let Some(pos) = pos {
                // Get current value if we need to modify a parameter
                let new_value_str = if let Some(ref index_key) = parsed_query.index {
                    // Get the current value
                    if let Some(attr_struct) = current_body.get(pos) {
                        if let Some(attr) = attr_struct.as_attribute() {
                            let current_value = attr.value.to_string();
                            update_param_in_source(&current_value, index_key, value)?
                        } else {
                            return Err(anyhow!("Expected attribute at position"));
                        }
                    } else {
                        return Err(anyhow!("Attribute not found at position"));
                    }
                } else {
                    format!("\"{}\"", value)
                };

                // Create new attribute
                let new_expr: Expression = new_value_str.parse().with_context(|| {
                    format!("Failed to parse expression: {}", new_value_str)
                })?;
                let key = Ident::new(parsed_query.attribute.clone());
                let new_attr = Attribute::new(key, new_expr);

                // Remove old and insert new
                current_body.remove(pos);
                current_body
                    .try_insert(pos, new_attr)
                    .map_err(|_| anyhow!("Failed to insert attribute"))?;

                found = true;
                break;
            } else {
                return Err(anyhow!(
                    "Attribute '{}' not found in block",
                    parsed_query.attribute
                ));
            }
        }
    }

    if !found {
        return Err(anyhow!(
            "Block not found: {}",
            parsed_query.block_type
        ));
    }

    // Write back to file
    fs::write(&file_path, body.to_string())?;
    Ok(())
}

fn update_object_attribute(
    body: &mut hcl_edit::structure::Body,
    attr_path: &[String],
    new_value: &str,
    index: Option<&str>,
) -> Result<()> {
    if attr_path.is_empty() {
        return Err(anyhow!("Empty attribute path"));
    }
    
    let first_attr = &attr_path[0];
    
    // Find the first attribute in the body
    let pos = body.iter().position(|item| {
        item.as_attribute()
            .map(|a| a.key.as_str() == first_attr)
            .unwrap_or(false)
    });
    
    if let Some(pos) = pos {
        if let Some(item) = body.get(pos) {
            if let Some(attr) = item.as_attribute() {
                let current_value = attr.value.to_string();
                
                // Update the value within the object
                let new_value_str = if attr_path.len() == 1 {
                    // Direct attribute update
                    if let Some(index_key) = index {
                        update_param_in_source(&current_value, index_key, new_value)?
                    } else {
                        format!("\"{}\"", new_value)
                    }
                } else {
                    // Need to update nested attribute within object
                    update_in_object_string(&current_value, &attr_path[1..], new_value, index)?
                };
                
                // Create new attribute with updated value
                let new_expr: Expression = new_value_str.parse().with_context(|| {
                    format!("Failed to parse expression: {}", new_value_str)
                })?;
                let key = Ident::new(first_attr.clone());
                let new_attr = Attribute::new(key, new_expr);
                
                // Remove old and insert new
                body.remove(pos);
                body.try_insert(pos, new_attr)
                    .map_err(|_| anyhow!("Failed to insert attribute"))?;
                
                return Ok(());
            }
        }
    }
    
    Err(anyhow!("Attribute '{}' not found", first_attr))
}

fn update_in_object_string(
    object_str: &str,
    attr_path: &[String],
    new_value: &str,
    index: Option<&str>,
) -> Result<String> {
    // Update a value within an object string
    // object_str looks like: {source = "hashicorp/aws", version = "6.15.0"}
    // or multi-line:
    // {
    //   source = "hashicorp/aws"
    //   version = "6.15.0"
    // }
    
    if attr_path.is_empty() {
        return Err(anyhow!("Empty attribute path"));
    }
    
    let target_attr = &attr_path[0];
    
    // Parse the object structure
    let trimmed = object_str.trim();
    let opening_brace = if let Some(pos) = trimmed.find('{') {
        &trimmed[..=pos]
    } else {
        ""
    };
    
    let closing_brace_pos = trimmed.rfind('}').unwrap_or(trimmed.len());
    let closing_brace = if closing_brace_pos < trimmed.len() {
        &trimmed[closing_brace_pos..]
    } else {
        ""
    };
    
    // Get the content between braces
    let content_start = if !opening_brace.is_empty() {
        opening_brace.len()
    } else {
        0
    };
    let content = &trimmed[content_start..closing_brace_pos];
    
    // Find and replace the attribute value
    let pattern = format!("{} =", target_attr);
    if let Some(start_idx) = content.find(&pattern) {
        let before_attr = &content[..start_idx];
        let after_equals_start = start_idx + pattern.len();
        let after_equals = &content[after_equals_start..];
        
        // Find where the old value ends (looking for newline, comma, or end)
        let mut value_end = after_equals.len();
        for (idx, ch) in after_equals.char_indices() {
            if ch == '\n' || ch == ',' {
                value_end = idx;
                break;
            }
        }
        
        // Extract whitespace before and after the value
        let whitespace_before = after_equals[..after_equals.len().min(value_end)]
            .chars()
            .take_while(|c| c.is_whitespace() && *c != '\n')
            .collect::<String>();
        let value_start_in_after = whitespace_before.len();
        let after_value = &after_equals[value_end..];
        
        // Format the new value
        let formatted_new_value = if index.is_some() {
            format!("\"{}\"", new_value)
        } else if attr_path.len() > 1 {
            // More nesting
            let old_value = after_equals[value_start_in_after..value_end].trim().trim_matches('"');
            update_in_object_string(old_value, &attr_path[1..], new_value, index)?
        } else {
            format!("\"{}\"", new_value)
        };
        
        // Reconstruct the object with better formatting
        let mut result = String::new();
        result.push_str(opening_brace);
        result.push_str(before_attr);
        result.push_str(&pattern);
        result.push_str(&whitespace_before);
        result.push_str(&formatted_new_value);
        result.push_str(after_value);
        result.push_str(closing_brace);
        
        return Ok(result);
    }
    
    Err(anyhow!("Attribute '{}' not found in object", target_attr))
}

fn update_param_in_source(source: &str, param_name: &str, new_value: &str) -> Result<String> {
    // Remove quotes from source string
    let source = source.trim().trim_matches('"');

    // Handle special cases for "url" and "path"
    if param_name == "url" {
        return Ok(format!("\"{}\"", update_url_in_source(source, new_value)));
    } else if param_name == "path" {
        return Ok(format!("\"{}\"", update_path_in_source(source, new_value)));
    }

    // Look for param_name=value pattern in query string
    if let Some(param_start) = source.find(&format!("{}=", param_name)) {
        let value_start = param_start + param_name.len() + 1;
        let remaining = &source[value_start..];

        // Value goes until end of string or next parameter
        let value_end = remaining.find('&').unwrap_or(remaining.len());

        let mut result = String::new();
        result.push_str(&source[..value_start]);
        result.push_str(new_value);
        result.push_str(&remaining[value_end..]);

        return Ok(format!("\"{}\"", result));
    }

    // If parameter doesn't exist, add it to query string
    let separator = if source.contains('?') { "&" } else { "?" };
    Ok(format!(
        "\"{}{}{}={}\"",
        source, separator, param_name, new_value
    ))
}

fn update_url_in_source(source: &str, new_url: &str) -> String {
    // Replace URL part while preserving path and query string
    // Original: git::https://github.com/org/repo.git//path?ref=version
    // New URL: github.com/myorg/mymod.git
    // Result: github.com/myorg/mymod.git//path?ref=version
    //
    // The new URL replaces the entire URL including the git:: prefix if present

    // First, find where to search for path delimiter (skip protocol like https://)
    let search_start = if let Some(protocol_end) = source.find("://") {
        protocol_end + 3
    } else if source.starts_with("git::") {
        // If there's git:: but no protocol after it, search after git::
        5
    } else {
        0
    };

    // Extract path and query components (everything after the URL)
    let remaining_part = if let Some(path_idx) = source[search_start..].find("//") {
        // Found path delimiter
        &source[search_start + path_idx..]
    } else {
        // No path, check for query string
        if let Some(query_idx) = source.find('?') {
            &source[query_idx..]
        } else {
            ""
        }
    };

    // Reconstruct with new URL (which may or may not have git:: prefix)
    format!("{}{}", new_url, remaining_part)
}

fn update_path_in_source(source: &str, new_path: &str) -> String {
    // Replace path part while preserving URL and query string
    // Original: git::https://github.com/org/repo.git//path?ref=version
    // Keep: git::https://github.com/org/repo.git and ?ref=version

    // First, find where to search for path delimiter (skip protocol like https://)
    let search_start = if let Some(protocol_end) = source.find("://") {
        protocol_end + 3
    } else {
        0
    };

    let mut url_part = source;
    let mut query_part = "";

    // Look for path delimiter after protocol
    if let Some(path_idx) = source[search_start..].find("//") {
        let absolute_path_idx = search_start + path_idx;
        let before_path = &source[..absolute_path_idx];
        let after_path = &source[absolute_path_idx + 2..];

        // Check if there's a query string after the path
        if let Some(query_idx) = after_path.find('?') {
            query_part = &after_path[query_idx..];
        }

        url_part = before_path;
    } else {
        // No existing path, check for query string on the URL
        if let Some(query_idx) = source.find('?') {
            query_part = &source[query_idx..];
            url_part = &source[..query_idx];
        }
    }

    // Normalize the path - remove leading slash if present
    let normalized_path = if new_path.is_empty() {
        String::new()
    } else if let Some(stripped) = new_path.strip_prefix('/') {
        stripped.to_string()
    } else {
        new_path.to_string()
    };

    if normalized_path.is_empty() {
        format!("{}{}", url_part, query_part)
    } else {
        format!("{}//{}{}", url_part, normalized_path, query_part)
    }
}

#[derive(Debug)]
struct ScanQuery {
    block_type: String,
    block_label: Option<String>,  // None means wildcard
    nested_blocks: Vec<String>,
    attribute: Option<String>,  // None if we're just matching the block
    filter: Option<AttributeFilter>,
}

#[derive(Debug)]
struct AttributeFilter {
    attribute: String,
    value: String,
}

fn parse_scan_query(query: &str) -> Result<ScanQuery> {
    // Expected formats:
    // - module.* (all modules)
    // - module.vpc.source (specific module with attribute)
    // - terraform.required_providers.* (terraform block with nested required_providers)
    // - terraform.required_providers.aws (specific provider)
    // - module.*.source[url=="https://..."] (with filter)
    
    // First check if there's a filter
    let (query_part, filter) = if let Some(bracket_start) = query.find('[') {
        let bracket_end = query.find(']')
            .ok_or_else(|| anyhow!("Unclosed bracket in query"))?;
        let filter_str = &query[bracket_start + 1..bracket_end];
        let query_before_filter = &query[..bracket_start];
        
        // Parse filter: e.g., url=="https://..." or ref=="v1.0.0"
        let filter = parse_attribute_filter(filter_str)?;
        (query_before_filter, Some(filter))
    } else {
        (query, None)
    };
    
    let parts: Vec<&str> = query_part.split('.').collect();
    if parts.is_empty() {
        return Err(anyhow!("Query cannot be empty"));
    }
    
    let block_type = parts[0].to_string();
    
    if parts.len() == 1 {
        // Just block type: "module" or "terraform"
        return Ok(ScanQuery {
            block_type,
            block_label: None,
            nested_blocks: vec![],
            attribute: None,
            filter,
        });
    }
    
    // Parse remaining parts
    let remaining = &parts[1..];
    
    // Determine if block_type typically has labels (like "module") or not (like "terraform")
    let block_has_labels = block_type == "module" || block_type == "resource" || block_type == "data";
    
    let (block_label, content_start) = if block_has_labels {
        // For module/resource/data, second part is label (or wildcard)
        if remaining[0] == "*" {
            (None, 1)  // Wildcard label
        } else {
            (Some(remaining[0].to_string()), 1)
        }
    } else {
        // For terraform/variable/output/etc, no label
        (None, 0)
    };
    
    // Handle rest as nested blocks and/or attribute
    if content_start < remaining.len() {
        let rest_parts: Vec<String> = remaining[content_start..].iter().map(|s| s.to_string()).collect();
        
        // Last part could be attribute or wildcard
        if rest_parts.is_empty() {
            // No more parts after label
            Ok(ScanQuery {
                block_type,
                block_label,
                nested_blocks: vec![],
                attribute: None,
                filter,
            })
        } else if rest_parts.last().map(|s| s.as_str()) == Some("*") {
            // Ends with wildcard - all parts are nested blocks/paths
            let nested = rest_parts[..rest_parts.len()-1].to_vec();
            Ok(ScanQuery {
                block_type,
                block_label: None,  // Wildcard at end means any label
                nested_blocks: nested,
                attribute: None,
                filter,
            })
        } else {
            // Last part is specific attribute
            let attribute = rest_parts.last().unwrap().clone();
            let nested = if rest_parts.len() > 1 {
                rest_parts[..rest_parts.len()-1].to_vec()
            } else {
                vec![]
            };
            
            Ok(ScanQuery {
                block_type,
                block_label,
                nested_blocks: nested,
                attribute: Some(attribute),
                filter,
            })
        }
    } else {
        // No rest parts - just block type and label/wildcard
        Ok(ScanQuery {
            block_type,
            block_label,
            nested_blocks: vec![],
            attribute: None,
            filter,
        })
    }
}

fn parse_attribute_filter(filter_str: &str) -> Result<AttributeFilter> {
    // Parse: url=="value" or ref=="value" or path=="value"
    // Also support single equals for matching
    
    let (attribute, rest) = if let Some(pos) = filter_str.find("==") {
        (&filter_str[..pos], &filter_str[pos+2..])
    } else if let Some(pos) = filter_str.find('=') {
        (&filter_str[..pos], &filter_str[pos+1..])
    } else {
        return Err(anyhow!("Invalid filter format: {}", filter_str));
    };
    
    let value = rest.trim().trim_matches('"').to_string();
    
    Ok(AttributeFilter {
        attribute: attribute.trim().to_string(),
        value,
    })
}

fn find_all_tf_files(dir: &std::path::Path) -> Result<Vec<PathBuf>> {
    let mut tf_files = Vec::new();
    
    if !dir.exists() {
        return Err(anyhow!("Directory does not exist: {:?}", dir));
    }
    
    if !dir.is_dir() {
        return Err(anyhow!("Path is not a directory: {:?}", dir));
    }
    
    fn visit_dir(dir: &std::path::Path, tf_files: &mut Vec<PathBuf>) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                visit_dir(&path, tf_files)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("tf") {
                tf_files.push(path);
            }
        }
        Ok(())
    }
    
    visit_dir(dir, &mut tf_files)?;
    Ok(tf_files)
}

fn scan_files(query: &str, dir: &std::path::Path) -> Result<Vec<PathBuf>> {
    let scan_query = parse_scan_query(query)?;
    let tf_files = find_all_tf_files(dir)?;
    
    let mut matching_files = Vec::new();
    
    for file_path in tf_files {
        if matches_query(&file_path, &scan_query)? {
            matching_files.push(file_path);
        }
    }
    
    Ok(matching_files)
}

fn matches_query(file_path: &std::path::Path, scan_query: &ScanQuery) -> Result<bool> {
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {:?}", file_path))?;
    
    let body: Body = content
        .parse()
        .with_context(|| format!("Failed to parse HCL: {:?}", file_path))?;
    
    // Look for blocks matching the query
    for structure in body.iter() {
        if let Some(block) = structure.as_block() {
            if block.ident.as_str() != scan_query.block_type {
                continue;
            }
            
            // Check block label if specified
            if let Some(ref expected_label) = scan_query.block_label {
                let labels: Vec<String> = block
                    .labels
                    .iter()
                    .map(|l| l.as_str().to_string())
                    .collect();
                
                if labels.first().map(|s| s.as_str()) != Some(expected_label.as_str()) {
                    continue;
                }
            }
            
            // If no nested blocks or attribute specified, we found a match
            if scan_query.nested_blocks.is_empty() && scan_query.attribute.is_none() {
                return Ok(true);
            }
            
            // Navigate through nested blocks
            let mut current_body = &block.body;
            
            for nested_name in &scan_query.nested_blocks {
                let mut found_this_level = false;
                
                for item in current_body.iter() {
                    if let Some(nested_block) = item.as_block() {
                        if nested_block.ident.as_str() == nested_name {
                            current_body = &nested_block.body;
                            found_this_level = true;
                            break;
                        }
                    }
                }
                
                if !found_this_level {
                    // Couldn't find nested block, so this file doesn't match
                    return Ok(false);
                }
            }
            
            // Check attribute if specified
            if let Some(ref attr_name) = scan_query.attribute {
                for item in current_body.iter() {
                    if let Some(attr) = item.as_attribute() {
                        if attr.key.as_str() == attr_name {
                            // Check filter if specified
                            if let Some(ref filter) = scan_query.filter {
                                let value_str = attr.value.to_string();
                                if !matches_filter(&value_str, filter)? {
                                    continue;
                                }
                            }
                            
                            return Ok(true);
                        }
                    }
                }
            } else {
                // No specific attribute required, nested blocks matched
                return Ok(true);
            }
        }
    }
    
    Ok(false)
}

fn matches_filter(value_str: &str, filter: &AttributeFilter) -> Result<bool> {
    // Extract the value based on the filter attribute (url, ref, path, etc.)
    let extracted = extract_param_from_source(value_str, &filter.attribute)?;
    
    if let Some(extracted_value) = extracted {
        Ok(extracted_value == filter.value)
    } else {
        Ok(false)
    }
}
