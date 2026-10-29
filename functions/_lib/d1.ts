// The slice of Cloudflare's D1 API these Functions use. Declared here rather
// than pulling in @cloudflare/workers-types, so the repo stays dependency-light
// and the unit tests can supply a node:sqlite stand-in (test/d1-sqlite.ts).

export interface D1Result {
  results: Record<string, unknown>[];
  meta: { changes: number };
}

export interface D1PreparedStatement {
  bind(...values: unknown[]): D1PreparedStatement;
  all(): Promise<D1Result>;
  run(): Promise<D1Result>;
}

export interface D1Database {
  prepare(sql: string): D1PreparedStatement;
  batch(statements: D1PreparedStatement[]): Promise<D1Result[]>;
}

/** The part of a Pages Functions context these handlers read. */
export interface PagesContext<E> {
  request: Request;
  env: E;
}
