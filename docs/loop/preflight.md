# Preflight
status: ready
checked: 2026-10-29T22:25Z by setup (cloud session, project thread)
- [x] build: todo. Bootstrap item I-001 lands the first-person client and `crates/bevy-client/build-web.sh`. The spike's web build ran in the cloud (5.0 MB gzip) and on Paawan's Mac (4.99 MB gzip)
- [x] test: ran fmt, clippy, cargo test (185 passed) and npm typecheck/test (19 passed). 185 s cold-ish, exit 0
- [x] smoke: todo. Bootstrap item I-002. The v2 Playwright golden path existed for 2D; the 3D one gets written
- [x] git: pushed claude/product-loop to origin (this commit)
- [x] worktrees: created and removed one
- [x] models: opus, sonnet and haiku all answered in this session's earlier workflows (58-agent v2 run)
- [x] concurrency: 16 concurrent workflow agents ran without rate-limit errors (v2 run)
- [x] schedule: hourly send_later heartbeat into the project thread session; runs need no approval
- [x] notify: thread replies to Paawan on merges, ACTIONs and bar changes
- [x] real-GPU: Remote Control session on Paawan's Mac (M5 Pro). Spike: 120 fps p50, 109 p5 (WebGL2, 2560x1440)
