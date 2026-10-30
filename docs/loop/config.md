# Config

streak_nights: 3
plateau_nights: 4
plateau_min_change: 0.05
max_rounds: 3
session_minutes: 55
step_timeout_minutes: 25
merge_policy: auto            # Paawan 2026-10-29: merge to main whenever CI passes (PR, CI green, merge)
paused: false
staging_deploy: none          # production deploy is human-only; no staging yet
tie_break: arbiter
after_done: harden
continuous_waves: true
retry_parked_after_waves: 10
stall_waves: 3
notify: thread reply in the project thread on each merge, ACTION or bar change
max_parallel_items: 2
# 2026-10-30 Paawan: "parallelize as much as you can". Build slots stay 2: the 4-CPU/16 GB cloud box crashes Chromium suites with a third concurrent build + golden. Planning runs wide (design_ahead 7) so a ready spec is always waiting.
max_lanes: 3
design_ahead: 7
max_concurrent_agents: 8
artifact_dir: .loop-artifacts
nightly_budget_tokens: 6000000   # confirm with Paawan
weekly_budget_tokens: 30000000   # confirm with Paawan

## Runtime
- Claude Code cloud session (the project thread), with an hourly `send_later` heartbeat that starts each cycle.
- Waves run as Workflow scripts or background agents in git worktrees.
- Real-GPU checks run via the Remote Control session on Paawan's Mac (MacBook Pro M5 Pro, Chrome 155).
  - Clone: ~/Documents/realgame
  - Spike worktree: ~/realgame-bevy-spike
  - Target dir: ~/bevy-target
  - Bench harness: ~/bevy-bench
  - Opening browser windows there is allowed (Paawan, 22:11 UTC).

## Model routing
planner: opus   critic: sonnet   arbiter: opus (fresh)   builder: opus   breaker: sonnet   panel: haiku (6yo, 9yo), sonnet (13yo, teacher), opus (QA)

## Commands
build: `bash crates/bevy-client/build-web.sh`   # output: crates/bevy-client/web/pkg-out
smoke: `QUERY=?low node tests/e2e/golden.mjs`   # after build; SHOTS=<dir> saves screenshots
test: `cargo fmt --all --check && cargo clippy --all-targets --locked -- -D warnings && cargo test --locked && npm run typecheck && npm test`
lint: `bash .husky/pre-push`
perf: todo
analytics-e2e: todo
