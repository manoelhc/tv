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
    }

    Ok(())
}

#[derive(Debug)]
struct Query {
    block_type: String,
    block_label: String,
    attribute: String,
    index: Option<String>,
}

fn parse_query(query: &str) -> Result<Query> {
    // Expected format: module.name.source["ref"]
    // or: module.name.attribute

    let parts: Vec<&str> = query.split('.').collect();
    if parts.len() < 3 {
        return Err(anyhow!(
            "Query must have at least 3 parts: block_type.block_label.attribute"
        ));
    }

    let block_type = parts[0].to_string();
    let block_label = parts[1].to_string();

    // Parse the attribute and optional index
    let rest = parts[2..].join(".");
    let (attribute, index) = if let Some(bracket_start) = rest.find('[') {
        let bracket_end = rest
            .find(']')
            .ok_or_else(|| anyhow!("Unclosed bracket in query"))?;
        let attr = rest[..bracket_start].to_string();
        let idx = rest[bracket_start + 1..bracket_end]
            .trim_matches('"')
            .to_string();
        (attr, Some(idx))
    } else {
        (rest, None)
    };

    Ok(Query {
        block_type,
        block_label,
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
            // Check labels
            let labels: Vec<String> = block
                .labels
                .iter()
                .map(|l| l.as_str())
                .map(|s| s.to_string())
                .collect();

            if labels.first().map(|s| s.as_str()) == Some(&parsed_query.block_label) {
                // Find the attribute
                for attr_item in block.body.iter() {
                    if let Some(attr) = attr_item.as_attribute()
                        && attr.key.as_str() == parsed_query.attribute
                    {
                        let value_str = attr.value.to_string();

                        if let Some(ref index_key) = parsed_query.index {
                            // Need to extract the value from the string
                            // Looking for key=value pattern in the source string
                            return extract_param_from_source(&value_str, index_key);
                        }

                        return Ok(Some(value_str.trim().trim_matches('"').to_string()));
                    }
                }
            }
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
            // Check labels
            let labels: Vec<String> = block
                .labels
                .iter()
                .map(|l| l.as_str())
                .map(|s| s.to_string())
                .collect();

            if labels.first().map(|s| s.as_str()) == Some(&parsed_query.block_label) {
                // Find the attribute position
                let pos = block.body.iter().position(|s| {
                    s.as_attribute()
                        .map(|a| a.key.as_str() == parsed_query.attribute)
                        .unwrap_or(false)
                });

                if let Some(pos) = pos {
                    // Get current value if we need to modify a parameter
                    let new_value_str = if let Some(ref index_key) = parsed_query.index {
                        // Get the current value
                        if let Some(attr_struct) = block.body.get(pos) {
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
                    block.body.remove(pos);
                    block
                        .body
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
    }

    if !found {
        return Err(anyhow!(
            "Block not found: {}.{}",
            parsed_query.block_type,
            parsed_query.block_label
        ));
    }

    // Write back to file
    fs::write(&file_path, body.to_string())?;
    Ok(())
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
