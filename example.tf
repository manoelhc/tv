module "vpc" {
source = "git::https://github.com/terraform-aws-modules/terraform-aws-vpc.git//modules/core?ref=v5.0.0"
  
  name = "my-vpc"
  cidr = "10.0.0.0/16"
}

module "eks" {
  source = "git::https://github.com/terraform-aws-modules/terraform-aws-eks.git?ref=v18.0.0"
  
  cluster_name = "my-cluster"
  cluster_version = "1.24"
}
