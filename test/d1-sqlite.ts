// A D1 stand-in over node:sqlite (built into Node 22), so the tests run the
// real migration and the real SQL from functions/_lib/analytics.ts.
import { readFileSync, readdirSync } from "node:fs";
import { DatabaseSync, type SQLInputValue } from "node:sqlite";
import type { D1Database, D1PreparedStatement, D1Result } from "../functions/_lib/d1.ts";

class Statement implements D1PreparedStatement {
  private readonly db: DatabaseSync;
  private readonly sql: string;
  private readonly params: SQLInputValue[];
  constructor(db: DatabaseSync, sql: string, params: SQLInputValue[] = []) {
    this.db = db;
    this.sql = sql;
    this.params = params;
  }
  bind(...values: unknown[]): D1PreparedStatement {
    for (const v of values) {
      // D1 rejects undefined; so does this stand-in.
      if (v === undefined) throw new TypeError("D1_TYPE_ERROR: undefined is not a supported type");
    }
    return new Statement(this.db, this.sql, values as SQLInputValue[]);
  }
  exec(): D1Result {
    const stmt = this.db.prepare(this.sql);
    if (stmt.columns().length > 0) {
      return { results: stmt.all(...this.params) as Record<string, unknown>[], meta: { changes: 0 } };
    }
    const r = stmt.run(...this.params);
    return { results: [], meta: { changes: Number(r.changes) } };
  }
  async all(): Promise<D1Result> {
    return this.exec();
  }
  async run(): Promise<D1Result> {
    return this.exec();
  }
}

export function memoryD1(): D1Database & { raw: DatabaseSync } {
  const raw = new DatabaseSync(":memory:");
  const dir = new URL("../migrations/", import.meta.url);
  for (const f of readdirSync(dir).filter((n) => n.endsWith(".sql")).sort()) {
    raw.exec(readFileSync(new URL(f, dir), "utf8"));
  }
  return {
    raw,
    prepare: (sql) => new Statement(raw, sql),
    async batch(statements) {
      // D1 runs a batch as one transaction.
      raw.exec("BEGIN");
      try {
        const out = statements.map((s) => (s as Statement).exec());
        raw.exec("COMMIT");
        return out;
      } catch (err) {
        raw.exec("ROLLBACK");
        throw err;
      }
    },
  };
}
