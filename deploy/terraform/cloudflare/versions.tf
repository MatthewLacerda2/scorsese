# The Cloudflare side of the hosted web app, as code (#957): the tunnel's
# public hostnames, the zone's DNS, the www redirect, DNSSEC and the
# security@ mailbox. Everything Cloudflare lets Terraform manage is managed
# here and nowhere else; a change made in the dashboard is drift, and the
# next `make infra-plan` shows it as such.
#
# `deploy/terraform/tf cloudflare …` runs it with what deploy/.env holds, and
# keeps the state out of the checkout (docs/web.md, "Infrastructure, as code").

terraform {
  required_version = ">= 1.9"

  # The state's path is given at init by `tf`, under SCORSESE_DATA, so it
  # outlives the worktree a change was applied from.
  backend "local" {}

  required_providers {
    cloudflare = {
      source  = "cloudflare/cloudflare"
      version = "~> 5.27"
    }
  }
}

provider "cloudflare" {
  # Reads CLOUDFLARE_API_TOKEN from the environment; `tf` exports it.
}
