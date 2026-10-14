# The Stripe side of the hosted web app, as code (#1256): the webhook endpoint
# Stripe tells the server how a credit purchase went through (#548).
#
# `deploy/terraform/tf stripe …` runs it with deploy/.env's STRIPE_SECRET_KEY.
# Test and live mode are separate worlds at Stripe, so `tf` keeps one state
# for each and picks it by the key's mode (docs/web.md, "Infrastructure, as
# code").

terraform {
  required_version = ">= 1.9"

  backend "local" {}

  required_providers {
    stripe = {
      source  = "stripe/stripe"
      version = "~> 0.3.0"
    }
  }
}

provider "stripe" {
  # Reads STRIPE_API_KEY from the environment; `tf` exports it from
  # STRIPE_SECRET_KEY.
}
