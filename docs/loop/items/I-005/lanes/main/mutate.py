#!/usr/bin/env python3
# Acceptance test 4 mutations: apply one at a time, run analytics-e2e, restore.
import os, shutil, subprocess, sys

repo = sys.argv[1]
only = sys.argv[2:] or None
out_dir = os.path.dirname(os.path.abspath(__file__))
M = {
    'a': ('web/dist/analytics.js',
          "for (const quest of questEvents(prev, state, questsSent)) send({ type: 'quest_complete', band, quest });",
          "questEvents(prev, state, questsSent); // MUTATION a: quest_complete send removed"),
    'b': ('web/dist/analytics.js',
          "  if (!nav) return true;\n  return nav.doNotTrack",
          "  return false; // MUTATION b\n  return nav.doNotTrack"),
    'c': ('web/dist/age.js',
          "start(b.dataset.band ?? '');",
          "start(b.dataset.band === 'seedling' ? 'sprout' : (b.dataset.band ?? '')); // MUTATION c"),
    'd': ('web/dist/analytics.js',
          "send({ type: 'session_end', band, dur: bucket });",
          "// MUTATION d: session_end removed"),
    'e': ('web/dist/analytics.js',
          "    const body = eventBody(event, dailyId(storage(), new Date()));\n    if (body === null) return;",
          "    let body = eventBody(event, dailyId(storage(), new Date()));\n    if (body === null) return;\n    body = JSON.stringify({ ...JSON.parse(body), ua: navigator.userAgent }); // MUTATION e"),
    'f': ('functions/_lib/analytics.ts',
          "funnel: { started, quests, rooms, trialComplete },",
          "funnel: { started, rooms, trialComplete }, // MUTATION f"),
    'g': ('functions/_lib/analytics.ts',
          ".bind(day, e.id, e.type, e.band, e.room, e.dur, DAILY_CAP),",
          ".bind(day, e.id, e.type, e.band, e.type === \"quest_complete\" ? 0 : e.room, e.dur, DAILY_CAP), // MUTATION g"),
}
env = dict(os.environ, SKIP_BUILD='1', QUERY='?low')
for key, (rel, old, new) in M.items():
    if only and key not in only:
        continue
    path = os.path.join(repo, rel)
    backup = path + '.orig'
    shutil.copyfile(path, backup)
    try:
        src = open(path).read()
        assert src.count(old) == 1, f'{key}: pattern not found once in {rel}'
        open(path, 'w').write(src.replace(old, new))
        r = subprocess.run(['node', 'tests/e2e/analytics-e2e.mjs'], cwd=repo, env=env, capture_output=True, text=True, timeout=1500)
        log = r.stdout + r.stderr
        open(os.path.join(out_dir, f'mut-{key}.log'), 'w').write(log)
        fails = [l[:160] for l in log.splitlines() if l.startswith('FAIL ')]
        summary = [l for l in log.splitlines() if 'checks passed' in l]
        print(f'MUTATION {key}: exit {r.returncode} {summary}')
        for f in fails:
            print(f'   {f}')
        sys.stdout.flush()
    finally:
        shutil.move(backup, path)
