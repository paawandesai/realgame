# Deploying to Cloudflare

The game is a static site on Cloudflare Pages, with two small Pages Functions (`/api/event`, `/api/stats`) and a D1 database for anonymous play counts (see `docs/ANALYTICS.md`). Everything fits in Cloudflare's free plan.

There are two ways to deploy. **Pick one and stay with it**: a Pages project made by uploading from your computer (option A) cannot later be switched to building from GitHub (option B), and the other way round.

- **Option A, from your computer (recommended to start).** One command builds and uploads. You decide when the site changes.
- **Option B, Cloudflare builds from GitHub.** Every push to `main` deploys by itself.

## Before you start (both options)

1. A free Cloudflare account: <https://dash.cloudflare.com/sign-up>.
2. On your computer: Node 22, Rust, the WebAssembly target and `wasm-pack`, as in the README's "Run it locally":

   ```sh
   rustup target add wasm32-unknown-unknown
   cargo install wasm-pack --locked
   ```

3. In a terminal, in the repository folder:

   ```sh
   npm ci              # installs the pinned wrangler (Cloudflare's CLI) and the git hooks
   npx wrangler login  # opens the browser; click "Allow"
   ```

## Option A: deploy from your computer

```sh
npm run deploy
```

That runs `scripts/deploy.sh`, which is safe to run as often as you like. Each time, it:

1. checks you are logged in to Cloudflare;
2. creates the D1 database `realgame-analytics` if it does not exist yet, and writes its id into `wrangler.toml`;
3. applies any new database migrations from `migrations/`;
4. creates the Pages project `realgame` if it does not exist yet;
5. creates a random `DASHBOARD_TOKEN` secret if the project has none, and **prints it once**;
6. builds the game and the dashboard into `web/dist` (`scripts/build-site.sh`);
7. uploads it as the production deployment.

After the first run:

1. **Save the printed token** in your password manager. You need it to open the dashboard.
2. **Commit the database id** it wrote into `wrangler.toml` (it is not a secret):

   ```sh
   git add wrangler.toml
   git commit -m "Record the D1 database id"
   ```

3. Open the game at the URL wrangler printed (usually `https://realgame.pages.dev`; Cloudflare adds a suffix if that name is taken), and the dashboard at the same address plus `/dashboard/`. Paste the token once; the tab remembers it until you close it.

To deploy a new version later, run `npm run deploy` again.

Optional settings: `PAGES_PROJECT=myname npm run deploy` uses another project name, and `DEPLOY_BRANCH=preview npm run deploy` makes a preview deployment at its own URL instead of replacing the live site.

## Option B: let Cloudflare build from GitHub

1. In the Cloudflare dashboard: **Workers & Pages → Create → Pages → Connect to Git**, and pick `paawandesai/realgame`.
2. Fill in the build settings:

   | Setting | Value |
   | --- | --- |
   | Project name | `realgame` (if you choose another, change `name` in `wrangler.toml` to match) |
   | Production branch | `main` |
   | Framework preset | None |
   | Build command | `bash scripts/cf-pages-build.sh` |
   | Build output directory | `web/dist` |
   | Root directory | leave empty |
   | Environment variable | `NODE_VERSION` = `22` |

   Cloudflare's build machines have Node but not Rust, so `scripts/cf-pages-build.sh` installs Rust and `wasm-pack` 0.15.0 before building. Compiling `wasm-pack` makes the first build take a few minutes.
3. Click **Save and Deploy**. The first build will serve the game, but the analytics will not work yet, which is fine: the game ignores it.
4. On your computer, create the database and record its id:

   ```sh
   npm run db:setup
   git add wrangler.toml
   git commit -m "Record the D1 database id"
   git push
   ```

   Cloudflare reads the database binding from `wrangler.toml`, so this push redeploys with analytics connected.
5. Make the dashboard token and store it as a secret (copy the first command's output):

   ```sh
   node -e "console.log(require('crypto').randomBytes(32).toString('hex'))"
   npx wrangler pages secret put DASHBOARD_TOKEN --project-name realgame
   ```

   Or in the dashboard: the project → **Settings → Variables and Secrets → Add**, type **Secret**, name `DASHBOARD_TOKEN`. Secrets take effect on the next deployment: **Deployments → … → Retry deployment**.
6. Whenever a pull request adds a file to `migrations/`, run `npm run db:setup` again after merging. Cloudflare's builds do not run migrations.

## Protecting the dashboard more strongly: Cloudflare Access

The token is enough to keep the counts private. Cloudflare Access is stronger: Cloudflare checks who you are (for example with a one-time code sent to your email) before the request reaches the page or the stats Function, and it logs who signed in. It is free for up to 50 users.

1. Cloudflare dashboard → **Zero Trust** (pick the free plan the first time) → **Access → Applications → Add an application → Self-hosted**.
2. Application domain: your Pages domain (for example `realgame.pages.dev`), path `dashboard`. Add a second destination with the same domain and path `api/stats`.
3. Add a policy: Action **Allow**, Include **Emails**, and your email address.
4. Save. Opening `/dashboard/` now asks for your email first. Keep `DASHBOARD_TOKEN` set as a second layer.

## Running it locally

```sh
npm run build:site                   # game + dashboard into web/dist
echo 'DASHBOARD_TOKEN="local-dev-token-0123456789abcdef0123456789"' > .dev.vars
npm run dev:pages                    # local D1 + Functions at http://127.0.0.1:8788
```

`.dev.vars` and `.wrangler/` (the local database) are git-ignored. `npm --prefix web run dev` (Vite) still works for game development; it has no `/api/event`, and the game does not mind.

## Useful commands

| Command | What it does |
| --- | --- |
| `npm run deploy` | Everything above, for option A |
| `npm run db:setup` | Only create/migrate the database and record its id |
| `npm run analytics:purge` | Delete events older than 90 days now (this also happens automatically) |
| `npx wrangler pages secret put DASHBOARD_TOKEN --project-name realgame` | Replace the dashboard token, then deploy again |
| `npx wrangler d1 execute realgame-analytics --remote --command "SELECT day, COUNT(DISTINCT daily_id) FROM events GROUP BY day"` | Look at the raw counts directly |

## Free plan limits

| Resource | Free allowance | What uses it |
| --- | --- | --- |
| Pages static requests | unlimited | the game and the dashboard page |
| Pages Functions requests | 100,000 a day (shared with Workers) | each event and each dashboard load is one; about 8 per play-through |
| D1 rows written | 100,000 a day | each event writes one row plus its index entry |
| D1 rows read | 5 million a day | the daily cap check, and each dashboard load scans the last 30 days |
| D1 storage | 5 GB | 90 days of events is far below this |

If a limit is reached, the Functions answer with errors until the next UTC day. Only `/api/*` runs Functions, so the game keeps loading and playing normally, and the analytics module ignores the errors.
