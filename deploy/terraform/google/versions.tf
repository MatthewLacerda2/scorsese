# The Google Cloud side of the hosted web app, as code (#952): a project of
# scorsese's own, the APIs enabled on it, its billing budget, and the bucket
# every configuration's Terraform state lives in. Google offers no API for the
# OAuth consent screen or a web application's OAuth client, so those two are
# made by hand in that project's console (docs/web.md, "Infrastructure, as
# code"); everything Terraform can reach is here.
#
# `deploy/terraform/tf google …` runs it with the maintainer's own gcloud
# login.

terraform {
  required_version = ">= 1.9"

  # The bucket and prefix are given at init by `tf`. This configuration makes
  # that bucket, so its own state started on disk and moved in afterwards
  # (`tf google bootstrap`, docs/web.md).
  backend "gcs" {}

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

# The budget API refuses a gcloud user token billed to gcloud's own project
# ("end user credentials … are not supported"): each call has to name the
# project it is billed to. This alias does, for billing.tf only, so every
# other call goes on exactly as before.
provider "google" {
  alias                 = "billing"
  billing_project       = var.project_id
  user_project_override = true
}
