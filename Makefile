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
# Two gates are conditional: `app` runs when the branch touches `app/`, and
# `web` when it touches `web/`, and each is skipped — reported, never silently —
# when it does not. That is what lets the desktop app's separate workspace be
# covered here without putting a `wgpu` build on the path of every headless
# commit, and the React front-end without asking every Rust branch for Bun.
# The reasoning is at the `app` target; `web` is the same argument.
#
# One gate is *narrower* off Linux, which is a different thing: `test` runs
# everywhere, but the golden fixtures inside it skip themselves on any other
# platform. Both `test` and `gates` say so out loud, for the same reason `app`
# does — see `PIXEL_GATE_RUNS` below.
#
# Gates block; signals inform (CLAUDE.md, "Gates vs. signals"). `make gates`
# runs only gates. `make coverage`, `make mutants` and `make mutants-remote` are
# signals and are opt-in precisely so a local run never trains anyone to treat
# one as the other. CI runs neither on a pull request (#651): coverage runs
# weekly on `main`, mutation weekly by crate and otherwise only when asked.
#
# The last line `gates` prints is about one of those signals, and it is a
# *report* rather than a run: whether `make mutants` has been run on this
# branch, and what it found if it has. Running it from here would put minutes
# on the target that gets run most, which is how a signal turns into something
# people route around; saying nothing would leave the branch with no sighting
# of it at all, now that no pull-request run reports one (#340, #651). So the
# line names the commands, and never claims a result it does not have — the
# app gate's rule, applied to a signal.

# Prerequisites run in order, and a gate that fails should be the last thing
# printed rather than one of several racing to the terminal.
.NOTPARALLEL:

# The size gate lives in its own workspace so it needs neither ffmpeg nor a
# build of scorsese. Every invocation of it points at that manifest.
LINT := --manifest-path tools/lint/Cargo.toml

# The hard gates, in the order `make gates` runs them: cheapest first, so the
# feedback that costs nothing arrives before the feedback that costs a build.
# `inventory` below holds this list to the ones documented as gates. The two
# conditional ones come last, because they are the ones that may decide not to
# run at all; `app` after `web` because it is the one that can build a graphics
# dependency tree.
GATES := format size scripts clippy docs test deny deploy web app

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
#
# Written once as `TOUCHES`, with the paths as its argument, because the web
# gate asks the same question about `web/` and two copies of this would drift.
TOUCHES = { \
	! git rev-parse --verify --quiet origin/main >/dev/null 2>&1 \
	|| ! git diff --quiet origin/main...HEAD -- $(1) \
	|| [ -n "$$(git status --porcelain -- $(1))" ]; }
TOUCHES_APP = $(call TOUCHES,app/ ':(exclude)*.md')
TOUCHES_WEB = $(call TOUCHES,web/ ':(exclude)*.md')

# Whether the golden fixtures are among the tests `test` just ran. They are
# `#[ignore]`d on any target that is not Linux, because the references are
# blessed on Linux and compared on Linux by CI — off it the comparison measures
# the local ffmpeg's decode path as much as it measures the compositor, and the
# tolerances were never sized for that. `docs/golden-renders.md` argues it in
# full, including why this is keyed on the platform rather than on a decoder
# mismatch; #300 is where it was decided.
#
# Said here because nextest reports the skip only as a count, and a number in a
# summary is not a reader noticing that the pixel gate did not run. The rule is
# the app gate's: never green over a check that did not run.
#
# `uname -s` is the host, and `#[cfg]` is the target. Those are the same answer
# for every way this repo is built — nothing here cross-compiles, and the suite
# has to run on the machine it was built for anyway — so the duplication has no
# room to disagree. The day a cross-compile appears, this is the line to fix.
PIXEL_GATE_RUNS = [ "$$(uname -s)" = "Linux" ]
PIXEL_GATE_SKIPPED = pixel gate not run -- the golden fixtures are skipped on $$(uname -s); they are authoritative on Linux, where the references were blessed. See docs/golden-renders.md.

# What `make mutants` leaves behind so that `make gates` can say whether it
# ran: line 1 the branch it ran on, line 2 what it found, and its mtime the
# moment it finished. Under `target/`, which is gitignored, never shared
# between worktrees (see `target-dir`) and wiped by `cargo clean` — all three
# of which are the behaviour wanted here, the last one included: a stamp that
# is gone reads as *not run*, which is the safe direction to be wrong in.
#
# The branch is on it because `target/` outlives a checkout of a different
# branch in the same worktree, and a stamp from other work reporting a clean
# result on this one is exactly the false green this line exists to avoid.
MUTANTS_STAMP := target/mutants-signal

# Where cargo-mutants builds. It copies this worktree under `$TMPDIR` and
# compiles every mutant in the copy, and the default `/tmp` on the development
# machine is a 7.8 GB tmpfs — RAM, shared with the compile that is running in
# it. So the scratch goes on disk instead: a run neither competes for memory
# with itself nor depends on how much room /tmp happens to have (#392).
#
# `~/.cache` and not `target/`, deliberately. It is outside the tree being
# copied, which is what makes a scratch copy of the worktree structurally
# unable to land inside the worktree; it is per-user rather than per-worktree,
# and cargo-mutants names each run's directory uniquely, so parallel branches
# do not collide. cargo-mutants deletes its own directory when the run ends,
# and what a killed run leaves behind is in a cache directory, which is the one
# place a leftover is not litter.
MUTANTS_TMPDIR := $(or $(XDG_CACHE_HOME),$(HOME)/.cache)/scorsese/mutants

# A directory per run inside it, deleted on the way out — including when the
# run is interrupted, which is what the `trap` is for. Two reasons, and the
# second is the one that would otherwise bite: two worktrees can run this at
# once, so a blanket clean would delete the other one's scratch; and `TMPDIR`
# is where *the tests* put their own temporary directories too. The project
# fixtures under `crates/core/tests/common/` create a `scorsese-<label>.scor`
# per call and never remove it, which is invisible while it lands in a /tmp
# that is wiped at boot and would be a slow leak in a cache directory that is
# not. A run cleans up after itself instead.

# How many mutants are built and tested at once. Left unset, cargo-mutants
# takes the CPU count — 8 here — and each of those jobs is a full rustc build
# in a copy of the worktree, so the default sizes the run by the resource this
# machine has spare and ignores the one it does not. That is what OOM-killed
# the agent session twice in one day (#398), and it does not announce itself:
# Claude Code runs with `oom_score_adj: 200`, so the kernel reaps the session
# and leaves the compiler that ate the memory running.
#
# The number and the argument for it are in `.cargo/mutants.toml`, under "How
# wide a run fans out" — one place, because CI passes the same flag and
# cargo-mutants 27.1.0 has no config key to read it from. What belongs here
# instead is the override: a machine carrying more than usual asks for less
# with `make mutants MUTANTS_JOBS=1`, which beats editing a shared file for
# one afternoon's load.
MUTANTS_JOBS ?= 2

# What the run found, in one sentence, counted from the lists cargo-mutants
# writes beside its report rather than parsed back out of `outcomes.json`:
# those files are one mutant per line, so `wc -l` cannot half-understand a
# schema, and the Markdown report remains the one thing that reads the JSON.
#
# No `mutants.out` at all is the case the summary script calls "no mutants to
# run" — cargo-mutants writes nothing when the diff it was handed has no
# mutable lines in it, which is what a branch that only adds tests produces.
#
# The two cases below it are there so that an absence never renders as a clean
# bill of health: a `mutants.out` with no lists in it, and lists that add up to
# no mutants, are both a run that proved nothing, and they say so. Counting the
# empty file as zero survivors is what would turn either into "all 0 mutations
# were caught".
MUTANTS_VERDICT = \
	if [ ! -d mutants.out ]; then \
		echo "nothing in the changed lines is in the mutated surface"; \
	elif [ ! -f mutants.out/missed.txt ]; then \
		echo "the run left no result lists behind, so there is nothing to report"; \
	else \
		missed=$$(wc -l < mutants.out/missed.txt); \
		caught=$$(wc -l < mutants.out/caught.txt 2>/dev/null || echo 0); \
		timeout=$$(wc -l < mutants.out/timeout.txt 2>/dev/null || echo 0); \
		viable=$$((missed + caught + timeout)); \
		if [ "$$viable" -eq 0 ]; then \
			said="the run tested no mutants"; \
		elif [ "$$missed" -eq 0 ]; then \
			said="all $$viable mutations were caught"; \
			if [ "$$timeout" -gt 0 ]; then said="$$said ($$timeout by timing out)"; fi; \
		else \
			said="$$missed of $$viable mutations survived"; \
			if [ "$$timeout" -gt 0 ]; then said="$$said, $$timeout timed out"; fi; \
			said="$$said; they are listed in mutants.out/missed.txt"; \
		fi; \
		echo "$$said"; \
	fi

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
# `app` and `web` are on this list for a reason worth stating: there are
# directories called `app/` and `web/`, so without it make sees the target as
# already built and `make app` prints "up to date" without running a thing. A
# check that silently does nothing is worse than no check.
.PHONY: help setup gates pre-commit target-dir inventory $(GATES) app-gates web-gates release format-fix mcp-table coverage mutants mutants-remote mutants-status mergeable queue live-check

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

# `APP` and `WEB` are target-specific, so they reach `app` and `web` below as
# prerequisites of this and nowhere else: reaching a gate through `make gates`
# is scoped to the diff, asking for `make app` by name is not.
#
# The summary says which gates were run rather than which exist. A runner that
# prints "all green" over a check it decided not to run is the failure mode
# `inventory` was written to prevent, and skipping a conditional gate silently
# would be that failure mode arriving by a different door.
gates: APP := scoped
gates: WEB := scoped
gates: target-dir inventory $(GATES) ## Everything CI blocks on. Run this before opening a PR
	@ran="$(filter-out web app,$(GATES))"; \
	if $(TOUCHES_WEB); then ran="$$ran web"; fi; \
	if $(TOUCHES_APP); then ran="$$ran app"; fi; \
	echo "gates: all green -- $$ran"
	@$(TOUCHES_WEB) || echo "gates: web not run -- this branch changes nothing under web/."
	@$(TOUCHES_APP) || echo "gates: app not run -- this branch changes nothing under app/."
# `test` did run, and is on that list; part of what it covers did not. So this
# narrows the claim rather than removing a gate from it — which is why it reads
# differently from the app line above and has to be here at all.
	@$(PIXEL_GATE_RUNS) || echo "gates: $(PIXEL_GATE_SKIPPED)"
# Last, and about a signal rather than a gate — so it comes after the green
# line and can never change it. See `mutants-status`.
	@$(MAKE) --no-print-directory mutants-status

# For a delivery render, and not much else. The dev profile is optimised (see
# the note in Cargo.toml), so the ordinary `cargo build` is already fast enough
# to iterate a cut with — this is the last ~2x, at the cost of a full rebuild
# and no debug assertions.
release: ## Optimised binaries, for a final render rather than for iterating
	cargo build --release
	@echo "release: binaries in target/release -- scorsese, scorsese-mcp, scorsese-server"

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
#
# Postgres joins ffmpeg as something the suite needs rather than something it
# works around: the server's database tests fail without one instead of
# skipping, so `tools/with-postgres` supplies one — a throwaway container for
# the length of the run, or SCORSESE_TEST_DATABASE_URL when set. Its header has
# the reasoning; CI gets the same from a service container.
#
# The pinned chrome-headless-shell joins them for the same reason (#775): the
# page goldens capture real pages and fail without it. `tools/chromium/fetch`
# downloads and verifies the build `tools/chromium/pin` names.
test: ## [gate] The whole suite, golden renders and database included (needs ffmpeg, Chromium, nextest, docker)
	@command -v ffmpeg >/dev/null 2>&1 || { \
		echo "test: ffmpeg is not on PATH -- the render and golden tests need it." >&2; \
		echo "      Install it from your package manager, or point SCORSESE_FFMPEG at a binary." >&2; \
		exit 1; }
	@[ -x "$${SCORSESE_CHROME:-}" ] || command -v chrome-headless-shell >/dev/null 2>&1 || { \
		echo "test: no chrome-headless-shell -- the page goldens capture with it." >&2; \
		echo '      export SCORSESE_CHROME="$$(tools/chromium/fetch)"' >&2; \
		exit 1; }
	@$(NEXTEST_CHECK)
	tools/with-postgres cargo nextest run --workspace --locked
	cargo test --doc --workspace --locked
# Last, so it is the line still on screen when the suite goes green. nextest
# has already counted the skip; this is what makes it a sentence.
	@$(PIXEL_GATE_RUNS) || echo "test: $(PIXEL_GATE_SKIPPED)"

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
# **Where `--config` goes depends on the cargo-deny**, so the line asks it.
# 0.19 takes it only after `check`; 0.20 takes it only before. Each refuses the
# other's spelling outright, and both have been on a machine running this gate:
# #630 moved it after `check` for a cloud container's 0.19, which broke the Mac's
# 0.20 (#635). Leaving the flag out is no answer either: 0.20 then looks for an
# `app/deny.toml`, finds none, and passes on cargo-deny's permissive defaults —
# measured, with a policy that refuses MIT going green — which is the false
# green this gate exists to refuse. `check --help` naming the flag is the test.
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
	@if cargo deny check --help 2>/dev/null | grep -q -- '--config <'; then \
		set -x; cargo deny --all-features --manifest-path app/Cargo.toml \
			check --config deny.toml advisories bans sources licenses; \
	else \
		set -x; cargo deny --all-features --manifest-path app/Cargo.toml \
			--config deny.toml check advisories bans sources licenses; \
	fi

# The deploy (#532) is YAML and Dockerfiles that nothing else here reads, so
# without this a typo in deploy/compose.yaml is found by the maintainer at
# `docker compose up`, on the machine customers are served from. Three checks,
# all about correctness and all under a second, which is why this is a gate
# and runs on every branch rather than only on ones touching deploy/:
#
# - Every variable compose.yaml reads is documented in deploy/.env.example.
#   The example is the only list of what `.env` must hold, and a variable
#   added to one and not the other is a deploy that refuses to start with a
#   message the runbook never mentioned.
# - Compose parses the file, with the example's blanks filled by placeholders:
#   the file is valid, and every variable it requires is one the example has.
#   Once per tunnel mode (#568) — none, quick, named — with the two settings
#   whose blank means something left blank outside the mode that reads them,
#   and each mode must start its own tunnel and no other: a misspelt profile
#   is otherwise a tunnel that silently never starts.
# - Every folder under SCORSESE_DATA that compose.yaml bind-mounts is one the
#   runbook's first-time setup makes (docs/web.md, its `mkdir -p` line). A
#   bind's source must exist when Compose *creates* the container, before
#   anything has started, so a folder the server makes is not there yet on a
#   new host and the whole `up` fails partway (#861). Such a folder is mounted
#   as a volume instead, which is resolved at start: `capture-spool`.
#
# It does not build the images. That is a Rust release build and a Bun build
# inside Docker, minutes and gigabytes, to re-prove what `test` and `web`
# already prove; the runbook's update step is where an image build happens.
DEPLOY_VARS = grep -o '[$$]{[A-Z_][A-Z0-9_]*' deploy/compose.yaml | cut -c3- | sort -u
DEPLOY_BINDS = grep -o 'source: [$$]{SCORSESE_DATA}/[^ ]*' deploy/compose.yaml | cut -d/ -f2- | sort -u
DEPLOY_MADE = grep -o 'mkdir -p /path/to/scorsese-data/.*' docs/web.md | grep -o 'scorsese-data/[^ ]*' | cut -d/ -f2-

deploy: ## [gate] deploy/compose.yaml parses, and .env.example documents what it reads
	@command -v docker >/dev/null 2>&1 || { \
		echo "deploy: docker is not installed -- the check is 'docker compose config'." >&2; \
		exit 1; }
	@missing=; for var in $$($(DEPLOY_VARS)); do \
		grep -q "^$$var=" deploy/.env.example || missing="$$missing $$var"; \
	done; \
	[ -z "$$missing" ] || { \
		echo "deploy: compose.yaml reads variables deploy/.env.example does not document:$$missing" >&2; \
		exit 1; }
	@made=$$($(DEPLOY_MADE)); missing=; for dir in $$($(DEPLOY_BINDS)); do \
		echo "$$made" | grep -qx "$$dir" || missing="$$missing $$dir"; \
	done; \
	[ -z "$$missing" ] || { \
		echo "deploy: compose.yaml binds folders under SCORSESE_DATA that docs/web.md's first-time setup does not make:$$missing" >&2; \
		echo "deploy: a bind's source must exist before anything starts; mount a folder the server makes as a volume, like capture-spool (#861)" >&2; \
		exit 1; }
	@env=$$(mktemp) && trap 'rm -f "$$env"' EXIT && \
		sed -E '/^(COMPOSE_PROFILES|CLOUDFLARE_TUNNEL_TOKEN)=/!s/=$$/=placeholder/' \
			deploy/.env.example > "$$env" && \
		for mode in none quick-tunnel named-tunnel; do \
			profile=$$mode; token=; \
			[ $$mode = none ] && profile=; \
			[ $$mode = named-tunnel ] && token=placeholder; \
			services=$$(COMPOSE_PROFILES=$$profile CLOUDFLARE_TUNNEL_TOKEN=$$token \
				docker compose --file deploy/compose.yaml --env-file "$$env" \
				config --services) || exit 1; \
			tunnels=$$(echo "$$services" | grep -- '-tunnel$$' | paste -sd ' '); \
			[ "$$tunnels" = "$$profile" ] || { \
				echo "deploy: tunnel mode '$$mode' starts [$$tunnels], not just its own" >&2; \
				exit 1; }; \
		done
	@echo "deploy: compose.yaml is valid in every tunnel mode, every variable it reads is documented, and every folder it binds is one the runbook makes"

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

# The React front-end (#531), gated the way `app` is and for the same reason:
# `web/` is its own project with its own toolchain, and a Rust-only branch must
# not need Bun installed to pass `make gates`. So it runs when the branch
# touches `web/` and says so when it does not; CI's `web` job runs either way.
#
# $(WEB) is `scoped` only when reached through `make gates`, exactly as $(APP).
web: ## [gate] The web front-end, when the branch touches web/
	@if [ "$(WEB)" != "scoped" ] || $(TOUCHES_WEB); then \
		$(MAKE) --no-print-directory web-gates; \
	else \
		echo "web: this branch changes nothing under web/ -- not run."; \
	fi

# The same commands CI's `web` job runs, in the same order. `--frozen-lockfile`
# is `--locked`'s counterpart: an install that would change `bun.lock` fails
# instead, so the packages checked are the packages committed. Biome is both
# the linter and the formatter, so `lint` is also the format check. `build` is
# on the list because it is what the deploy ships — a page that typechecks and
# does not bundle is not a page.
web-gates:
	@command -v bun >/dev/null 2>&1 || { \
		echo "web: bun is not installed -- the web front-end builds with it." >&2; \
		echo "     https://bun.com/docs/installation  (the version is pinned in web/package.json)" >&2; \
		exit 1; }
	cd web && bun install --frozen-lockfile
	cd web && bun run lint
	cd web && bun run typecheck
	cd web && bun test
	cd web && bun run build

##@ Merging — asked of GitHub, not of the code

# What merging left for a human: every unticked item under a human-check
# heading in the pull requests merged since SINCE, as Markdown to paste into
# a batch report. A human check is never a merge hold, so this is where they
# come back together (#821, the script's docstring has the rest).
.PHONY: checks
checks: ## Unticked human checks in PRs merged since then. make checks SINCE=2026-10-05T18:00:00Z
	@test -n "$(SINCE)" || { \
		echo "checks: since when? e.g. make checks SINCE=2026-10-05T18:00:00Z (UTC)" >&2; \
		exit 1; }
	@python3 .github/scripts/human-checks.py --since "$(SINCE)"

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

# The same question in a loop, with the rebase and the waiting done for you.
# Merging stays serialized — this merges one branch at a time, in the order
# given, and asks `mergeable` about every one of them before it does. What it
# removes is an agent sitting through twenty cold CI runs holding worktrees
# open; what it never does is resolve a conflict, build anything locally, or
# look at a mutation report. The script's docstring has the reasoning, the
# measurement behind it, and what it deliberately leaves for a human to clean
# up afterwards. WATCH=1 drops the list and takes every ready pull request
# labelled `queue` as it appears, for a bounded time (#690, `merge-watch.py`),
# printing one `watch: ` line per board change and outcome (#819); SINCE=<ISO
# time> limits the board to pull requests opened since then.
queue: ## Rebase, wait for CI, merge each in turn. PRS="486 488", or WATCH=1 [SINCE=time] for `queue`-labelled PRs
	@test -n "$(PRS)$(WATCH)" || { \
		echo "queue: which pull requests? e.g. make queue PRS=\"486 488\"," >&2; \
		echo "       or make queue WATCH=1 to take every ready PR labelled \`queue\`" >&2; \
		exit 1; }
	@python3 .github/scripts/merge-queue.py $(if $(WATCH),--watch) $(if $(SINCE),--since $(SINCE)) $(PRS)

##@ Signals — informational, never a merge gate

coverage: ## Which pub items no test reaches. A signal: no threshold, blocks nothing
	@command -v cargo-llvm-cov >/dev/null 2>&1 || { \
		echo "coverage: cargo-llvm-cov is not installed." >&2; \
		echo "          Install it with: cargo install --locked cargo-llvm-cov" >&2; \
		echo "          It also needs: rustup component add llvm-tools-preview" >&2; \
		exit 1; }
	@mkdir -p target
	tools/with-postgres cargo llvm-cov --workspace --locked --exclude-from-report scorsese-golden \
		--json --output-path target/coverage.json
	python3 .github/scripts/coverage-summary.py target/coverage.json

# Diff-scoped, as `make mutants-remote SCOPE=diff` runs it, because the useful question
# before a push is "would anything notice if what I just wrote were wrong?" and
# not "how is the whole codebase doing". `cargo mutants` on its own sweeps the
# full scoped surface — 3875 mutants — and is there when that is what you want.
#
# Surviving mutants exit 2 and timeouts exit 3; neither is a failure of the run,
# so neither fails this target. Anything else is the tool breaking and does —
# and 1 is the one worth a sentence, because it is what a copy that would not
# fit exits with. #392 is that failure with the reason stripped off it: "Disk
# quota exceeded", from a copy of a build directory the run had no use for,
# reads as *this machine is out of space* and gets answered by deleting things.
# So an unexpected status names the scratch copy and how much room it has.
# Exit 4 is excluded from that: the tests failing unmutated is an answer about
# the code, and pointing at a directory would be misdirection.
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
# rather than guarded here, which is what keeps this and an on-request run saying the
# same thing.
#
# Every path out of here writes $(MUTANTS_STAMP), including the two that run
# no mutants, because "it was run and there was nothing to do" is an answer and
# `make gates` has to be able to tell it from silence. A run that dies for a
# reason that is not a surviving mutant writes nothing, and is then reported as
# never having happened — which it effectively did not.
#
# Before any of that, the surface itself is checked -- see the `surface-floor:`
# line in `.cargo/mutants.toml`, and #363 for what it is checking against. A
# run over a surface that has been excluded down to nothing reports *nothing to
# report*, which is exactly what a healthy branch touching no mutated lines
# reports, so the silence is unreadable and everything after it is worthless.
# It costs `cargo mutants --list`, which builds nothing and answers in under a
# second, and it aborts the target rather than mutating on top of a broken
# instrument. It is deliberately not in $(GATES): it proves the instrument
# works rather than that the code is right, and `make gates` should not grow a
# dependency on a tool it otherwise never invokes.
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
	@cargo mutants --list > target/mutants-list.txt
	@python3 .github/scripts/mutation-surface.py target/mutants-list.txt
	@git diff origin/main...HEAD -- '*.rs' > target/pr.diff
	@if [ ! -s target/pr.diff ]; then \
		echo "mutants: this branch changes no Rust source against origin/main."; \
		said="this branch changed no Rust source"; \
	else \
		rm -rf mutants.out; \
		mkdir -p $(MUTANTS_TMPDIR); \
		scratch=$$(mktemp -d $(MUTANTS_TMPDIR)/run-XXXXXX); \
		trap 'rm -rf "$$scratch"' EXIT INT TERM; \
		TMPDIR=$$scratch cargo mutants --in-diff target/pr.diff \
			--jobs $(MUTANTS_JOBS); status=$$?; \
		case $$status in \
			0|2|3) ;; \
			4) exit $$status ;; \
			*) echo "mutants: cargo-mutants exited $$status without reporting on a single mutant." >&2; \
			   echo "         It builds each mutant in a copy of this worktree under" >&2; \
			   echo "         $(MUTANTS_TMPDIR) ($$(df -Ph $(MUTANTS_TMPDIR) 2>/dev/null | awk 'NR==2 {print $$4}') free)," >&2; \
			   echo "         so a message about space or a quota is about that copy, not about the repo." >&2; \
			   exit $$status ;; \
		esac; \
		python3 .github/scripts/mutants-summary.py mutants.out/outcomes.json; \
		said=$$($(MUTANTS_VERDICT)); \
	fi; \
	printf '%s\n%s\n' "$$(git rev-parse --abbrev-ref HEAD)" "$$said" > $(MUTANTS_STAMP)

# The same question as `mutants`, asked of GitHub's runners instead of this
# machine (#651): mutation compiles every mutant from scratch for as long as
# the scope takes, and here that competes with sibling worktrees and with the
# web app's users. It dispatches `.github/workflows/mutants-on-request.yml` on
# this branch — pushed, because the runner mutates what GitHub has — waits,
# and prints the report and the survivors' diffs. SCOPE is `diff` (this branch
# against origin/main), a crate (`scorsese-core`), or path globs
# (`'crates/zimmer/src/song/**'`), and the run covers exactly that: the
# workflow's header says why paths go through `--in-diff`, and the report says
# if anything outside the scope came along anyway.
#
# Exit 0 is a report, survivors or not; 1 is no report — a red, cancelled or
# missing run, said as such and never as zero survivors; 3 is GitHub
# unreachable. It writes no $(MUTANTS_STAMP): that line is about this
# checkout, and a remote run's answer is in the terminal and the artifact.
mutants-remote: ## Mutation on GitHub's runners, scoped. SCOPE=diff | scorsese-core | 'path/glob/**'
	@test -n "$(SCOPE)" || { \
		echo "mutants-remote: what should be mutated? SCOPE=diff, a crate, or path globs" >&2; \
		echo "                e.g. make mutants-remote SCOPE='crates/core/src/keyframe/**'" >&2; \
		exit 1; }
	@python3 .github/scripts/mutants-remote.py '$(SCOPE)'

# The line `make gates` ends on, and the reason it is reached from the recipe
# rather than sitting in $(GATES): it runs nothing, it can fail nothing, and a
# signal that could redden the gate summary would have stopped being a signal.
# It costs two `git diff --name-only` calls and a stat, so `make gates` is not
# measurably slower for having it.
#
# Three answers, and the middle one is why mtimes are consulted at all. A stamp
# from before the last edit describes code that no longer exists, and reporting
# what it said would be the false green in slow motion — so it is reported as
# stale, naming the file that outdates it.
#
# "Changed Rust" is asked in three parts because `make mutants` scopes itself
# to the committed diff and `make gates` is most often run with an edit still
# in the working tree: an uncommitted or untracked `.rs` is code the signal
# demonstrably has not seen, whatever the stamp says. Nothing is printed at all
# when the branch changes no Rust — there is no signal to be missing, and a
# documentation branch does not need telling.
mutants-status:
	@changed=$$( { git diff --name-only origin/main...HEAD -- '*.rs' 2>/dev/null; \
	               git diff --name-only HEAD -- '*.rs' 2>/dev/null; \
	               git ls-files --others --exclude-standard -- '*.rs'; } | sort -u); \
	[ -n "$$changed" ] || exit 0; \
	if [ ! -f $(MUTANTS_STAMP) ] || \
	   [ "$$(head -n 1 $(MUTANTS_STAMP))" != "$$(git rev-parse --abbrev-ref HEAD)" ]; then \
		echo "gates: mutation signal not run -- this branch changes Rust and 'make mutants' has not run here; it blocks nothing, and 'make mutants-remote SCOPE=diff' asks GitHub's runners instead. See docs/mutation-testing.md."; \
		exit 0; \
	fi; \
	for file in $$changed; do \
		[ -e "$$file" ] && [ "$$file" -nt $(MUTANTS_STAMP) ] || continue; \
		echo "gates: mutation signal is stale -- $$file changed after the last 'make mutants'."; \
		exit 0; \
	done; \
	echo "gates: mutation signal -- $$(tail -n 1 $(MUTANTS_STAMP))."

##@ Spending — real calls to the vendors, run by a person, never by CI

# The live provider check (#567, docs/live-check.md): the smallest real call to
# each vendor, through the product's own clients, reporting whether each reply
# is still the shape scorsese reads. It spends money — a few cents, $0.20 more
# with --include-veo — so it is neither a gate nor a signal CI runs: it quotes,
# asks, and is held to budget_cents like any spend. Keys resolve the one way
# they always do (docs/credentials.md); a vendor with no key is skipped.
live-check: ## Real calls to every provider, quoted first. ARGS="--include-veo --record DIR --yes"
	cargo run --locked --quiet -p scorsese-cli --bin scorsese -- check-providers $(ARGS)

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
mcp-table: ## Rewrite docs/mcp.md's tool table and docs/web.md's served-tool lists from the code
	UPDATE_MCP_TABLE=1 cargo test --locked -p scorsese-mcp --test table
	UPDATE_WEB_TOOLS=1 cargo test --locked -p scorsese-server --lib tools::surface::page
	@echo "mcp-table: docs/mcp.md and docs/web.md now say what the code says."

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
