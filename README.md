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

Queries follow the pattern: `block_type.block_label.attribute[" index"]`

Examples:
- `module.example.source["ref"]` - Get/set the `ref` parameter in the `source` attribute of the `example` module
- `module.example.variable` - Get/set the `variable` attribute of the `example` module

## Examples

Given a Terraform file `main.tf`:

```hcl
module "example" {
  source = "git::https://github.com/example/repo.git?ref=v1.0.0"
  
  variable1 = "value1"
}
```

Get the current version:
```bash
$ tv get 'module.example.source["ref"]' --file main.tf
v1.0.0
```

Update the version:
```bash
$ tv set 'module.example.source["ref"]' v1.0.1 --file main.tf
```

Verify the update:
```bash
$ tv get 'module.example.source["ref"]' --file main.tf
v1.0.1
```
