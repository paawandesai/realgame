#!/usr/bin/env bash
# One-command deploy to Cloudflare Pages. Safe to run again and again: every
# step checks what already exists first. See docs/DEPLOY.md.
#
#   npm run deploy            create what is missing, migrate, build, deploy
#   npm run db:setup          only the database steps (for the GitHub-connected setup)
#   deploy.sh --prebuilt      deploy web/dist as it is (the GitHub Actions
#                             workflow builds it in an earlier job)
#
# Environment overrides: PAGES_PROJECT (default realgame), DEPLOY_BRANCH
# (default main, the production branch; anything else makes a preview and
# skips the database migrations, so unmerged migrations never touch the real
# database). Logs in with CLOUDFLARE_API_TOKEN and CLOUDFLARE_ACCOUNT_ID when
# they are set (CI), otherwise with `npx wrangler login`.
set -euo pipefail
cd "$(dirname "$0")/.."

PROJECT="${PAGES_PROJECT:-realgame}"
DB_NAME="realgame-analytics"
BRANCH="${DEPLOY_BRANCH:-main}"
DB_ONLY=0
PREBUILT=0
case "${1:-}" in
  --db-only) DB_ONLY=1 ;;
  --prebuilt) PREBUILT=1 ;;
esac
IN_CI=0
[ "${GITHUB_ACTIONS:-}" = true ] && IN_CI=1

say() { printf '\n==> %s\n' "$*"; }
wrangler() { npx --no-install wrangler "$@"; }

[ -x node_modules/.bin/wrangler ] || { say "Installing repo tooling (wrangler)"; npm ci --no-audit --no-fund; }

say "Checking the Cloudflare login"
if wrangler whoami 2>&1 | grep -qi "not authenticated"; then
  echo "Not logged in to Cloudflare. Run:  npx wrangler login   then run this again." >&2
  exit 1
fi

say "D1 database '$DB_NAME'"
find_db_id() {
  wrangler d1 list --json 2>/dev/null | node -e '
    let s = ""; process.stdin.on("data", (d) => (s += d)).on("end", () => {
      const list = JSON.parse(s.slice(Math.max(0, s.search(/^\s*\[\s*(\{|\]|$)/m))));
      const db = list.find((d) => d.name === process.argv[1]);
      if (db) process.stdout.write(db.uuid);
    });' "$DB_NAME"
}
DB_ID="$(find_db_id)"
if [ -z "$DB_ID" ]; then
  echo "Creating it."
  wrangler d1 create "$DB_NAME" >/dev/null
  DB_ID="$(find_db_id)"
fi
[ -n "$DB_ID" ] || { echo "Could not find or create the D1 database." >&2; exit 1; }
echo "id: $DB_ID"

# wrangler.toml must carry the real id for remote commands and for deploys.
node -e '
  const fs = require("fs");
  const s = fs.readFileSync("wrangler.toml", "utf8");
  const t = s.replace(/^database_id = ".*"$/m, `database_id = "${process.argv[1]}"`);
  if (t !== s) { fs.writeFileSync("wrangler.toml", t); console.log("Wrote the database id into wrangler.toml: commit that change."); }
' "$DB_ID"

if [ "$BRANCH" = main ] || [ "$DB_ONLY" = 1 ]; then
  say "Applying database migrations (remote)"
  wrangler d1 migrations apply "$DB_NAME" --remote
else
  say "Preview deploy: skipping database migrations"
fi

[ "$DB_ONLY" = 1 ] && { say "Database ready."; exit 0; }

say "Pages project '$PROJECT'"
project_exists() {
  wrangler pages project list --json 2>/dev/null | node -e '
    let s = ""; process.stdin.on("data", (d) => (s += d)).on("end", () => {
      const list = JSON.parse(s.slice(Math.max(0, s.search(/^\s*\[\s*(\{|\]|$)/m))));
      process.exit(list.some((p) => Object.values(p).includes(process.argv[1])) ? 0 : 1);
    });' "$PROJECT"
}
if project_exists; then
  echo "exists."
else
  wrangler pages project create "$PROJECT" --production-branch main
fi

say "Dashboard token"
if wrangler pages secret list --project-name "$PROJECT" 2>/dev/null | grep -q "DASHBOARD_TOKEN"; then
  echo "DASHBOARD_TOKEN is already set (unchanged)."
elif [ "$IN_CI" = 1 ]; then
  # Never print a secret into a CI log (the repository is public). Use the
  # DASHBOARD_TOKEN repository secret when there is one.
  if [ -n "${DASHBOARD_TOKEN:-}" ]; then
    printf '%s' "$DASHBOARD_TOKEN" | wrangler pages secret put DASHBOARD_TOKEN --project-name "$PROJECT" >/dev/null
    echo "Set DASHBOARD_TOKEN from the repository secret."
  else
    echo "::warning::DASHBOARD_TOKEN is not set, so /dashboard/ will refuse everyone. Add it as a repository secret (or set it in the Cloudflare dashboard) and deploy again."
  fi
else
  TOKEN="$(node -e 'process.stdout.write(require("crypto").randomBytes(32).toString("hex"))')"
  printf '%s' "$TOKEN" | wrangler pages secret put DASHBOARD_TOKEN --project-name "$PROJECT" >/dev/null
  echo "Created a new DASHBOARD_TOKEN. Save it in your password manager; it is shown only once:"
  echo
  echo "    $TOKEN"
fi

if [ "$PREBUILT" = 1 ]; then
  [ -f web/dist/index.html ] || { echo "--prebuilt: web/dist/index.html is missing." >&2; exit 1; }
  say "Using the prebuilt web/dist"
else
  say "Building the game and the dashboard"
  bash scripts/build-site.sh
fi

say "Deploying to Cloudflare Pages (branch: $BRANCH)"
wrangler pages deploy --project-name "$PROJECT" --branch "$BRANCH" --commit-dirty=true

say "Done. The game is at the URL printed above (usually https://$PROJECT.pages.dev); the dashboard is that address plus /dashboard/"
