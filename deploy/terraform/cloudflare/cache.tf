# The landing page's HTML is kept at Cloudflare's edge, so a first visit is
# answered near the visitor rather than from the home server (the maintainer,
# 2026-10-14). Only `/`: every other page needs a session, and the files it
# loads are already fingerprinted and cached for good (deploy/nginx.conf).
#
# The HTML names those fingerprinted files, so after an update the kept copy
# names files the new build no longer has: `deploy/purge-landing` drops it,
# and docs/web.md's *Updating* runs it. The edge keeps it an hour at most, so a
# forgotten purge heals itself; browsers still revalidate it every time, as
# nginx's `no-cache` asks.
resource "cloudflare_ruleset" "cache" {
  zone_id = local.zone_id
  name    = "Cache"
  kind    = "zone"
  phase   = "http_request_cache_settings"

  rules = [{
    description = "The landing page's HTML at the edge"
    expression  = "(http.request.uri.path eq \"/\")"
    action      = "set_cache_settings"
    action_parameters = {
      cache       = true
      edge_ttl    = { mode = "override_origin", default = 3600 }
      browser_ttl = { mode = "respect_origin" }
    }
  }]
}
