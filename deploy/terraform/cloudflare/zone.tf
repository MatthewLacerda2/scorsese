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
