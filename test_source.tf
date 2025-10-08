module "example" {
  source = "git::https://github.com/myorg/mymod.git//module/rds?ref=v1.0.0"
  
  name = "test"
}

module "registry" {
  source = "terraform-aws-modules/vpc/aws"
  version = "5.0.0"
  
  name = "test-vpc"
}

module "local" {
  source = "./modules/vpc"
  
  name = "local-vpc"
}
