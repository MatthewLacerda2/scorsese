# The local task runner: every gate CI runs, runnable before the push.
#
# `make help` prints the list. That list is the answer to "what has to be green
# before this merges?" — nothing else states the set, and reading
# `.github/workflows/ci.yml` to find out is the archaeology this file exists to
# end. Each recipe is the same command CI runs, `--locked` included, so a green
# `make gates` here means the same thing it means there.
#
# Two entry points, split by speed rather than by importance:
#
#   make pre-commit   fmt + the size gate. No build, well under a second. This
#                     is what the committed hook runs, so a file over the line
#                     limit never reaches a branch.
#   make gates        all of it, for use before opening a PR.
#
# Clippy is on the slow side deliberately — see the `clippy` target.
#
# Gates block; signals inform (CLAUDE.md, "Gates vs. signals"). `make gates`
# runs only gates. `make coverage` is a signal and is opt-in precisely so a
# local run never trains anyone to treat one as the other.

# Prerequisites run in order, and a gate that fails should be the last thing
# printed rather than one of several racing to the terminal.
.NOTPARALLEL:

# The size gate lives in its own workspace so it needs neither ffmpeg nor a
# build of scorsese. Every invocation of it points at that manifest.
LINT := --manifest-path tools/lint/Cargo.toml

# The hard gates, in the order `make gates` runs them: cheapest first, so the
# feedback that costs nothing arrives before the feedback that costs a build.
# `inventory` below holds this list to the ones documented as gates.
GATES := format size clippy docs test deny

.DEFAULT_GOAL := help
.PHONY: help setup gates pre-commit inventory $(GATES) format-fix coverage

##@ Everyday

help: ## Print this list
	@awk 'BEGIN { FS = ":.*##" } \
		/^##@/ { printf "\n%s\n", substr($$0, 5); next } \
		/^[a-z][a-z-]*:.*##/ { printf "  \033[1m%-12s\033[0m %s\n", $$1, $$2 }' \
		$(MAKEFILE_LIST)
	@echo

setup: ## Install the committed git hooks (one-time, covers every worktree)
	git config core.hooksPath .githooks
	@echo "hooks: core.hooksPath -> .githooks; 'make pre-commit' now runs before each commit."
	@echo "hooks: to commit anyway on a work-in-progress commit, use 'git commit --no-verify'."

pre-commit: format size ## The fast half: what the pre-commit hook runs
	@echo "pre-commit: ok"

gates: inventory $(GATES) ## Everything CI blocks on. Run this before opening a PR
	@echo "gates: all green -- $(GATES)"

##@ The gates

format: ## [gate] cargo fmt --check, workspace and tools/lint
	cargo fmt --all --check
	cargo fmt $(LINT) --all --check

# The gate's own tests run first, as they do in CI: a classifier that quietly
# misfiles a path is a gate that has stopped gating without saying so. Their
# output is held back unless they fail — this runs on every commit, and a wall
# of passing dots is how a hook's output stops being read.
size: ## [gate] Source files <= 300 lines, test files <= 150 (tools/lint, tested first)
	@out=$$(cargo test $(LINT) --locked --quiet 2>&1) \
		|| { printf '%s\n' "$$out" >&2; exit 1; }
	cargo run $(LINT) --locked --quiet

# On the slow side of the split, from measurement rather than taste. A no-op
# run is 0.1s and a leaf edit 0.3s, but editing anything in `crates/core`
# rebuilds every dependent — 3.4s — and that is the commit an agent makes most
# often. A fresh worktree pays 15s for the first one, and this repo runs
# several at a time. Worse than the seconds: clippy requires the workspace to
# compile, and a pre-commit hook that demands a compiling tree cannot be used
# to checkpoint a half-finished refactor — which is exactly the practice
# break-testing depends on. Formatting and the size gate need no build and
# never have that problem.
clippy: ## [gate] clippy -D warnings, workspace and tools/lint
	cargo clippy --workspace --all-targets --locked -- -D warnings
	cargo clippy $(LINT) --all-targets --locked -- -D warnings

docs: ## [gate] cargo doc with -D warnings: a broken intra-doc link is a failure
	RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked

test: ## [gate] The whole suite, golden renders included (needs ffmpeg on PATH)
	@command -v ffmpeg >/dev/null 2>&1 || { \
		echo "test: ffmpeg is not on PATH -- the render and golden tests need it." >&2; \
		echo "      Install it from your package manager, or point SCORSESE_FFMPEG at a binary." >&2; \
		exit 1; }
	cargo test --workspace --locked

deny: ## [gate] Supply chain: advisories, bans, sources, licenses
	@command -v cargo-deny >/dev/null 2>&1 || { \
		echo "deny: cargo-deny is not installed." >&2; \
		echo "      Install it with: cargo install --locked cargo-deny" >&2; \
		exit 1; }
	cargo deny check advisories bans sources licenses

##@ Signals — informational, never a merge gate

coverage: ## Which pub items no test reaches. A signal: no threshold, blocks nothing
	@command -v cargo-llvm-cov >/dev/null 2>&1 || { \
		echo "coverage: cargo-llvm-cov is not installed." >&2; \
		echo "          Install it with: cargo install --locked cargo-llvm-cov" >&2; \
		echo "          It also needs: rustup component add llvm-tools-preview" >&2; \
		exit 1; }
	@mkdir -p target
	cargo llvm-cov --workspace --locked --exclude-from-report scorsese-golden \
		--json --output-path target/coverage.json
	python3 .github/scripts/coverage-summary.py target/coverage.json

##@ Fixing

format-fix: ## Rewrite files to satisfy the format gate
	cargo fmt --all
	cargo fmt $(LINT) --all

# `make gates` is only trustworthy if it runs every gate, and the failure mode
# that matters is a gate quietly dropped from $(GATES) while its target stays
# in the list — a runner that reports success on a check it no longer runs is
# worse than no runner. So the two are cross-checked: every target documented
# `## [gate]` must appear in $(GATES), and every entry of $(GATES) must be
# documented as one. Adding a gate means touching both, and forgetting either
# fails here instead of silently narrowing what green means.
inventory:
	@documented=$$(awk -F: '/^[a-z][a-z-]*:.*## \[gate\]/ { print $$1 }' \
		$(MAKEFILE_LIST) | sort | tr '\n' ' '); \
	declared=$$(printf '%s\n' $(GATES) | sort | tr '\n' ' '); \
	if [ "$$documented" != "$$declared" ]; then \
		echo "make: the gate list disagrees with itself -- 'make gates' is not the full set." >&2; \
		echo "  documented '## [gate]': $$documented" >&2; \
		echo "  run by 'make gates':    $$declared" >&2; \
		exit 1; \
	fi
