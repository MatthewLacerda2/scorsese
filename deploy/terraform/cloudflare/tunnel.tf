# The tunnel `cloudflared` holds open from the home server (the named-tunnel
# service in deploy/compose.yaml). It was created in the dashboard and is
# imported, never recreated: its token in deploy/.env must stay valid.
#
# Every hostname goes to nginx's tunnel-only door, which alone splits off
# /api (deploy/nginx.conf), so the ingress names hosts and nothing else.
resource "cloudflare_zero_trust_tunnel_cloudflared_config" "site" {
  account_id = local.account_id
  tunnel_id  = local.tunnel_id

  config = {
    ingress = [
      { hostname = var.domain, service = "http://web:80" },
      { hostname = "www.${var.domain}", service = "http://web:80" },
      # Required last: a request for any other host the tunnel is asked about.
      { service = "http_status:404" },
    ]
  }
}

# The hostnames reach the tunnel through proxied CNAMEs; the apex one is
# flattened by Cloudflare, which is what lets an apex be a CNAME at all.
resource "cloudflare_dns_record" "tunnel" {
  for_each = toset([var.domain, "www.${var.domain}"])

  zone_id = local.zone_id
  name    = each.value
  type    = "CNAME"
  content = "${local.tunnel_id}.cfargotunnel.com"
  proxied = true
  ttl     = 1 # "automatic", the only TTL a proxied record takes
}

# www is an alias: one address for the site, so links and cookies agree.
resource "cloudflare_ruleset" "redirects" {
  zone_id = local.zone_id
  name    = "Redirects"
  kind    = "zone"
  phase   = "http_request_dynamic_redirect"

  rules = [{
    description = "www to the apex, keeping the path and query"
    expression  = "(http.host eq \"www.${var.domain}\")"
    action      = "redirect"
    action_parameters = {
      from_value = {
        status_code           = 301
        preserve_query_string = true
        target_url = {
          expression = "concat(\"https://${var.domain}\", http.request.uri.path)"
        }
      }
    }
  }]
}
