import { DatabaseSync } from "node:sqlite";

import { SCHEMA_STATEMENTS } from "./schema.ts";

export type SqlValue = string | number | null;

export interface SqlRow {
  [column: string]: SqlValue | undefined;
}

/** Small sync SQL surface shared by Durable Object storage and the Node test driver. */
export interface SqlRunner {
  all(sql: string, params?: SqlValue[]): SqlRow[];
  run(sql: string, params?: SqlValue[]): void;
  transaction<T>(body: () => T): T;
}

export function initializeSchema(sql: SqlRunner): void {
  for (const statement of SCHEMA_STATEMENTS) {
    sql.run(statement);
  }
}

export function nodeSqliteRunner(database: DatabaseSync): SqlRunner {
  return {
    all(sql, params = []) {
      return database.prepare(sql).all(...params) as SqlRow[];
    },
    run(sql, params = []) {
      database.prepare(sql).run(...params);
    },
    transaction(body) {
      database.exec("BEGIN");
      try {
        const value = body();
        database.exec("COMMIT");
        return value;
      } catch (error) {
        database.exec("ROLLBACK");
        throw error;
      }
    },
  };
}

/**
 * Adapter for Durable Object SQLite. `sql.exec` runs one statement and rejects
 * `BEGIN` / `COMMIT`, so multi-statement work goes through `transactionSync`.
 */
export function durableObjectRunner(storage: {
  sql: {
    exec(query: string, ...bindings: unknown[]): { toArray(): SqlRow[] };
  };
  transactionSync<T>(callback: () => T): T;
}): SqlRunner {
  return {
    all(sql, params = []) {
      return storage.sql.exec(sql, ...params).toArray();
    },
    run(sql, params = []) {
      storage.sql.exec(sql, ...params).toArray();
    },
    transaction(body) {
      return storage.transactionSync(body);
    },
  };
}
