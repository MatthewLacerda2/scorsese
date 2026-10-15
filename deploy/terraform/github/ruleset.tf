# What `main` accepts: a change arrives as a squashed pull request with
# `CI ok` green (#1351's job, which stands for every job CI blocks on), and
# `main` is never deleted or force-pushed. No review is required: the
# maintainer does not read code. Nobody bypasses it. Made by hand on
# 2026-10-12, so imported.
import {
  to = github_repository_ruleset.main
  id = "scorsese:24966227"
}

resource "github_repository_ruleset" "main" {
  repository  = github_repository.scorsese.name
  name        = "main: every change arrives as a pull request"
  target      = "branch"
  enforcement = "active"

  conditions {
    ref_name {
      include = ["~DEFAULT_BRANCH"]
      exclude = []
    }
  }

  rules {
    deletion         = true
    non_fast_forward = true

    pull_request {
      required_approving_review_count = 0
      allowed_merge_methods           = ["squash"]
    }

    required_status_checks {
      required_check {
        context = "CI ok"
        # GitHub Actions, so no other app can post a `CI ok` that counts.
        integration_id = 15368
      }
      # The branch must hold `main`'s tip, so `CI ok` judged the tree that
      # lands. It costs nothing: every merge goes through `make queue`, which
      # rebases onto `main` and waits for the run on that head (Dependabot's
      # branch rebases itself, and the queue waits for it to).
      strict_required_status_checks_policy = true
    }
  }
}
