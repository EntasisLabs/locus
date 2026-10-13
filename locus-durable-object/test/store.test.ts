import assert from "node:assert/strict";
import { DatabaseSync } from "node:sqlite";
import test from "node:test";

import { nodeSqliteRunner } from "../src/sql.ts";
import { LocusStoreEngine, type AvecState, type SttpNode } from "../src/store.ts";

function avec(stability: number): AvecState {
  return { stability, friction: 0.2, logic: 0.5, autonomy: 0.4 };
}

function node(sessionId: string, syncKey: string, stability: number): SttpNode {
  const state = avec(stability);
  return {
    raw: `raw-${syncKey}`,
    sessionId,
    tier: "raw",
    timestamp: "2026-03-05T06:30:00.000Z",
    compressionDepth: 1,
    syncKey,
    updatedAt: "2026-03-05T06:30:00.000Z",
    contextSummary: `summary-${syncKey}`,
    semanticTags: ["alpha"],
    userAvec: state,
    modelAvec: state,
    compressionAvec: state,
    rho: 0.9,
    kappa: 0.8,
    psi: state.stability + state.friction + state.logic + state.autonomy,
  };
}

function openStore(): LocusStoreEngine {
  return new LocusStoreEngine(nodeSqliteRunner(new DatabaseSync(":memory:")));
}

test("upsert classifies create, duplicate, and update", () => {
  const store = openStore();
  const created = store.upsertNode(node("demo", "sync-1", 0.8));
  assert.equal(created.status, "created");

  const duplicate = store.upsertNode(node("demo", "sync-1", 0.8));
  assert.equal(duplicate.status, "duplicate");
  assert.equal(duplicate.nodeId, created.nodeId);

  const updated = store.upsertNode({ ...node("demo", "sync-1", 0.8), raw: "raw-changed" });
  assert.equal(updated.status, "updated");
  assert.equal(updated.nodeId, created.nodeId);
  assert.equal(store.queryNodes({ sessionId: "demo" })[0].raw, "raw-changed");
});

test("resonance orders by model avec distance", () => {
  const store = openStore();
  store.upsertNode(node("demo", "far", 0.1));
  store.upsertNode(node("demo", "near", 0.8));
  const ranked = store.getByResonance("demo", avec(0.8), { limit: 2 });
  assert.deepEqual(
    ranked.map((item) => item.syncKey),
    ["near", "far"],
  );
});

test("hybrid prefers the embedding match inside the candidate window", () => {
  const store = openStore();
  store.upsertNode({ ...node("demo", "avec", 0.8), embedding: [0, 1] });
  store.upsertNode({ ...node("demo", "semantic", 0.55), embedding: [1, 0] });
  const ranked = store.getByHybrid("demo", avec(0.8), {
    queryEmbedding: [1, 0],
    alpha: 0.2,
    beta: 0.8,
    limit: 1,
  });
  assert.equal(ranked[0].syncKey, "semantic");
});

test("changes cursor pages in sync key order", () => {
  const store = openStore();
  store.upsertNode({ ...node("demo", "a", 0.5), updatedAt: "2026-03-05T01:00:00.000Z" });
  store.upsertNode({ ...node("demo", "b", 0.5), updatedAt: "2026-03-05T02:00:00.000Z" });
  const page = store.queryChangesSince("demo", null, 1);
  assert.equal(page.hasMore, true);
  assert.equal(page.nodes[0].syncKey, "a");
  const next = store.queryChangesSince("demo", page.nextCursor, 10);
  assert.equal(next.hasMore, false);
  assert.equal(next.nodes[0].syncKey, "b");
});

test("checkpoint round trip and calibration history", () => {
  const store = openStore();
  store.storeCalibration("demo", avec(0.2), "first");
  store.storeCalibration("demo", avec(0.9), "second");
  assert.equal(store.getLastAvec("demo")?.stability, 0.9);
  assert.deepEqual(store.getTriggerHistory("demo"), ["first", "second"]);
  store.putCheckpoint({
    sessionId: "demo",
    connectorId: "connector",
    cursor: { updatedAt: "2026-03-05T06:00:00.000Z", syncKey: "a" },
    updatedAt: "2026-03-06T00:00:00.000Z",
  });
  assert.equal(store.getCheckpoint("demo", "connector")?.cursor?.syncKey, "a");
});

test("delete dry run then purge removes tags", () => {
  const store = openStore();
  const created = store.upsertNode(node("demo", "sync-1", 0.4));
  store.syncNodeTags(
    {
      tenantId: "default",
      sessionId: "demo",
      nodeId: created.nodeId,
      syncKey: "sync-1",
    },
    ["alpha", "beta"],
  );
  const preview = store.deleteNodes({
    tenantId: "default",
    sessionId: "demo",
    syncKeys: ["sync-1"],
    dryRun: true,
  });
  assert.equal(preview.deleted, 1);
  assert.equal(preview.records[0].reason, "would delete");

  const purged = store.purgeSession({
    tenantId: "default",
    sessionId: "demo",
    includeCalibration: true,
  });
  assert.equal(purged.deleted, 1);
  assert.deepEqual(store.queryTagRecords({ tenantId: "default" }), []);
});

test("rekey moves the session scope", () => {
  const store = openStore();
  const created = store.upsertNode(node("demo", "sync-1", 0.4));
  store.storeCalibration("demo", avec(0.4), "seed");
  const moved = store.batchRekeyScopes(
    [`temporal_node:${created.nodeId}`],
    "acme",
    "tenant:acme::session:next",
  );
  assert.equal(moved.temporalNodesUpdated, 1);
  assert.equal(moved.calibrationsUpdated, 1);
  assert.equal(store.queryNodes({ sessionId: "demo" }).length, 0);
  assert.equal(store.queryNodes({ sessionId: "tenant:acme::session:next" }).length, 1);
});

test("tag sync replaces removed tags and match-all lookup", () => {
  const store = openStore();
  for (const [syncKey, labels] of [
    ["a", ["x", "y"]],
    ["b", ["x"]],
  ] as const) {
    store.syncNodeTags(
      { tenantId: "default", sessionId: "demo", nodeId: syncKey, syncKey },
      [...labels],
    );
  }
  store.syncNodeTags(
    { tenantId: "default", sessionId: "demo", nodeId: "a", syncKey: "a" },
    ["x"],
  );
  assert.deepEqual(
    store.findSyncKeysByTags("default", ["x", "y"], true),
    [],
  );
  assert.deepEqual(store.findSyncKeysByTags("default", ["x"], false), ["a", "b"]);
});
