# A monthly budget on the project, so a cost nobody expected arrives as an
# email rather than on the statement: at half the amount and at all of it, to
# the billing account's admins (the default). A budget only warns; it never
# stops a service.
#
# In reais, because the billing account is, and Google refuses a budget in any
# currency but its account's: R$5 is about the US$1 a month the maintainer
# chose on 2026-10-15. What the project uses today is inside the free tier,
# so the first alert is the first real cost.
resource "google_billing_budget" "monthly" {
  provider = google.billing

  billing_account = var.billing_account
  display_name    = "scorseseestudio, monthly"

  budget_filter {
    projects = ["projects/${google_project.scorsese.number}"]
  }

  amount {
    specified_amount {
      currency_code = "BRL"
      units         = "5"
    }
  }

  threshold_rules {
    threshold_percent = 0.5
  }
  threshold_rules {
    threshold_percent = 1.0
  }

  depends_on = [google_project_service.apis]
}
