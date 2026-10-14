variable "public_origin" {
  description = "deploy/.env's SCORSESE_PUBLIC_ORIGIN: where Stripe reaches the site."
  type        = string
}

# The three events crates/server's payments webhook acts on; anything else it
# would answer 200 and ignore, so Stripe is not asked to send it.
resource "stripe_webhook_endpoint" "payments" {
  url         = "${trimsuffix(var.public_origin, "/")}/api/payments/stripe/webhook"
  description = "scorsese: credit purchases (#548)"
  enabled_events = [
    "checkout.session.completed",
    "checkout.session.async_payment_succeeded",
    "checkout.session.expired",
  ]
}

# deploy/.env's STRIPE_WEBHOOK_SECRET. Stripe returns it only when the
# endpoint is created, so an imported endpoint has none here.
# `tf stripe secret` writes it into the env file without printing it.
output "webhook_secret" {
  value     = stripe_webhook_endpoint.payments.secret
  sensitive = true
}
