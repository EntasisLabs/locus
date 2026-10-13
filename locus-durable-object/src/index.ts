import { DurableObject } from "cloudflare:workers";

import { durableObjectRunner } from "./sql.ts";
import { LocusStoreEngine, type SttpNode, type SyncCheckpoint } from "./store.ts";

export interface Env {
  LOCUS_STORE: DurableObjectNamespace;
}

interface SqlStorage {
  exec(query: string, ...bindings: unknown[]): { toArray(): Array<Record<string, string | number | null>> };
}

interface DurableObjectState {
  storage: {
    sql: SqlStorage;
    transactionSync<T>(callback: () => T): T;
  };
  blockConcurrencyWhile(work: () => Promise<unknown>): Promise<void>;
}

const METHODS = [
  "upsertNode",
  "queryNodes",
  "getByResonance",
  "getByResonanceGlobal",
  "getByHybrid",
  "getByHybridGlobal",
  "listNodes",
  "getLastAvec",
  "getTriggerHistory",
  "storeCalibration",
  "queryChangesSince",
  "getCheckpoint",
  "putCheckpoint",
  "deleteNodes",
  "purgeSession",
  "batchRekeyScopes",
  "syncNodeTags",
  "deleteNodeTags",
  "findSyncKeysByTags",
  "findTags",
  "queryTagRecords",
] as const;

type StoreMethod = (typeof METHODS)[number];

/**
 * One Durable Object is one Locus database. Address it with `getByName(storeName)`.
 * Session sharding can sit in front of this later; global recall and rekey need the
 * whole database, so they stay inside the object.
 */
export class LocusStore extends DurableObject<Env> {
  private engine!: LocusStoreEngine;

  constructor(ctx: DurableObjectState, env: Env) {
    super(ctx, env);
    const open = () => new LocusStoreEngine(durableObjectRunner(ctx.storage));
    let opened = false;
    ctx.blockConcurrencyWhile(async () => {
      this.engine = open();
      opened = true;
    });
    if (!opened) {
      this.engine = open();
    }
  }

  upsertNode(node: SttpNode) {
    return this.engine.upsertNode(node);
  }

  queryNodes(query: Parameters<LocusStoreEngine["queryNodes"]>[0]) {
    return this.engine.queryNodes(query);
  }

  getByResonance(...args: Parameters<LocusStoreEngine["getByResonance"]>) {
    return this.engine.getByResonance(...args);
  }

  getByResonanceGlobal(...args: Parameters<LocusStoreEngine["getByResonanceGlobal"]>) {
    return this.engine.getByResonanceGlobal(...args);
  }

  getByHybrid(...args: Parameters<LocusStoreEngine["getByHybrid"]>) {
    return this.engine.getByHybrid(...args);
  }

  getByHybridGlobal(...args: Parameters<LocusStoreEngine["getByHybridGlobal"]>) {
    return this.engine.getByHybridGlobal(...args);
  }

  listNodes(limit: number, sessionId?: string) {
    return this.engine.listNodes(limit, sessionId);
  }

  getLastAvec(sessionId: string) {
    return this.engine.getLastAvec(sessionId);
  }

  getTriggerHistory(sessionId: string) {
    return this.engine.getTriggerHistory(sessionId);
  }

  storeCalibration(sessionId: string, avec: SttpNode["modelAvec"], trigger: string) {
    this.engine.storeCalibration(sessionId, avec, trigger);
  }

  queryChangesSince(...args: Parameters<LocusStoreEngine["queryChangesSince"]>) {
    return this.engine.queryChangesSince(...args);
  }

  getCheckpoint(sessionId: string, connectorId: string) {
    return this.engine.getCheckpoint(sessionId, connectorId);
  }

  putCheckpoint(checkpoint: SyncCheckpoint) {
    this.engine.putCheckpoint(checkpoint);
  }

  deleteNodes(request: Parameters<LocusStoreEngine["deleteNodes"]>[0]) {
    return this.engine.deleteNodes(request);
  }

  purgeSession(request: Parameters<LocusStoreEngine["purgeSession"]>[0]) {
    return this.engine.purgeSession(request);
  }

  batchRekeyScopes(...args: Parameters<LocusStoreEngine["batchRekeyScopes"]>) {
    return this.engine.batchRekeyScopes(...args);
  }

  syncNodeTags(...args: Parameters<LocusStoreEngine["syncNodeTags"]>) {
    return this.engine.syncNodeTags(...args);
  }

  deleteNodeTags(tenantId: string, syncKey: string) {
    this.engine.deleteNodeTags(tenantId, syncKey);
  }

  findSyncKeysByTags(...args: Parameters<LocusStoreEngine["findSyncKeysByTags"]>) {
    return this.engine.findSyncKeysByTags(...args);
  }

  findTags(...args: Parameters<LocusStoreEngine["findTags"]>) {
    return this.engine.findTags(...args);
  }

  queryTagRecords(filter: Parameters<LocusStoreEngine["queryTagRecords"]>[0]) {
    return this.engine.queryTagRecords(filter);
  }
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);
    if (request.method === "GET" && url.pathname === "/health") {
      return Response.json({ ok: true });
    }

    const match = url.pathname.match(/^\/stores\/([^/]+)$/);
    if (!match || request.method !== "POST") {
      return Response.json({ error: "not found" }, { status: 404 });
    }

    let body: { method?: string; args?: unknown[] };
    try {
      body = (await request.json()) as { method?: string; args?: unknown[] };
    } catch {
      return Response.json({ error: "expected json body" }, { status: 400 });
    }

    if (!body.method || !METHODS.includes(body.method as StoreMethod)) {
      return Response.json({ error: "unknown method" }, { status: 400 });
    }

    const storeName = decodeURIComponent(match[1]);
    const stub = env.LOCUS_STORE.getByName(storeName);
    try {
      const result = await stub[body.method as StoreMethod](...(body.args ?? []));
      return Response.json({ result });
    } catch (error) {
      const message = error instanceof Error ? error.message : "store call failed";
      return Response.json({ error: message }, { status: 400 });
    }
  },
};

declare module "cloudflare:workers" {
  export class DurableObject<E = unknown> {
    ctx: DurableObjectState;
    env: E;
    constructor(ctx: DurableObjectState, env: E);
  }
}

interface DurableObjectNamespace {
  getByName(name: string): LocusStore;
}
