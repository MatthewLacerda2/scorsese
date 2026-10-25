# Where each visitor is, roughly, for nginx's access log (#1526): Cloudflare
# adds `cf-ipcity`, `cf-region-code` and the rest to every request it sends
# down the tunnel, and nginx logs city, region and country, believed only on
# the tunnel's door (deploy/nginx.conf). The headers it adds beyond those
# (latitude, longitude, postal code) reach nginx and are dropped there.
#
# This resource holds the zone's whole set of Managed Transforms: one left out
# here is off.
resource "cloudflare_managed_transforms" "site" {
  zone_id = local.zone_id

  managed_request_headers = [{
    id      = "add_visitor_location_headers"
    enabled = true
  }]
  managed_response_headers = []
}
