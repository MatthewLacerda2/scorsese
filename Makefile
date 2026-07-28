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
# runs only gates. `make coverage` and `make mutants` are signals and are
# opt-in precisely so a local run never trains anyone to treat one as the other.

# Prerequisites run in order, and a gate that fails should be the last thing
# printed rather than one of several racing to the terminal.
.NOTPARALLEL:

# The size gate lives in its own workspace so it needs neither ffmpeg nor a
# build of scorsese. Every invocation of it points at that manifest.
LINT := --manifest-path tools/lint/Cargo.toml

# The hard gates, in the order `make gates` runs them: cheapest first, so the
# feedback that costs nothing arrives before the feedback that costs a build.
# `inventory` below holds this list to the ones documented as gates.
GATES := format size scripts clippy docs test deny

.DEFAULT_GOAL := help
# `app` is on this list for a reason worth stating: there is a directory called
# `app/`, so without it make sees the target as already built and `make app`
# prints "up to date" without running a thing. A check that silently does
# nothing is worse than no check.
.PHONY: help setup gates pre-commit inventory $(GATES) app format-fix coverage mutants

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

# The two Python scripts render the coverage and mutation signals, and a signal
# that dies rendering its own report reads as the tool being broken — which is
# how a branch adding only tests used to be reported. They are a gate and not a
# signal because what is checked here is ordinary correctness, and because they
# cost a fraction of a second: stdlib `unittest`, no dependency to install.
scripts: ## [gate] The signal renderers under .github/scripts
	python3 -m unittest discover --start-directory .github/scripts/tests --quiet

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

# Diff-scoped, exactly as the `mutants` job runs it, because the useful question
# before a push is "would anything notice if what I just wrote were wrong?" and
# not "how is the whole codebase doing". `cargo mutants` on its own sweeps the
# full scoped surface — ~16 minutes — and is there when that is what you want.
#
# Surviving mutants exit 2 and timeouts exit 3; neither is a failure of the run,
# so neither fails this target. Anything else is the tool breaking and does.
#
# An empty diff is answered here rather than passed on: `cargo mutants` treats
# one as "nothing to do", exits 0 and leaves any previous `mutants.out` where it
# was — so handing that to the summary script would reprint an old report as if
# it were this branch's. Hence the check, and the `rm -rf` before a real run.
#
# A *non-empty* diff with nothing mutable in it is the other half of the same
# case, and it is not rare: a branch that only adds tests changes Rust files, so
# it passes the check above, and then cargo-mutants finds nothing in the mutated
# surface and writes no `outcomes.json`. The summary script answers that itself
# — see `NOTHING_TO_RUN` there — so the report is rendered unconditionally
# rather than guarded here, which is what keeps this and the CI job saying the
# same thing.
#
# What gets mutated, and what to do with a survivor: docs/mutation-testing.md.
mutants: ## Which changes to the code no test would notice. A signal: blocks nothing
	@command -v cargo-mutants >/dev/null 2>&1 || { \
		echo "mutants: cargo-mutants is not installed." >&2; \
		echo "         Install it with: cargo install --locked cargo-mutants" >&2; \
		exit 1; }
	@git rev-parse --verify --quiet origin/main >/dev/null || { \
		echo "mutants: origin/main is not in this clone -- there is no diff to scope to." >&2; \
		echo "         Fetch it with: git fetch origin main" >&2; \
		exit 1; }
	@mkdir -p target
	@git diff origin/main...HEAD -- '*.rs' > target/pr.diff
	@if [ ! -s target/pr.diff ]; then \
		echo "mutants: this branch changes no Rust source against origin/main."; \
	else \
		rm -rf mutants.out; \
		cargo mutants --in-diff target/pr.diff; status=$$?; \
		case $$status in 0|2|3) ;; *) exit $$status ;; esac; \
		python3 .github/scripts/mutants-summary.py mutants.out/outcomes.json; \
	fi

##@ Fixing

app: ## Build the desktop app (its own workspace; not part of `make gates`)
	cargo fmt --manifest-path app/Cargo.toml --all --check
	cargo clippy --manifest-path app/Cargo.toml --all-targets --locked -- -D warnings
	cargo build --manifest-path app/Cargo.toml --locked

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
