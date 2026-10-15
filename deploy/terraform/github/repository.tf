# The repository itself. It existed long before this file, so it is imported,
# never created, and nothing here may destroy it: `prevent_destroy` refuses a
# plan that would, and were that ever lifted, `archive_on_destroy` archives
# instead of deleting. Every field is what the repository already had, except
# the ones #1352 decided, each said where it is set.
import {
  to = github_repository.scorsese
  id = "scorsese"
}

resource "github_repository" "scorsese" {
  name        = "scorsese"
  description = "Video Editor with Claude Code and Google Veo support"
  visibility  = "public"

  has_issues      = true
  has_discussions = false
  # Neither was ever used: the wiki was never created and planning lives in
  # issues.
  has_wiki     = false
  has_projects = false

  # Squash only: the merge queue squashes (`gh pr merge --squash`), so one
  # pull request is one commit on `main`, titled by its pull request.
  allow_squash_merge          = true
  allow_merge_commit          = false
  allow_rebase_merge          = false
  squash_merge_commit_title   = "COMMIT_OR_PR_TITLE"
  squash_merge_commit_message = "COMMIT_MESSAGES"
  # The queue merges; GitHub's own auto-merge would be a second way in.
  allow_auto_merge = false
  # A merged pull request's branch is deleted. The pull request keeps its
  # commits and a *Restore branch* button.
  delete_branch_on_merge = true

  security_and_analysis {
    secret_scanning {
      status = "enabled"
    }
    secret_scanning_push_protection {
      status = "enabled"
    }
  }

  archive_on_destroy = true

  lifecycle {
    prevent_destroy = true
  }
}

# Dependabot alerts: a known vulnerability in a dependency, flagged. They are
# what the security updates below act on.
resource "github_repository_vulnerability_alerts" "scorsese" {
  repository = github_repository.scorsese.name
  enabled    = true
}

# A pull request from Dependabot whenever an alert has a fixed version.
# `.github/dependabot.yml` keeps the monthly version updates beside it.
resource "github_repository_dependabot_security_updates" "scorsese" {
  repository = github_repository_vulnerability_alerts.scorsese.repository
  enabled    = true
}
