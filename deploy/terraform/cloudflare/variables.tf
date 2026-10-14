variable "domain" {
  description = "The apex the web app answers on. www redirects to it."
  type        = string
  default     = "scorseseestudio.com"
}

variable "tunnel_token" {
  description = <<-EOT
    deploy/.env's CLOUDFLARE_TUNNEL_TOKEN. It is base64 JSON naming the account
    (`a`) and the tunnel (`t`), so neither ID is written anywhere else. Only
    those two are read; the secret beside them never reaches a resource.
  EOT
  type        = string
  sensitive   = true
}

variable "security_forward_to" {
  description = <<-EOT
    Where mail to security@ and contato@ is forwarded: a destination address
    already verified in Email Routing. Kept in deploy/.env, not here, because it is
    the maintainer's personal mailbox and this repository is public.
  EOT
  type        = string
  sensitive   = true
}
