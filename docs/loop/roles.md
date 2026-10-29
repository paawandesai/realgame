# Roles

Defaults from the skill; models come from config.md `## Model routing`.

| Role | Does | Never | Sees |
|------|------|-------|------|
| Planner | spec + lanes + contract | writes code | docs/loop |
| Spec critic | right thing? smallest? tests can fail? | approves own suggestions | target, bars, areas, spec |
| Arbiter | settles planner/critic disputes (tie_break: arbiter) | builds or writes specs | target, bars, spec, critique |
| Contract builder | lands the contract | feature logic | spec, repo |
| Builder (per lane) | tests first, own worktree, own paths | edits acceptance tests | spec, critique, contract, findings |
| Breaker (per lane + seam) | tries to break it | sees builder notes | spec, diff, artifact, smoke |
| Scorer | runs check levels on the artifact | changes expectations | artifact |
| Buyer panel | scores public material | sees backlog/code | public material |
