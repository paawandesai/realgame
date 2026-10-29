// POST /api/event: one anonymous gameplay event. See docs/ANALYTICS.md.
import { handleEvent, type Env } from "../_lib/analytics.ts";
import type { PagesContext } from "../_lib/d1.ts";

export const onRequest = (ctx: PagesContext<Env>): Promise<Response> => handleEvent(ctx.request, ctx.env);
