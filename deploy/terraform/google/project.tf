variable "project_id" {
  description = "The project's ID, unique across all of Google Cloud and fixed once made."
  type        = string
  default     = "scorseseestudio"
}

# scorsese's own project, so the sign-in client (#952) and, later, the Gemini
# key live apart from any other project of the maintainer's. Under no
# organisation: the maintainer's account has none. No billing account yet:
# sign-in needs none, and the Gemini key moves here (with billing) once the
# credits on the project it uses today run out.
resource "google_project" "scorsese" {
  name       = "scorsese"
  project_id = var.project_id

  # A project holds the OAuth client every user signs in through; losing it
  # to a stray `destroy` would lock everyone out.
  deletion_policy = "PREVENT"
}

# The Gemini API (Gemini images, Veo), enabled ahead of the key's move here.
# Sign-in itself needs no API enabled: the consent screen and the OAuth
# client are console settings of the project.
resource "google_project_service" "apis" {
  for_each = toset(["generativelanguage.googleapis.com"])

  project = google_project.scorsese.project_id
  service = each.value

  disable_on_destroy = false
}
