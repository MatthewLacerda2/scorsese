# The Cloudflare side of the hosted web app, as code (#957): the tunnel's
# public hostnames, the zone's DNS, the www redirect, DNSSEC and the
# security@ mailbox. Everything Cloudflare lets Terraform manage is managed
# here and nowhere else; a change made in the dashboard is drift, and the
# next `make infra-plan` shows it as such.
#
# `deploy/terraform/tf cloudflare …` runs it with what deploy/.env holds, and
# keeps the state in Google Cloud Storage (docs/web.md, "Infrastructure, as
# code").

terraform {
  required_version = ">= 1.9"

  # The bucket and prefix are given at init by `tf`: the state bucket the
  # google configuration makes, so the state outlives both the worktree a
  # change was applied from and the disk it was applied on.
  backend "gcs" {}

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
