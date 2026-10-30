# I-003 spec critique

**Verdict: revise**

Right thing: yes. B2 and B3 are unmeasurable today, and the loop cannot reach "3 nightly passes" without a scorer. It measures from outside the client, touches only tests-e2e and one doc, and respects the first-person 3D hard rule (the workload drives the real FPS controls). Teachers and kids on school Chromebooks are served by the 4x throttle and 15 MB bars. Tests can fail: threshold overrides, the throttle-ratio check, the missing-wasm run and the no-cast run all have a failing path. It is large for one lane, but the pieces share one script, so no split is needed.

## Changes (applied to spec.md, now Revision 2)

1. Test 2 mixed `ONLY=low@1x` into the argument list. Made it explicitly an env var.
2. The verdict was undefined when `ONLY` omits a configuration a bar needs. Added: such bars are `{pass:null, skipped:true}`, the run reports `partial: true` and `pass: false`, and only a full run can exit 0.
3. The "raise thresholds, exit 0" check must be a full run, otherwise it contradicts change 2. Stated so, plus a check that a partial run with raised thresholds still exits 1.
4. Threshold parsing: numbers like `1e9` are explicitly valid.

## Residual notes (no change)

- Brotli q11 is stricter than Cloudflare's edge compression, so B3 size is conservative. Fine.
- Test 7 (within 30%) may be flaky on a shared box; treat a failure as a rerun-once, not a park.
- The Mac recipe cannot be verified here; "not run" is allowed and does not block.
