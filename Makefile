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
# One gate is conditional, and only one: `app` runs when the branch touches
# `app/` and is skipped — reported, never silently — when it does not. That is
# what lets the desktop app's separate workspace be covered here without
# putting a `wgpu` build on the path of every headless commit. Its reasoning is
# at the target.
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
# `inventory` below holds this list to the ones documented as gates. `app` is
# last because it is the only one that can build a graphics dependency tree,
# and because it is the only one that may decide not to run at all — see it.
GATES := format size scripts clippy docs test deny app

# Whether this branch touches the desktop app, and so whether its gates are on
# the path of this change. The question is asked in two places — the `app` gate
# and the summary line `gates` prints — so it is written once, here.
#
# Two halves, because a branch is not only its commits. `origin/main...HEAD` is
# the committed diff, the same answer `make mutants` scopes itself to, so there
# is one definition of "what this branch changed" rather than two. `git status`
# is the other half: work under `app/` edited and not yet committed, which is
# the state `make gates` is most often run in.
#
# Markdown is excluded because CI excludes it. This workflow's own path filter
# drops `**.md`, so a branch changing only `app/README.md` starts no CI run at
# all — and answering "yes" to it here would build `eframe`, `wgpu` and `winit`
# to check a typo against a job that was never going to run. A conditional gate
# that expensive on that little is how people learn to route around it.
#
# When `origin/main` is not in the clone the question cannot be answered, and
# the answer is then *yes*. A skip has to be a decision; it must never be what
# not knowing looks like.
APP_PATHS := app/ ':(exclude)*.md'
TOUCHES_APP = { \
	! git rev-parse --verify --quiet origin/main >/dev/null 2>&1 \
	|| ! git diff --quiet origin/main...HEAD -- $(APP_PATHS) \
	|| [ -n "$$(git status --porcelain -- $(APP_PATHS))" ]; }

# Both test gates run through nextest, so both ask for it the same way, and a
# missing runner says what it is and how to get it rather than surfacing as
# `error: no such command: nextest` from cargo. The prebuilt tarball is offered
# first for the reason CI installs a binary too: building the runner costs more
# than the faster runs save.
NEXTEST_CHECK = command -v cargo-nextest >/dev/null 2>&1 || { \
	echo "nextest: cargo-nextest is not installed -- the suite runs through it." >&2; \
	echo "         Prebuilt, in seconds:  https://get.nexte.st/" >&2; \
	echo "         Or from source:        cargo install --locked cargo-nextest" >&2; \
	exit 1; }

.DEFAULT_GOAL := help
# `app` is on this list for a reason worth stating: there is a directory called
# `app/`, so without it make sees the target as already built and `make app`
# prints "up to date" without running a thing. A check that silently does
# nothing is worse than no check.
.PHONY: help setup gates pre-commit target-dir inventory $(GATES) app-gates release format-fix mcp-table coverage mutants mergeable

##@ Everyday

help: ## Print this list
	@awk 'BEGIN { FS = ":.*##" } \
		/^##@/ { printf "\n%s\n", substr($$0, 5); next } \
		/^[a-z][a-z-]*:.*##/ { printf "  \033[1m%-12s\033[0m %s\n", $$1, $$2 }' \
		$(MAKEFILE_LIST)
	@echo

setup: ## Once per clone: the committed git hooks, and the tools a gate needs
	git config core.hooksPath .githooks
	@echo "hooks: core.hooksPath -> .githooks; 'make pre-commit' now runs before each commit."
	@echo "hooks: to commit anyway on a work-in-progress commit, use 'git commit --no-verify'."
# The one tool a fresh clone needs that it does not already have. Said here
# rather than left to the first `make test` to discover, because this is the
# target a new checkout runs on purpose and that one is run in a hurry.
	@command -v cargo-nextest >/dev/null 2>&1 \
		&& echo "tools: cargo-nextest found -- 'make test' can run." \
		|| { echo "tools: cargo-nextest is missing -- both test gates run through it."; \
		     echo "tools: get it prebuilt from https://get.nexte.st/, or with"; \
		     echo "tools: 'cargo install --locked cargo-nextest'."; }

pre-commit: format size ## The fast half: what the pre-commit hook runs
	@echo "pre-commit: ok"

# `APP` is target-specific, so it reaches `app` below as a prerequisite of this
# and nowhere else: reaching a gate through `make gates` is scoped to the diff,
# asking for `make app` by name is not.
#
# The summary says which gates were run rather than which exist. A runner that
# prints "all green" over a check it decided not to run is the failure mode
# `inventory` was written to prevent, and skipping the app gates silently would
# be that failure mode arriving by a different door.
gates: APP := scoped
gates: target-dir inventory $(GATES) ## Everything CI blocks on. Run this before opening a PR
	@if $(TOUCHES_APP); then \
		echo "gates: all green -- $(GATES)"; \
	else \
		echo "gates: all green -- $(filter-out app,$(GATES))"; \
		echo "gates: app not run -- this branch changes nothing under app/."; \
	fi

# For a delivery render, and not much else. The dev profile is optimised (see
# the note in Cargo.toml), so the ordinary `cargo build` is already fast enough
# to iterate a cut with — this is the last ~2x, at the cost of a full rebuild
# and no debug assertions.
release: ## Optimised binaries, for a final render rather than for iterating
	cargo build --release
	@echo "release: binaries in target/release -- scorsese, scorsese-mcp"

##@ The gates

format: ## [gate] cargo fmt --check, workspace and tools/lint
	cargo fmt --all --check
	cargo fmt $(LINT) --all --check

# The gate's own tests run first, as they do in CI: a classifier that quietly
# misfiles a path is a gate that has stopped gating without saying so. Their
# output is held back unless they fail — this runs on every commit, and a wall
# of passing dots is how a hook's output stops being read.
#
# The one test run left on plain `cargo test`, and deliberately. This is in
# `pre-commit`, so putting it through nextest would mean a clone that has not
# installed the runner yet cannot commit at all — and there is nothing to win:
# four files, one test binary, nothing for a scheduler to overlap.
size: ## [gate] Source <= 300 lines of code, tests <= 150; blanks and comments free
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

# nextest rather than `cargo test`, and the reason is scheduling. This
# workspace has 31 integration test binaries; cargo runs them essentially one at
# a time and threads only *within* each, so the slow render and golden binaries
# leave the other cores idle while they finish. nextest puts every test from
# every binary into one global pool and gives each its own process. Measured
# warm on the dev machine (4 cores / 8 threads): 44.5s down to 34.0s, a failure
# prints the moment it happens instead of at the end, and a test that aborts now
# costs one result rather than every result in its binary.
#
# Doctests are the one thing nextest cannot run — an upstream limitation, not a
# nextest decision — so they get their own line. Every code fence in the tree is
# ```text or ```jsonc, so there are currently zero compiled doctests and this
# line takes a second. It is here so that the day someone writes a real one, it
# does not silently stop being run.
test: ## [gate] The whole suite, golden renders included (needs ffmpeg, nextest)
	@command -v ffmpeg >/dev/null 2>&1 || { \
		echo "test: ffmpeg is not on PATH -- the render and golden tests need it." >&2; \
		echo "      Install it from your package manager, or point SCORSESE_FFMPEG at a binary." >&2; \
		exit 1; }
	@$(NEXTEST_CHECK)
	cargo nextest run --workspace --locked
	cargo test --doc --workspace --locked

# The two Python scripts render the coverage and mutation signals, and a signal
# that dies rendering its own report reads as the tool being broken — which is
# how a branch adding only tests used to be reported. They are a gate and not a
# signal because what is checked here is ordinary correctness, and because they
# cost a fraction of a second: stdlib `unittest`, no dependency to install.
scripts: ## [gate] The signal renderers under .github/scripts
	python3 -m unittest discover --start-directory .github/scripts/tests --quiet

# Both workspaces, and the second line is the whole point. `app/` is its own
# cargo workspace with its own lockfile, so the first run does not see a single
# crate of it — 385 of them went unchecked for exactly that reason, `eframe`,
# `wgpu` and `winit` among them. `--config` aims the second run at the root
# policy rather than letting it look for an `app/deny.toml`, which must never
# exist: one policy, checked twice.
#
# `--all-features` on both, because CI's action passes it by default and this
# file promises to run the command CI runs. Without it a feature-gated
# dependency is in the graph there and not here, and `make deny` would go green
# on a tree CI rejects — which is the one thing this runner must never do.
deny: ## [gate] Supply chain, both workspaces: advisories, bans, sources, licenses
	@command -v cargo-deny >/dev/null 2>&1 || { \
		echo "deny: cargo-deny is not installed." >&2; \
		echo "      Install it with: cargo install --locked cargo-deny" >&2; \
		exit 1; }
	cargo deny --all-features check advisories bans sources licenses
	cargo deny --all-features --manifest-path app/Cargo.toml --config deny.toml \
		check advisories bans sources licenses

# The one gate that decides whether to run, and the condition is the point.
# `app/` is its own cargo workspace precisely so a headless change never pays
# for `eframe`, `wgpu` and `winit`; putting it in $(GATES) unconditionally would
# hand that cost back to every commit that has nothing to do with the window.
# So it runs when the branch touches `app/`, and says out loud when it does not
# — CI blocks on a `desktop app` job either way, and until this was here
# `make gates` could go green on a tree that job rejects. Four of the seven CI
# failures since the Makefile arrived were in exactly that blind spot.
#
# $(APP) is `scoped` only when this is reached through `make gates`. `make app`
# by name runs it outright: asking for a gate is a different act from running
# the set, and someone who types it wants the app built.
app: ## [gate] The desktop app's own workspace, when the branch touches app/
	@if [ "$(APP)" != "scoped" ] || $(TOUCHES_APP); then \
		$(MAKE) --no-print-directory app-gates; \
	else \
		echo "app: this branch changes nothing under app/ -- not run."; \
	fi

# Sound is linked, not optional: `cpal` reaches `alsa-sys`, whose build script
# asks pkg-config for `alsa.pc` and fails the compile without it. Said here with
# the fix, because the failure it produces otherwise is a panic inside a build
# script three crates down that names neither the package nor the reason. A
# sound *card* is not needed — the preview plays the picture silently without
# one, which is what CI does — only the headers.
app-gates:
	@{ command -v pkg-config >/dev/null 2>&1 && pkg-config --exists alsa; } || { \
		echo "app: the ALSA development headers are missing -- cpal cannot build without them." >&2; \
		echo "     Debian/Ubuntu: sudo apt-get install libasound2-dev" >&2; \
		echo "     Arch:          sudo pacman -S alsa-lib" >&2; \
		echo "     Fedora:        sudo dnf install alsa-lib-devel" >&2; \
		exit 1; }
	@$(NEXTEST_CHECK)
	cargo fmt --manifest-path app/Cargo.toml --all --check
	cargo clippy --manifest-path app/Cargo.toml --all-targets --locked -- -D warnings
	cargo build --manifest-path app/Cargo.toml --locked
# Running them, not only compiling them. `clippy --all-targets` and `build`
# already build the test targets, so a test that does not compile was caught —
# and one that fails was not, which reads as coverage while proving nothing.
# Through the same runner as the workspace gate, and with the same doctest line
# beside it, so "the tests" means one thing in this repo rather than two.
	cargo nextest run --manifest-path app/Cargo.toml --locked
	cargo test --doc --manifest-path app/Cargo.toml --locked

##@ Merging — asked of GitHub, not of the code

# Neither a gate nor a signal, because it is not about this code at all.
# `make gates` answers "is this good?"; this answers "did anything check it?",
# which is a question about GitHub and can only be asked once a pull request
# exists. It is the last step before `gh pr merge` — see #153 for the failure
# that made it necessary, and the script's own docstring for why branch
# protection is not the fix it looks like.
mergeable: ## Did CI really run on this PR's head? make mergeable PR=171
	@test -n "$(PR)" || { \
		echo "mergeable: which pull request? e.g. make mergeable PR=171" >&2; \
		exit 1; }
	@python3 .github/scripts/mergeable.py $(PR)

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
# full scoped surface — 3018 mutants — and is there when that is what you want.
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

format-fix: ## Rewrite files to satisfy the format gate
	cargo fmt --all
	cargo fmt $(LINT) --all

# The same relationship `format-fix` has to `format`, and for the same reason.
# The tool table in `docs/mcp.md` is generated from the MCP registry, the `test`
# gate fails when the checked-in page is not what the generator writes, and this
# is how it gets written. One source — the tool's own description and cost,
# which sit beside the `call` they describe — rather than two copies policed
# against each other.
#
# It runs the checking test with an environment variable set rather than being a
# second implementation, so what rewrites the page and what holds the page to it
# can never disagree about the answer.
mcp-table: ## Rewrite docs/mcp.md's tool table from the MCP registry
	UPDATE_MCP_TABLE=1 cargo test --locked -p scorsese-mcp --test table
	@echo "mcp-table: docs/mcp.md now says what the registry says."

# Neither a gate nor a signal: it checks the room the gates are about to run
# in, so it comes before them and is not one of them. `inventory` below is the
# same shape — both ask whether an answer from this Makefile would mean
# anything, before spending a build finding one out.
#
# Cargo keys build artifacts by package, version, features and profile, never
# by source path. Two worktrees pointed at one target directory therefore write
# the same `libscorsese_core.rlib` and the same test binaries to the same
# place, and each build overwrites the other's output for every crate it did
# not itself just rebuild. The phantom red that produces — a golden failing on
# a branch that touches no rendering code, a test that does not exist on this
# branch — costs an hour and announces itself. The false *green* does not: it
# says the gates passed on code that was never compiled, which is precisely the
# claim CLAUDE.md says a **ready** pull request makes. CI is unaffected, being
# a cold clean machine per branch, so the corrupted signal is the local one an
# agent is told to trust.
#
# Refusing beats documenting because the override is *tempting* — it looks like
# free disk — and a rule in a document gets re-derived away by the next session
# that wants some. There is nothing to configure here: cargo's default is
# already right, so an unset variable is the passing case.
#
# `CARGO_BUILD_TARGET_DIR` is checked beside it because cargo reads both, and a
# check that names only one of two doors is a check people walk around.
target-dir:
	@here=$$(pwd -P); \
	root=$$(git rev-parse --show-toplevel 2>/dev/null || printf '%s' "$$here"); \
	root=$$(cd "$$root" && pwd -P); \
	for pair in "CARGO_TARGET_DIR:$$CARGO_TARGET_DIR" \
	            "CARGO_BUILD_TARGET_DIR:$$CARGO_BUILD_TARGET_DIR"; do \
		var=$${pair%%:*}; dir=$${pair#*:}; \
		[ -n "$$dir" ] || continue; \
		case "$$dir" in /*) abs=$$dir;; *) abs=$$here/$$dir;; esac; \
		abs=$$(cd "$$abs" 2>/dev/null && pwd -P || printf '%s' "$$abs"); \
		case "$$abs" in "$$root"|"$$root"/*) continue;; esac; \
		echo "target-dir: $$var points outside this worktree -- the gates would not be about this branch." >&2; \
		echo "  $$var = $$dir" >&2; \
		echo "  worktree = $$root" >&2; \
		echo "  Cargo keys artifacts by package, not by path, so worktrees sharing one" >&2; \
		echo "  target directory overwrite each other and a green run stops meaning" >&2; \
		echo "  this branch compiled. Unset it -- cargo's default gives every worktree" >&2; \
		echo "  its own target/, which is the configuration this repo wants. See #259." >&2; \
		exit 1; \
	done

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
