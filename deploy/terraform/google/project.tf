variable "project_id" {
  description = "The project's ID, unique across all of Google Cloud and fixed once made."
  type        = string
  default     = "scorseseestudio"
}

variable "billing_account" {
  description = "The billing account the project is charged to: the maintainer's own, kept in reais."
  type        = string
  default     = "0118FD-436A50-6FFD59"
}

# scorsese's own project, so the sign-in client (#952), every configuration's
# Terraform state (state.tf) and, later, the Gemini key live apart from any
# other project of the maintainer's. Under no organisation: the maintainer's
# account has none. Billed since 2026-10-15, when the state bucket needed it:
# linked by hand in the console that day, and written here so the code says
# what is true. Linking or changing the account is an in-place update of the
# project, never a replacement; billing.tf's budget watches what it costs.
resource "google_project" "scorsese" {
  name            = "scorsese"
  project_id      = var.project_id
  billing_account = var.billing_account

  # A project holds the OAuth client every user signs in through; losing it
  # to a stray `destroy` would lock everyone out.
  deletion_policy = "PREVENT"
}

# The APIs the project uses. Sign-in itself needs none: the consent screen and
# the OAuth client are console settings of the project.
#
#   generativelanguage  Gemini images and Veo, ahead of the key's move here.
#   storage             the bucket the Terraform state lives in (state.tf).
#   billingbudgets      billing.tf's budget, whose calls are billed to this
#                       project (versions.tf says why).
resource "google_project_service" "apis" {
  for_each = toset([
    "generativelanguage.googleapis.com",
    "storage.googleapis.com",
    "billingbudgets.googleapis.com",
  ])

  project = google_project.scorsese.project_id
  service = each.value

  disable_on_destroy = false
}
