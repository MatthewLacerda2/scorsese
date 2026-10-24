data "cloudflare_zone" "site" {
  filter = {
    name = var.domain
  }
}

locals {
  zone_id = data.cloudflare_zone.site.zone_id
  tunnel  = jsondecode(base64decode(var.tunnel_token))
  # Neither ID is secret, but both come from the token rather than a literal
  # so that one value names the tunnel everywhere.
  account_id = nonsensitive(local.tunnel.a)
  tunnel_id  = nonsensitive(local.tunnel.t)
}

resource "cloudflare_zone_dnssec" "site" {
  zone_id = local.zone_id
  status  = "active"
}

# HTTPS only. The session cookie is `Secure` either way, but without these the
# pages, the bundle and the API (an MCP client's token) answer over plain HTTP.
# Cloudflare sends every `http://` request a 301 before it reaches the tunnel.
resource "cloudflare_zone_setting" "always_use_https" {
  zone_id    = local.zone_id
  setting_id = "always_use_https"
  value      = "on"
}

# HSTS: a browser that has seen the site once never tries `http://` again.
# Six months; `www` is the only subdomain and is proxied, so it is covered.
# Not preloaded: the browsers' built-in list is slow to leave, so joining it is
# its own decision.
resource "cloudflare_zone_setting" "hsts" {
  zone_id    = local.zone_id
  setting_id = "security_header"
  value = {
    strict_transport_security = {
      enabled            = true
      max_age            = 15552000
      include_subdomains = true
      preload            = false
      nosniff            = false
    }
  }
}
