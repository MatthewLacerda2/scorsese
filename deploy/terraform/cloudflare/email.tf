# The domain receives mail and never sends it. The one address in use is
# security@, published in web/public/.well-known/security.txt.

# Email Routing's own records (MX, SPF, DKIM), which Cloudflare writes and
# keeps when routing is enabled on the zone.
resource "cloudflare_email_routing_dns" "site" {
  zone_id = local.zone_id
}

resource "cloudflare_email_routing_rule" "security" {
  zone_id = local.zone_id
  name    = "security@ to the maintainer"
  enabled = true

  matchers = [{ type = "literal", field = "to", value = "security@${var.domain}" }]
  actions  = [{ type = "forward", value = [var.security_forward_to] }]
}

# Nothing legitimate is ever sent from this domain, so mail claiming to be
# from it is refused outright rather than reported on.
resource "cloudflare_dns_record" "dmarc" {
  zone_id = local.zone_id
  name    = "_dmarc.${var.domain}"
  type    = "TXT"
  content = "\"v=DMARC1; p=reject;\""
  ttl     = 1
}
