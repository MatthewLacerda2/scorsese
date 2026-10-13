# The Google Cloud side of the hosted web app, as code (#952): a project of
# scorsese's own and the APIs enabled on it. Google offers no API for the
# OAuth consent screen or a web application's OAuth client, so those two are
# made by hand in that project's console (docs/web.md, "Infrastructure, as
# code"); everything Terraform can reach is here.
#
# `deploy/terraform/tf google …` runs it with the maintainer's own gcloud
# login, and keeps the state out of the checkout.

terraform {
  required_version = ">= 1.9"

  backend "local" {}

  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "~> 7.0"
    }
  }
}

provider "google" {
  # Reads GOOGLE_OAUTH_ACCESS_TOKEN, which `tf` takes from
  # `gcloud auth print-access-token`: no key file to keep anywhere.
}
