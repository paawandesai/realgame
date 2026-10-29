// GET /api/stats: aggregates for the dashboard, behind DASHBOARD_TOKEN.
import { handleStats, type Env } from "../_lib/analytics.ts";
import type { PagesContext } from "../_lib/d1.ts";

export const onRequest = (ctx: PagesContext<Env>): Promise<Response> => handleStats(ctx.request, ctx.env);
