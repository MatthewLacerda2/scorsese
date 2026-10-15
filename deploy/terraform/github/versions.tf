# The GitHub repository's own settings, as code (#1352): how a pull request
# merges, what happens to its branch, which security features are on, and the
# ruleset that makes `main` take changes only through a pull request with CI
# green. Labels are not here: `.github/labels.json` and its sync workflow keep
# them.
#
# `deploy/terraform/tf github …` runs it with the maintainer's own `gh` login,
# and keeps the state out of the checkout.

terraform {
  required_version = ">= 1.9"

  backend "local" {}

  required_providers {
    github = {
      source  = "integrations/github"
      version = "~> 6.13"
    }
  }
}

provider "github" {
  owner = "MatthewLacerda2"
  # Reads GITHUB_TOKEN, which `tf` takes from `gh auth token`: no key to keep
  # anywhere.
}
