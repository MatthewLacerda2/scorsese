variable "state_bucket" {
  description = "The bucket every configuration's Terraform state lives in; deploy/terraform/tf names it too."
  type        = string
  default     = "scorseseestudio-tfstate"
}

# Every configuration's state, this one's included, one prefix each
# (cloudflare, github, google, stripe-test, stripe-live): `tf` points each
# configuration's `gcs` backend here. The states hold secrets (the tunnel
# token, Stripe's signing secret), so nothing in the bucket is ever public.
#
# us-central1 is in Cloud Storage's Always Free tier (5 GB-month of regional
# standard storage in a US region); the states are kilobytes.
resource "google_storage_bucket" "state" {
  project  = google_project.scorsese.project_id
  name     = var.state_bucket
  location = "US-CENTRAL1"

  storage_class               = "STANDARD"
  uniform_bucket_level_access = true
  public_access_prevention    = "enforced"
  force_destroy               = false

  # Every apply writes a new version and keeps the one before, so a state
  # broken by a bad apply or a bad migration is one restore away.
  versioning {
    enabled = true
  }

  # An old version goes once it is both more than 30 days old and 20 versions
  # behind. Either condition alone loses something: a count alone lets one
  # afternoon of imports push every good version out; an age alone leaves
  # nothing to go back to after a quiet month. States are kilobytes, so
  # keeping both costs nothing.
  lifecycle_rule {
    condition {
      with_state                 = "ARCHIVED"
      days_since_noncurrent_time = 30
      num_newer_versions         = 20
    }
    action {
      type = "Delete"
    }
  }

  # Losing the bucket loses every configuration's record of what it made.
  lifecycle {
    prevent_destroy = true
  }

  depends_on = [google_project_service.apis]
}
