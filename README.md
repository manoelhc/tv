# tv
Terraform Version control

A CLI tool for managing Terraform module and provider versions in .tf files.

## Installation

```bash
cargo install --path .
```

## Usage

### Get a value

Get the value of a module attribute:

```bash
tv get 'module.example.source["ref"]' --file example.tf
```

Get a value with a default if not found:

```bash
tv get 'module.example.variable' default_value --file example.tf
```

### Set a value

Set the value of a module attribute:

```bash
tv set 'module.example.source["ref"]' v1.0.1 --file example.tf
```

Set a simple attribute value:

```bash
tv set 'module.example.variable' new_value --file example.tf
```

### Scan for files

Scan for .tf files that match a query pattern:

```bash
tv scan 'module.*' --dir .
```

Scan for files with specific attributes:

```bash
tv scan 'terraform.required_providers.aws' --dir .
```

Scan with filters:

```bash
tv scan 'module.*.source[url=="git::https://github.com/example/repo.git"]' --dir .
```

## Query Syntax

### For Get and Set Commands

Queries follow the pattern: `block_type.block_label.attribute["index"]`

Examples:
- `module.example.source["ref"]` - Get/set the `ref` parameter in the `source` attribute of the `example` module
- `module.example.source["url"]` - Get/set the URL part of the `source` attribute
- `module.example.source["path"]` - Get/set the path part of the `source` attribute (for git sources with subdirectories)
- `module.example.variable` - Get/set the `variable` attribute of the `example` module

### For Scan Command

Scan queries support wildcards (`*`) and filters:

Examples:
- `module.*` - Find all files with any module block
- `module.vpc` - Find files with a specific module named "vpc"
- `module.*.source` - Find files with modules that have a `source` attribute
- `module.*.version` - Find files with modules that have a `version` attribute
- `terraform.required_providers.*` - Find files with any required provider in terraform block
- `terraform.required_providers.aws` - Find files with AWS provider requirement
- `module.*.source[url=="git::https://github.com/example/repo.git"]` - Find modules with specific source URL
- `module.*.source[ref=="v1.0.0"]` - Find modules with specific version ref
- `module.*.source[path=="modules/vpc"]` - Find modules with specific subdirectory path

## Examples

### Working with Git Module Sources

Given a Terraform file `main.tf`:

```hcl
module "example" {
  source = "git::https://github.com/example/repo.git//modules/vpc?ref=v1.0.0"
  
  variable1 = "value1"
}
```

#### Get/Set the version (ref):
```bash
$ tv get 'module.example.source["ref"]' --file main.tf
v1.0.0

$ tv set 'module.example.source["ref"]' v1.0.1 --file main.tf

$ tv get 'module.example.source["ref"]' --file main.tf
v1.0.1
```

#### Get/Set the repository URL:
```bash
$ tv get 'module.example.source["url"]' --file main.tf
https://github.com/example/repo.git

$ tv set 'module.example.source["url"]' github.com/example/new-repo.git --file main.tf

$ tv get 'module.example.source["url"]' --file main.tf
github.com/example/new-repo.git
```

#### Get/Set the subdirectory path:
```bash
$ tv get 'module.example.source["path"]' --file main.tf
modules/vpc

$ tv set 'module.example.source["path"]' modules/rds --file main.tf

$ tv get 'module.example.source["path"]' --file main.tf
modules/rds
```

#### View the complete source:
```bash
$ tv get 'module.example.source' --file main.tf
git::github.com/example/new-repo.git//modules/rds?ref=v1.0.1
```

### Working with Registry Modules

For registry modules, use `version` instead of `ref`:

```hcl
module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "5.0.0"
}
```

```bash
$ tv get 'module.vpc.source' --file main.tf
terraform-aws-modules/vpc/aws

$ tv get 'module.vpc.version' --file main.tf
5.0.0
```

### Scanning for Terraform Files

The `scan` command helps you find .tf files that match specific patterns across your codebase.

#### Find all files with modules:
```bash
$ tv scan 'module.*' --dir .
./modules/networking/main.tf
./modules/compute/main.tf
./main.tf
```

#### Find files with specific provider requirements:
```bash
$ tv scan 'terraform.required_providers.aws' --dir .
./main.tf
./modules/networking/main.tf
```

#### Find files with any required provider:
```bash
$ tv scan 'terraform.required_providers.*' --dir .
./main.tf
./modules/networking/main.tf
./modules/compute/main.tf
```

#### Find modules with specific source URL:
```bash
$ tv scan 'module.*.source[url=="git::https://github.com/example/repo.git"]' --dir .
./main.tf
./modules/vpc/main.tf
```

#### Find modules with specific version:
```bash
$ tv scan 'module.*.source[ref=="v1.0.0"]' --dir .
./modules/vpc/main.tf
```

#### Find modules with version attribute (registry modules):
```bash
$ tv scan 'module.*.version' --dir .
./modules/rds/main.tf
```
