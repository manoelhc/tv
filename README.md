# tv
Terraform Version control

A CLI tool for managing Terraform module versions in .tf files.

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

## Query Syntax

Queries follow the pattern: `block_type.block_label.attribute["index"]`

Examples:
- `module.example.source["ref"]` - Get/set the `ref` parameter in the `source` attribute of the `example` module
- `module.example.source["url"]` - Get/set the URL part of the `source` attribute
- `module.example.source["path"]` - Get/set the path part of the `source` attribute (for git sources with subdirectories)
- `module.example.variable` - Get/set the `variable` attribute of the `example` module

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
