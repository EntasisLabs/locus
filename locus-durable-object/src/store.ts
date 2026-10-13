import { randomUUID } from "node:crypto";

import { initializeSchema, type SqlRow, type SqlRunner, type SqlValue } from "./sql.ts";

const DEFAULT_TENANT = "default";
const TENANT_PREFIX = "tenant:";
const TENANT_SEPARATOR = "::session:";

const NODE_COLUMNS = `id, tenant_id, session_id, raw, tier, timestamp_ms, compression_depth, parent_node_id,
sync_key, updated_at_ms, source_metadata, context_summary, semantic_tags, semantic_links,
embedding, embedding_model, embedding_dimensions, embedded_at_ms, psi, rho, kappa,
user_stability, user_friction, user_logic, user_autonomy, user_psi,
model_stability, model_friction, model_logic, model_autonomy, model_psi,
comp_stability, comp_friction, comp_logic, comp_autonomy, comp_psi`;

export interface AvecState {
  stability: number;
  friction: number;
  logic: number;
  autonomy: number;
}

export interface SemanticLink {
  rel: string;
  target: string;
  confidence?: number;
}

export interface SttpNode {
  raw: string;
  sessionId: string;
  tier: string;
  timestamp: string;
  compressionDepth: number;
  parentNodeId?: string | null;
  syncKey: string;
  updatedAt: string;
  sourceMetadata?: unknown;
  contextSummary?: string | null;
  semanticTags?: string[] | null;
  semanticLinks?: SemanticLink[] | null;
  embedding?: number[] | null;
  embeddingModel?: string | null;
  embeddingDimensions?: number | null;
  embeddedAt?: string | null;
  userAvec: AvecState;
  modelAvec: AvecState;
  compressionAvec?: AvecState | null;
  rho: number;
  kappa: number;
  psi: number;
}

export interface NodeUpsertResult {
  nodeId: string;
  syncKey: string;
  status: "created" | "updated" | "duplicate";
  updatedAt: string;
}

export interface SyncCursor {
  updatedAt: string;
  syncKey: string;
}

export interface ChangePage {
  nodes: SttpNode[];
  nextCursor: SyncCursor | null;
  hasMore: boolean;
}

export interface SyncCheckpoint {
  sessionId: string;
  connectorId: string;
  cursor?: SyncCursor | null;
  updatedAt: string;
  metadata?: unknown;
}

export interface DeleteRequest {
  tenantId?: string;
  sessionId: string;
  syncKeys?: string[];
  nodeIds?: string[];
  dryRun?: boolean;
}

export interface PurgeRequest {
  tenantId?: string;
  sessionId: string;
  tiers?: string[];
  dryRun?: boolean;
  includeCalibration?: boolean;
  includeCheckpoints?: boolean;
}

export interface DeleteRecord {
  nodeId: string;
  syncKey: string;
  status: "deleted" | "not_found" | "skipped";
  reason?: string;
}

export interface DeleteResult {
  dryRun: boolean;
  deleted: number;
  notFound: number;
  skipped: number;
  calibrationsDeleted: number;
  checkpointsDeleted: number;
  records: DeleteRecord[];
}

export interface RekeyResult {
  dryRun: boolean;
  requestedNodeIds: number;
  resolvedNodeIds: number;
  missingNodeIds: string[];
  temporalNodesUpdated: number;
  calibrationsUpdated: number;
  scopes: Array<{
    sourceTenantId: string;
    sourceSessionId: string;
    targetTenantId: string;
    targetSessionId: string;
    temporalNodes: number;
    calibrations: number;
    applied: boolean;
    conflict: boolean;
    message?: string;
  }>;
}

export interface TagRef {
  tenantId: string;
  sessionId: string;
  nodeId: string;
  syncKey: string;
}

export interface TagRecord {
  tenantId: string;
  sessionId: string;
  nodeId: string;
  syncKey: string;
  tag: string;
  embedding?: number[] | null;
  embeddingModel?: string | null;
  embeddingDimensions?: number | null;
  embeddedAt?: string | null;
  updatedAt: string;
}

interface StoredNode {
  id: string;
  node: SttpNode;
}

/**
 * One SQLite database implementing the Locus node and tag operations.
 * A Durable Object hosts exactly one of these. That object is one database,
 * the same boundary as a SQLite file.
 */
export class LocusStoreEngine {
  private readonly sql: SqlRunner;

  constructor(sql: SqlRunner) {
    this.sql = sql;
    initializeSchema(sql);
  }

  upsertNode(input: SttpNode): NodeUpsertResult {
    const syncKey = input.syncKey.trim();
    if (!syncKey) {
      throw new Error("sync_key is required");
    }
    const node = { ...input, syncKey };
    const compression = resolvedCompression(node);
    const tenantId = deriveTenantId(node.sessionId);
    return this.sql.transaction(() => {
      const existing = this.findExisting(tenantId, node.sessionId, syncKey);
      if (existing && nodesEquivalent(existing.node, node, compression)) {
        return {
          nodeId: existing.id,
          syncKey,
          status: "duplicate" as const,
          updatedAt: existing.node.updatedAt,
        };
      }
      if (existing) {
        this.writeNode(existing.id, tenantId, node, compression, false);
        return {
          nodeId: existing.id,
          syncKey,
          status: "updated" as const,
          updatedAt: node.updatedAt,
        };
      }
      const nodeId = randomUUID().replaceAll("-", "");
      try {
        this.writeNode(nodeId, tenantId, node, compression, true);
      } catch (error) {
        if (!isUniqueConflict(error)) {
          throw error;
        }
        const raced = this.findExisting(tenantId, node.sessionId, syncKey);
        if (!raced) {
          throw error;
        }
        if (nodesEquivalent(raced.node, node, compression)) {
          return {
            nodeId: raced.id,
            syncKey,
            status: "duplicate" as const,
            updatedAt: raced.node.updatedAt,
          };
        }
        this.writeNode(raced.id, tenantId, node, compression, false);
        return {
          nodeId: raced.id,
          syncKey,
          status: "updated" as const,
          updatedAt: node.updatedAt,
        };
      }
      return {
        nodeId,
        syncKey,
        status: "created" as const,
        updatedAt: node.updatedAt,
      };
    });
  }

  queryNodes(query: {
    sessionId?: string;
    fromUtc?: string;
    toUtc?: string;
    tiers?: string[];
    limit?: number;
  }): SttpNode[] {
    const { where, params } = nodeFilter(query);
    const limit = Math.max(query.limit ?? 500, 1);
    params.push(limit);
    return this.sql
      .all(
        `SELECT ${NODE_COLUMNS} FROM temporal_node ${where} ORDER BY timestamp_ms DESC LIMIT ?`,
        params,
      )
      .map(mapNode);
  }

  getByResonance(
    sessionId: string,
    avec: AvecState,
    options: { fromUtc?: string; toUtc?: string; tiers?: string[]; limit?: number } = {},
  ): SttpNode[] {
    return this.resonance(sessionId, avec, options);
  }

  getByResonanceGlobal(
    avec: AvecState,
    options: { fromUtc?: string; toUtc?: string; tiers?: string[]; limit?: number } = {},
  ): SttpNode[] {
    return this.resonance(undefined, avec, options);
  }

  getByHybrid(
    sessionId: string,
    avec: AvecState,
    options: {
      fromUtc?: string;
      toUtc?: string;
      tiers?: string[];
      queryEmbedding?: number[];
      alpha?: number;
      beta?: number;
      limit?: number;
    } = {},
  ): SttpNode[] {
    return this.hybrid(sessionId, avec, options);
  }

  getByHybridGlobal(
    avec: AvecState,
    options: {
      fromUtc?: string;
      toUtc?: string;
      tiers?: string[];
      queryEmbedding?: number[];
      alpha?: number;
      beta?: number;
      limit?: number;
    } = {},
  ): SttpNode[] {
    return this.hybrid(undefined, avec, options);
  }

  listNodes(limit: number, sessionId?: string): SttpNode[] {
    return this.queryNodes({ sessionId, limit: Math.min(Math.max(limit, 1), 200) });
  }

  getLastAvec(sessionId: string): AvecState | null {
    const row = this.sql.all(
      `SELECT stability, friction, logic, autonomy FROM calibration
       WHERE session_id = ? AND (tenant_id = ? OR tenant_id IS NULL OR tenant_id = '')
       ORDER BY created_at_ms DESC, rowid DESC LIMIT 1`,
      [sessionId, deriveTenantId(sessionId)],
    )[0];
    if (!row) {
      return null;
    }
    return {
      stability: numberValue(row.stability),
      friction: numberValue(row.friction),
      logic: numberValue(row.logic),
      autonomy: numberValue(row.autonomy),
    };
  }

  getTriggerHistory(sessionId: string): string[] {
    return this.sql
      .all(
        `SELECT trigger FROM calibration
         WHERE session_id = ? AND (tenant_id = ? OR tenant_id IS NULL OR tenant_id = '')
         ORDER BY created_at_ms ASC, rowid ASC`,
        [sessionId, deriveTenantId(sessionId)],
      )
      .map((row) => String(row.trigger));
  }

  storeCalibration(sessionId: string, avec: AvecState, trigger: string): void {
    this.sql.run(
      `INSERT INTO calibration (
        id, tenant_id, session_id, stability, friction, logic, autonomy, psi, trigger, created_at_ms
      ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
      [
        randomUUID().replaceAll("-", ""),
        deriveTenantId(sessionId),
        sessionId,
        avec.stability,
        avec.friction,
        avec.logic,
        avec.autonomy,
        psi(avec),
        trigger,
        Date.now(),
      ],
    );
  }

  queryChangesSince(sessionId: string, cursor: SyncCursor | null, limit: number): ChangePage {
    const capped = Math.max(limit, 1);
    const params: SqlValue[] = [sessionId, deriveTenantId(sessionId)];
    let cursorClause = "";
    if (cursor) {
      const updatedAt = millis(cursor.updatedAt);
      params.push(updatedAt, updatedAt, cursor.syncKey);
      cursorClause = "AND (updated_at_ms > ? OR (updated_at_ms = ? AND sync_key > ?))";
    }
    params.push(capped + 1);
    const nodes = this.sql
      .all(
        `SELECT ${NODE_COLUMNS} FROM temporal_node
         WHERE session_id = ? AND (tenant_id = ? OR tenant_id IS NULL OR tenant_id = '')
         ${cursorClause}
         ORDER BY updated_at_ms ASC, sync_key ASC
         LIMIT ?`,
        params,
      )
      .map(mapNode);
    const hasMore = nodes.length > capped;
    if (hasMore) {
      nodes.length = capped;
    }
    const last = nodes.at(-1);
    return {
      nodes,
      hasMore,
      nextCursor: last ? { updatedAt: last.updatedAt, syncKey: last.syncKey } : null,
    };
  }

  getCheckpoint(sessionId: string, connectorId: string): SyncCheckpoint | null {
    const row = this.sql.all(
      `SELECT session_id, connector_id, cursor_updated_at_ms, cursor_sync_key, metadata, updated_at_ms
       FROM sync_checkpoint
       WHERE session_id = ? AND connector_id = ?
         AND (tenant_id = ? OR tenant_id IS NULL OR tenant_id = '')
       LIMIT 1`,
      [sessionId, connectorId, deriveTenantId(sessionId)],
    )[0];
    if (!row) {
      return null;
    }
    const cursorUpdated = row.cursor_updated_at_ms;
    const cursorKey = row.cursor_sync_key;
    return {
      sessionId: String(row.session_id),
      connectorId: String(row.connector_id),
      cursor:
        cursorUpdated != null && cursorKey != null
          ? { updatedAt: iso(Number(cursorUpdated)), syncKey: String(cursorKey) }
          : null,
      updatedAt: iso(Number(row.updated_at_ms)),
      metadata: parseJson(row.metadata),
    };
  }

  putCheckpoint(checkpoint: SyncCheckpoint): void {
    this.sql.run(
      `INSERT INTO sync_checkpoint (
        id, tenant_id, session_id, connector_id, cursor_updated_at_ms, cursor_sync_key, metadata, updated_at_ms
      ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
      ON CONFLICT(tenant_id, session_id, connector_id) DO UPDATE SET
        cursor_updated_at_ms = excluded.cursor_updated_at_ms,
        cursor_sync_key = excluded.cursor_sync_key,
        metadata = excluded.metadata,
        updated_at_ms = excluded.updated_at_ms`,
      [
        randomUUID().replaceAll("-", ""),
        deriveTenantId(checkpoint.sessionId),
        checkpoint.sessionId,
        checkpoint.connectorId,
        checkpoint.cursor ? millis(checkpoint.cursor.updatedAt) : null,
        checkpoint.cursor?.syncKey ?? null,
        checkpoint.metadata == null ? null : JSON.stringify(checkpoint.metadata),
        millis(checkpoint.updatedAt),
      ],
    );
  }

  deleteNodes(request: DeleteRequest): DeleteResult {
    const tenantId = normalizeTenant(request.tenantId);
    const sessionId = request.sessionId.trim();
    if (!sessionId) {
      throw new Error("session id cannot be empty");
    }
    const syncKeys = request.syncKeys ?? [];
    const nodeIds = request.nodeIds ?? [];
    if (syncKeys.length === 0 && nodeIds.length === 0) {
      throw new Error("at least one sync key or node id is required");
    }
    const records: DeleteRecord[] = [];
    const targets = new Set<string>();
    for (const rawKey of syncKeys) {
      const syncKey = rawKey.trim();
      if (!syncKey) {
        continue;
      }
      const row = this.sql.all(
        "SELECT id FROM temporal_node WHERE tenant_id = ? AND session_id = ? AND sync_key = ? LIMIT 1",
        [tenantId, sessionId, syncKey],
      )[0];
      if (row) {
        targets.add(String(row.id));
      } else {
        records.push({
          nodeId: "",
          syncKey,
          status: "not_found",
          reason: "sync key not found in session scope",
        });
      }
    }
    for (const rawId of nodeIds) {
      const normalized = normalizeNodeId(rawId);
      if (!normalized) {
        records.push({
          nodeId: rawId,
          syncKey: "",
          status: "not_found",
          reason: "invalid node id",
        });
        continue;
      }
      const row = this.sql.all(
        "SELECT id, sync_key, session_id FROM temporal_node WHERE id = ?",
        [normalized],
      )[0];
      if (!row) {
        records.push({
          nodeId: normalized,
          syncKey: "",
          status: "not_found",
          reason: "node id not found",
        });
      } else if (row.session_id !== sessionId) {
        records.push({
          nodeId: normalized,
          syncKey: String(row.sync_key ?? ""),
          status: "not_found",
          reason: "node id not found in session scope",
        });
      } else {
        targets.add(String(row.id));
      }
    }
    for (const nodeId of [...targets].sort()) {
      const row = this.sql.all("SELECT sync_key FROM temporal_node WHERE id = ?", [nodeId])[0];
      if (!row) {
        continue;
      }
      records.push({
        nodeId,
        syncKey: String(row.sync_key),
        status: request.dryRun ? "skipped" : "deleted",
        reason: request.dryRun ? "would delete" : undefined,
      });
    }
    if (!request.dryRun) {
      this.sql.transaction(() => {
        for (const nodeId of targets) {
          this.sql.run("DELETE FROM temporal_node WHERE id = ?", [nodeId]);
        }
      });
    }
    return summarize(Boolean(request.dryRun), 0, 0, records);
  }

  purgeSession(request: PurgeRequest): DeleteResult {
    const tenantId = normalizeTenant(request.tenantId);
    const sessionId = request.sessionId.trim();
    if (!sessionId) {
      throw new Error("session id cannot be empty");
    }
    const { where, params } = nodeFilter({
      exactTenantId: tenantId,
      sessionId,
      tiers: request.tiers,
    });
    const targets = this.sql.all(
      `SELECT id, sync_key FROM temporal_node ${where} ORDER BY sync_key ASC`,
      params,
    );
    const records: DeleteRecord[] = targets.map((row) => ({
      nodeId: String(row.id),
      syncKey: String(row.sync_key),
      status: request.dryRun ? "skipped" : "deleted",
      reason: request.dryRun ? "would delete" : undefined,
    }));
    const calibrationsDeleted = request.includeCalibration
      ? this.countScope("calibration", tenantId, sessionId)
      : 0;
    const checkpointsDeleted = request.includeCheckpoints
      ? this.countScope("sync_checkpoint", tenantId, sessionId)
      : 0;
    if (!request.dryRun) {
      this.sql.transaction(() => {
        this.sql.run(`DELETE FROM temporal_node ${where}`, params);
        this.sql.run(
          "DELETE FROM semantic_tag_index WHERE tenant_id = ? AND session_id = ?",
          [tenantId, sessionId],
        );
        if (request.includeCalibration) {
          this.sql.run("DELETE FROM calibration WHERE tenant_id = ? AND session_id = ?", [
            tenantId,
            sessionId,
          ]);
        }
        if (request.includeCheckpoints) {
          this.sql.run("DELETE FROM sync_checkpoint WHERE tenant_id = ? AND session_id = ?", [
            tenantId,
            sessionId,
          ]);
        }
      });
    }
    return summarize(Boolean(request.dryRun), calibrationsDeleted, checkpointsDeleted, records);
  }

  batchRekeyScopes(
    nodeIds: string[],
    targetTenantId: string,
    targetSessionId: string,
    dryRun = false,
    allowMerge = false,
  ): RekeyResult {
    if (nodeIds.length === 0) {
      throw new Error("at least one node id is required");
    }
    if (!targetSessionId.trim()) {
      throw new Error("target session id cannot be empty");
    }
    const normalized = nodeIds
      .map(normalizeNodeId)
      .filter((id): id is string => Boolean(id));
    if (normalized.length === 0) {
      throw new Error("no valid node ids were provided");
    }
    const targetTenant = normalizeTenant(targetTenantId);
    const missing: string[] = [];
    const scopes: Array<[string, string]> = [];
    for (const nodeId of normalized) {
      const row = this.sql.all(
        "SELECT tenant_id, session_id FROM temporal_node WHERE id = ?",
        [nodeId],
      )[0];
      if (!row) {
        missing.push(nodeId);
        continue;
      }
      const sessionId = String(row.session_id);
      const tenantId = String(row.tenant_id ?? "").trim()
        ? String(row.tenant_id)
        : deriveTenantId(sessionId);
      if (!scopes.some(([tenant, session]) => tenant === tenantId && session === sessionId)) {
        scopes.push([tenantId, sessionId]);
      }
    }

    return this.sql.transaction(() => {
      let temporalNodesUpdated = 0;
      let calibrationsUpdated = 0;
      const scopeResults = scopes.map(([sourceTenantId, sourceSessionId]) => {
        const sameScope =
          sourceTenantId === targetTenant && sourceSessionId === targetSessionId;
        const includeLegacy = sourceTenantId === DEFAULT_TENANT;
        const temporalNodes = this.countRekey(
          "temporal_node",
          sourceTenantId,
          sourceSessionId,
          includeLegacy,
        );
        const calibrations = this.countRekey(
          "calibration",
          sourceTenantId,
          sourceSessionId,
          includeLegacy,
        );
        const targetTemporal = sameScope
          ? 0
          : this.countRekey(
              "temporal_node",
              targetTenant,
              targetSessionId,
              targetTenant === DEFAULT_TENANT,
            );
        const targetCalibrations = sameScope
          ? 0
          : this.countRekey(
              "calibration",
              targetTenant,
              targetSessionId,
              targetTenant === DEFAULT_TENANT,
            );
        const conflict =
          !allowMerge && !sameScope && (targetTemporal > 0 || targetCalibrations > 0);
        let applied = false;
        let message: string | undefined;
        if (sameScope) {
          message = "source and target scopes are identical";
        } else if (conflict) {
          message = "target scope already contains rows; set allow_merge=true to override";
        } else if (!dryRun) {
          this.applyRekey(
            "temporal_node",
            sourceTenantId,
            sourceSessionId,
            targetTenant,
            targetSessionId,
            includeLegacy,
          );
          this.applyRekey(
            "calibration",
            sourceTenantId,
            sourceSessionId,
            targetTenant,
            targetSessionId,
            includeLegacy,
          );
          applied = true;
          temporalNodesUpdated += temporalNodes;
          calibrationsUpdated += calibrations;
        }
        return {
          sourceTenantId,
          sourceSessionId,
          targetTenantId: targetTenant,
          targetSessionId,
          temporalNodes,
          calibrations,
          applied,
          conflict,
          message,
        };
      });
      return {
        dryRun,
        requestedNodeIds: normalized.length,
        resolvedNodeIds: normalized.length - missing.length,
        missingNodeIds: missing,
        temporalNodesUpdated,
        calibrationsUpdated,
        scopes: scopeResults,
      };
    });
  }

  syncNodeTags(
    nodeRef: TagRef,
    tags: string[],
    embeddings?: Record<string, { vector: number[]; model: string }>,
  ): void {
    const canonical = [
      ...new Set(tags.map(canonicalTag).filter((tag) => tag.length > 0)),
    ];
    const now = Date.now();
    this.sql.transaction(() => {
      const existing = this.sql.all(
        `SELECT tag, embedding, embedding_model, embedding_dimensions, embedded_at_ms
         FROM semantic_tag_index WHERE tenant_id = ? AND sync_key = ?`,
        [nodeRef.tenantId, nodeRef.syncKey],
      );
      for (const prior of existing) {
        if (!canonical.includes(String(prior.tag))) {
          this.sql.run(
            "DELETE FROM semantic_tag_index WHERE tenant_id = ? AND sync_key = ? AND tag = ?",
            [nodeRef.tenantId, nodeRef.syncKey, String(prior.tag)],
          );
        }
      }
      for (const tag of canonical) {
        const prior = existing.find((row) => row.tag === tag);
        const embedding = lookupEmbedding(embeddings, tag);
        this.sql.run(
          `INSERT INTO semantic_tag_index (
            tenant_id, session_id, node_id, sync_key, tag, embedding, embedding_model,
            embedding_dimensions, embedded_at_ms, updated_at_ms
          ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
          ON CONFLICT(tenant_id, sync_key, tag) DO UPDATE SET
            session_id = excluded.session_id,
            node_id = excluded.node_id,
            embedding = excluded.embedding,
            embedding_model = excluded.embedding_model,
            embedding_dimensions = excluded.embedding_dimensions,
            embedded_at_ms = excluded.embedded_at_ms,
            updated_at_ms = excluded.updated_at_ms`,
          [
            nodeRef.tenantId,
            nodeRef.sessionId,
            nodeRef.nodeId,
            nodeRef.syncKey,
            tag,
            embedding
              ? JSON.stringify(embedding.vector)
              : (prior?.embedding ?? null),
            embedding ? embedding.model : (prior?.embedding_model ?? null),
            embedding
              ? embedding.vector.length
              : (prior?.embedding_dimensions ?? null),
            embedding ? now : (prior?.embedded_at_ms ?? null),
            now,
          ],
        );
      }
    });
  }

  deleteNodeTags(tenantId: string, syncKey: string): void {
    this.sql.run("DELETE FROM semantic_tag_index WHERE tenant_id = ? AND sync_key = ?", [
      tenantId,
      syncKey,
    ]);
  }

  findSyncKeysByTags(
    tenantId: string,
    tags: string[],
    matchAll: boolean,
    sessionId?: string,
    limit = 100,
  ): string[] {
    const canonical = tags.map(canonicalTag).filter((tag) => tag.length > 0);
    if (canonical.length === 0) {
      return [];
    }
    const params: SqlValue[] = [tenantId];
    let sql = "SELECT sync_key, tag FROM semantic_tag_index WHERE tenant_id = ?";
    if (sessionId) {
      params.push(sessionId);
      sql += " AND session_id = ?";
    }
    const counts = new Map<string, number>();
    for (const row of this.sql.all(sql, params)) {
      if (canonical.includes(String(row.tag))) {
        const key = String(row.sync_key);
        counts.set(key, (counts.get(key) ?? 0) + 1);
      }
    }
    return [...counts.entries()]
      .filter(([, count]) => (matchAll ? count >= canonical.length : count > 0))
      .map(([key]) => key)
      .sort()
      .slice(0, limit);
  }

  findTags(tenantId: string, prefix?: string, limit = 100): string[] {
    const tags = this.sql
      .all(
        "SELECT DISTINCT tag FROM semantic_tag_index WHERE tenant_id = ? ORDER BY tag ASC",
        [tenantId],
      )
      .map((row) => String(row.tag));
    const filtered = prefix ? tags.filter((tag) => tag.startsWith(prefix.toLowerCase())) : tags;
    return filtered.slice(0, limit);
  }

  queryTagRecords(filter: {
    tenantId?: string;
    sessionId?: string;
    tags?: string[];
    tagPrefix?: string;
    hasEmbedding?: boolean;
    missingEmbeddingOnly?: boolean;
    limit?: number;
  }): TagRecord[] {
    const clauses: string[] = [];
    const params: SqlValue[] = [];
    if (filter.tenantId) {
      params.push(filter.tenantId);
      clauses.push("tenant_id = ?");
    }
    if (filter.sessionId) {
      params.push(filter.sessionId);
      clauses.push("session_id = ?");
    }
    if (filter.tags) {
      const canonical = filter.tags.map(canonicalTag).filter((tag) => tag.length > 0);
      if (canonical.length === 0) {
        return [];
      }
      clauses.push(`tag IN (${canonical.map(() => "?").join(", ")})`);
      params.push(...canonical);
    }
    if (filter.tagPrefix) {
      params.push(likePrefix(filter.tagPrefix.toLowerCase()));
      clauses.push("tag LIKE ? ESCAPE '\\'");
    }
    if (filter.hasEmbedding === true) {
      clauses.push("embedding IS NOT NULL");
    } else if (filter.hasEmbedding === false) {
      clauses.push("embedding IS NULL");
    }
    if (filter.missingEmbeddingOnly) {
      clauses.push("embedding IS NULL");
    }
    const limit = Math.max(filter.limit ?? 100, 1);
    params.push(limit);
    const where = clauses.length ? `WHERE ${clauses.join(" AND ")}` : "";
    return this.sql
      .all(
        `SELECT tenant_id, session_id, node_id, sync_key, tag, embedding, embedding_model,
                embedding_dimensions, embedded_at_ms, updated_at_ms
         FROM semantic_tag_index ${where}
         ORDER BY updated_at_ms DESC LIMIT ?`,
        params,
      )
      .map(mapTag);
  }

  private findExisting(tenantId: string, sessionId: string, syncKey: string): StoredNode | null {
    const row = this.sql.all(
      `SELECT ${NODE_COLUMNS} FROM temporal_node
       WHERE session_id = ? AND sync_key = ?
       ORDER BY CASE WHEN tenant_id = ? THEN 0 ELSE 1 END
       LIMIT 1`,
      [sessionId, syncKey, tenantId],
    )[0];
    return row ? { id: String(row.id), node: mapNode(row) } : null;
  }

  private writeNode(
    nodeId: string,
    tenantId: string,
    node: SttpNode,
    compression: AvecState,
    insert: boolean,
  ): void {
    const values = nodeValues(nodeId, tenantId, node, compression);
    if (insert) {
      const placeholders = values.map(() => "?").join(", ");
      this.sql.run(
        `INSERT INTO temporal_node (
          id, tenant_id, session_id, raw, tier, timestamp_ms, compression_depth, parent_node_id,
          sync_key, updated_at_ms, source_metadata, context_summary, semantic_tags, semantic_links,
          embedding, embedding_model, embedding_dimensions, embedded_at_ms, psi, rho, kappa,
          user_stability, user_friction, user_logic, user_autonomy, user_psi,
          model_stability, model_friction, model_logic, model_autonomy, model_psi,
          comp_stability, comp_friction, comp_logic, comp_autonomy, comp_psi
        ) VALUES (${placeholders})`,
        values,
      );
      return;
    }
    const assignments = [
      "tenant_id",
      "session_id",
      "raw",
      "tier",
      "timestamp_ms",
      "compression_depth",
      "parent_node_id",
      "sync_key",
      "updated_at_ms",
      "source_metadata",
      "context_summary",
      "semantic_tags",
      "semantic_links",
      "embedding",
      "embedding_model",
      "embedding_dimensions",
      "embedded_at_ms",
      "psi",
      "rho",
      "kappa",
      "user_stability",
      "user_friction",
      "user_logic",
      "user_autonomy",
      "user_psi",
      "model_stability",
      "model_friction",
      "model_logic",
      "model_autonomy",
      "model_psi",
      "comp_stability",
      "comp_friction",
      "comp_logic",
      "comp_autonomy",
      "comp_psi",
    ];
    const params = values.slice(1);
    params.push(nodeId);
    this.sql.run(
      `UPDATE temporal_node SET ${assignments.map((column) => `${column} = ?`).join(", ")} WHERE id = ?`,
      params,
    );
  }

  private resonance(
    sessionId: string | undefined,
    avec: AvecState,
    options: { fromUtc?: string; toUtc?: string; tiers?: string[]; limit?: number },
  ): SttpNode[] {
    const { where, params } = nodeFilter({ ...options, sessionId });
    params.push(avec.stability, avec.friction, avec.logic, avec.autonomy);
    const limit = Math.max(options.limit ?? 10, 1);
    params.push(limit);
    return this.sql
      .all(
        `SELECT ${NODE_COLUMNS} FROM temporal_node ${where}
         ORDER BY (ABS(model_stability - ?) + ABS(model_friction - ?) +
                   ABS(model_logic - ?) + ABS(model_autonomy - ?)) / 4.0 ASC,
                  updated_at_ms DESC
         LIMIT ?`,
        params,
      )
      .map(mapNode);
  }

  private hybrid(
    sessionId: string | undefined,
    avec: AvecState,
    options: {
      fromUtc?: string;
      toUtc?: string;
      tiers?: string[];
      queryEmbedding?: number[];
      alpha?: number;
      beta?: number;
      limit?: number;
    },
  ): SttpNode[] {
    const limit = Math.max(options.limit ?? 10, 1);
    const candidates = this.resonance(sessionId, avec, {
      ...options,
      limit: limit * 5,
    });
    candidates.sort((left, right) => {
      const score = hybridScore(right, avec, options) - hybridScore(left, avec, options);
      if (score !== 0) {
        return score;
      }
      return millis(right.updatedAt) - millis(left.updatedAt);
    });
    return candidates.slice(0, limit);
  }

  private countScope(table: string, tenantId: string, sessionId: string): number {
    const row = this.sql.all(
      `SELECT COUNT(*) AS count FROM ${table} WHERE tenant_id = ? AND session_id = ?`,
      [tenantId, sessionId],
    )[0];
    return Number(row?.count ?? 0);
  }

  private countRekey(
    table: string,
    tenantId: string,
    sessionId: string,
    includeLegacy: boolean,
  ): number {
    const row = this.sql.all(
      `SELECT COUNT(*) AS count FROM ${table}
       WHERE session_id = ? AND (tenant_id = ? OR (? = 1 AND (tenant_id IS NULL OR tenant_id = '')))`,
      [sessionId, tenantId, includeLegacy ? 1 : 0],
    )[0];
    return Number(row?.count ?? 0);
  }

  private applyRekey(
    table: string,
    sourceTenantId: string,
    sourceSessionId: string,
    targetTenantId: string,
    targetSessionId: string,
    includeLegacy: boolean,
  ): void {
    this.sql.run(
      `UPDATE ${table} SET tenant_id = ?, session_id = ?
       WHERE session_id = ? AND (tenant_id = ? OR (? = 1 AND (tenant_id IS NULL OR tenant_id = '')))`,
      [
        targetTenantId,
        targetSessionId,
        sourceSessionId,
        sourceTenantId,
        includeLegacy ? 1 : 0,
      ],
    );
  }
}

function nodeFilter(query: {
  sessionId?: string;
  exactTenantId?: string;
  fromUtc?: string;
  toUtc?: string;
  tiers?: string[];
}): { where: string; params: SqlValue[] } {
  const clauses: string[] = [];
  const params: SqlValue[] = [];
  if (query.exactTenantId && query.sessionId) {
    params.push(query.exactTenantId, query.sessionId);
    clauses.push("tenant_id = ? AND session_id = ?");
  } else if (query.sessionId) {
    params.push(query.sessionId, deriveTenantId(query.sessionId));
    clauses.push(
      "session_id = ? AND (tenant_id = ? OR tenant_id IS NULL OR tenant_id = '')",
    );
  }
  if (query.fromUtc) {
    params.push(millis(query.fromUtc));
    clauses.push("timestamp_ms >= ?");
  }
  if (query.toUtc) {
    params.push(millis(query.toUtc));
    clauses.push("timestamp_ms <= ?");
  }
  const tiers = (query.tiers ?? [])
    .map((tier) => tier.trim().toLowerCase())
    .filter((tier) => tier.length > 0);
  if (tiers.length > 0) {
    clauses.push(`lower(tier) IN (${tiers.map(() => "?").join(", ")})`);
    params.push(...tiers);
  }
  return {
    where: clauses.length ? `WHERE ${clauses.join(" AND ")}` : "",
    params,
  };
}

function nodeValues(
  nodeId: string,
  tenantId: string,
  node: SttpNode,
  compression: AvecState,
): SqlValue[] {
  return [
    nodeId,
    tenantId,
    node.sessionId,
    node.raw,
    node.tier,
    millis(node.timestamp),
    node.compressionDepth,
    node.parentNodeId ?? null,
    node.syncKey,
    millis(node.updatedAt),
    node.sourceMetadata == null ? null : JSON.stringify(node.sourceMetadata),
    node.contextSummary ?? null,
    node.semanticTags == null ? null : JSON.stringify(node.semanticTags),
    node.semanticLinks == null ? null : JSON.stringify(node.semanticLinks),
    node.embedding == null ? null : JSON.stringify(node.embedding),
    node.embeddingModel ?? null,
    node.embeddingDimensions ?? null,
    node.embeddedAt ? millis(node.embeddedAt) : null,
    node.psi,
    node.rho,
    node.kappa,
    node.userAvec.stability,
    node.userAvec.friction,
    node.userAvec.logic,
    node.userAvec.autonomy,
    psi(node.userAvec),
    node.modelAvec.stability,
    node.modelAvec.friction,
    node.modelAvec.logic,
    node.modelAvec.autonomy,
    psi(node.modelAvec),
    compression.stability,
    compression.friction,
    compression.logic,
    compression.autonomy,
    psi(compression),
  ];
}

function mapNode(row: SqlRow): SttpNode {
  const compression: AvecState = {
    stability: numberValue(row.comp_stability),
    friction: numberValue(row.comp_friction),
    logic: numberValue(row.comp_logic),
    autonomy: numberValue(row.comp_autonomy),
  };
  return {
    raw: String(row.raw),
    sessionId: String(row.session_id),
    tier: String(row.tier),
    timestamp: iso(Number(row.timestamp_ms)),
    compressionDepth: Number(row.compression_depth),
    parentNodeId: textOrNull(row.parent_node_id),
    syncKey: String(row.sync_key),
    updatedAt: iso(Number(row.updated_at_ms)),
    sourceMetadata: parseJson(row.source_metadata),
    contextSummary: textOrNull(row.context_summary),
    semanticTags: parseJson(row.semantic_tags) as string[] | null,
    semanticLinks: parseJson(row.semantic_links) as SemanticLink[] | null,
    embedding: parseJson(row.embedding) as number[] | null,
    embeddingModel: textOrNull(row.embedding_model),
    embeddingDimensions:
      row.embedding_dimensions == null ? null : Number(row.embedding_dimensions),
    embeddedAt: row.embedded_at_ms == null ? null : iso(Number(row.embedded_at_ms)),
    psi: numberValue(row.psi),
    rho: numberValue(row.rho),
    kappa: numberValue(row.kappa),
    userAvec: {
      stability: numberValue(row.user_stability),
      friction: numberValue(row.user_friction),
      logic: numberValue(row.user_logic),
      autonomy: numberValue(row.user_autonomy),
    },
    modelAvec: {
      stability: numberValue(row.model_stability),
      friction: numberValue(row.model_friction),
      logic: numberValue(row.model_logic),
      autonomy: numberValue(row.model_autonomy),
    },
    compressionAvec: compression,
  };
}

function mapTag(row: SqlRow): TagRecord {
  return {
    tenantId: String(row.tenant_id),
    sessionId: String(row.session_id),
    nodeId: String(row.node_id),
    syncKey: String(row.sync_key),
    tag: String(row.tag),
    embedding: parseJson(row.embedding) as number[] | null,
    embeddingModel: textOrNull(row.embedding_model),
    embeddingDimensions:
      row.embedding_dimensions == null ? null : Number(row.embedding_dimensions),
    embeddedAt: row.embedded_at_ms == null ? null : iso(Number(row.embedded_at_ms)),
    updatedAt: iso(Number(row.updated_at_ms)),
  };
}

function nodesEquivalent(existing: SttpNode, candidate: SttpNode, compression: AvecState): boolean {
  return (
    existing.raw === candidate.raw &&
    existing.tier === candidate.tier &&
    millis(existing.timestamp) === millis(candidate.timestamp) &&
    existing.compressionDepth === candidate.compressionDepth &&
    (existing.parentNodeId ?? null) === (candidate.parentNodeId ?? null) &&
    (existing.contextSummary ?? null) === (candidate.contextSummary ?? null) &&
    JSON.stringify(existing.semanticTags ?? null) === JSON.stringify(candidate.semanticTags ?? null) &&
    JSON.stringify(existing.semanticLinks ?? null) ===
      JSON.stringify(candidate.semanticLinks ?? null) &&
    vectorsEqual(existing.embedding, candidate.embedding) &&
    (existing.embeddingModel ?? null) === (candidate.embeddingModel ?? null) &&
    (existing.embeddingDimensions ?? null) === (candidate.embeddingDimensions ?? null) &&
    (existing.embeddedAt ? millis(existing.embeddedAt) : null) ===
      (candidate.embeddedAt ? millis(candidate.embeddedAt) : null) &&
    avecEqual(existing.userAvec, candidate.userAvec) &&
    avecEqual(existing.modelAvec, candidate.modelAvec) &&
    avecEqual(existing.compressionAvec ?? compression, compression) &&
    floatEqual(existing.rho, candidate.rho) &&
    floatEqual(existing.kappa, candidate.kappa) &&
    floatEqual(existing.psi, candidate.psi) &&
    JSON.stringify(existing.sourceMetadata ?? null) ===
      JSON.stringify(candidate.sourceMetadata ?? null)
  );
}

function resolvedCompression(node: SttpNode): AvecState {
  if (node.compressionAvec && psi(node.compressionAvec) !== 0) {
    return node.compressionAvec;
  }
  return node.modelAvec;
}

function hybridScore(
  node: SttpNode,
  avec: AvecState,
  options: { queryEmbedding?: number[]; alpha?: number; beta?: number },
): number {
  const alpha = clamp(options.alpha ?? 0.5);
  const beta = clamp(options.beta ?? 0.5);
  const resonance = clamp(1 - avecDistance(node.modelAvec, avec));
  const semantic = cosine(options.queryEmbedding, node.embedding ?? undefined);
  if (semantic == null) {
    return resonance;
  }
  return clamp(alpha * resonance + beta * ((semantic + 1) / 2));
}

function avecDistance(left: AvecState, right: AvecState): number {
  return (
    (Math.abs(left.stability - right.stability) +
      Math.abs(left.friction - right.friction) +
      Math.abs(left.logic - right.logic) +
      Math.abs(left.autonomy - right.autonomy)) /
    4
  );
}

function cosine(left?: number[], right?: number[]): number | null {
  if (!left || !right || left.length === 0 || left.length !== right.length) {
    return null;
  }
  let dot = 0;
  let leftNorm = 0;
  let rightNorm = 0;
  for (let index = 0; index < left.length; index += 1) {
    dot += left[index] * right[index];
    leftNorm += left[index] * left[index];
    rightNorm += right[index] * right[index];
  }
  if (leftNorm <= Number.EPSILON || rightNorm <= Number.EPSILON) {
    return null;
  }
  return dot / (Math.sqrt(leftNorm) * Math.sqrt(rightNorm));
}

function summarize(
  dryRun: boolean,
  calibrationsDeleted: number,
  checkpointsDeleted: number,
  records: DeleteRecord[],
): DeleteResult {
  const deleted = records.filter((record) => record.status === "deleted").length;
  const skipped = records.filter((record) => record.status === "skipped").length;
  const notFound = records.filter((record) => record.status === "not_found").length;
  return {
    dryRun,
    deleted: dryRun ? skipped : deleted,
    notFound,
    skipped: dryRun ? 0 : skipped,
    calibrationsDeleted,
    checkpointsDeleted,
    records,
  };
}

export function deriveTenantId(sessionId: string): string {
  const separator = sessionId.indexOf(TENANT_SEPARATOR);
  if (separator === -1) {
    return DEFAULT_TENANT;
  }
  const head = sessionId.slice(0, separator);
  if (!head.startsWith(TENANT_PREFIX)) {
    return DEFAULT_TENANT;
  }
  const tenant = head.slice(TENANT_PREFIX.length).trim();
  return tenant || DEFAULT_TENANT;
}

function normalizeTenant(value?: string): string {
  const trimmed = value?.trim() ?? "";
  return trimmed || DEFAULT_TENANT;
}

function normalizeNodeId(value: string): string | null {
  const trimmed = value.trim();
  if (!trimmed) {
    return null;
  }
  const split = trimmed.indexOf(":");
  if (split === -1) {
    return trimmed;
  }
  if (trimmed.slice(0, split) !== "temporal_node") {
    return null;
  }
  const id = trimmed.slice(split + 1).trim();
  return id || null;
}

function canonicalTag(tag: string): string {
  return tag.trim().toLowerCase();
}

function lookupEmbedding(
  embeddings: Record<string, { vector: number[]; model: string }> | undefined,
  tag: string,
): { vector: number[]; model: string } | undefined {
  if (!embeddings) {
    return undefined;
  }
  if (embeddings[tag]) {
    return embeddings[tag];
  }
  const match = Object.entries(embeddings).find(
    ([candidate]) => candidate.trim().toLowerCase() === tag,
  );
  return match?.[1];
}

function likePrefix(prefix: string): string {
  return `${prefix.replaceAll("\\", "\\\\").replaceAll("%", "\\%").replaceAll("_", "\\_")}%`;
}

function psi(avec: AvecState): number {
  return avec.stability + avec.friction + avec.logic + avec.autonomy;
}

function millis(value: string): number {
  const parsed = Date.parse(value);
  if (Number.isNaN(parsed)) {
    throw new Error(`invalid timestamp: ${value}`);
  }
  return parsed;
}

function iso(value: number): string {
  return new Date(value).toISOString();
}

function numberValue(value: SqlValue | undefined): number {
  return Number(value ?? 0);
}

function textOrNull(value: SqlValue | undefined): string | null {
  return value == null ? null : String(value);
}

function parseJson(value: SqlValue | undefined): unknown {
  if (value == null || value === "") {
    return null;
  }
  return JSON.parse(String(value));
}

function avecEqual(left: AvecState, right: AvecState): boolean {
  return (
    floatEqual(left.stability, right.stability) &&
    floatEqual(left.friction, right.friction) &&
    floatEqual(left.logic, right.logic) &&
    floatEqual(left.autonomy, right.autonomy)
  );
}

function vectorsEqual(left?: number[] | null, right?: number[] | null): boolean {
  if (left == null && right == null) {
    return true;
  }
  if (!left || !right || left.length !== right.length) {
    return false;
  }
  return left.every((value, index) => floatEqual(value, right[index]));
}

function floatEqual(left: number, right: number): boolean {
  return Math.abs(left - right) <= Number.EPSILON;
}

function clamp(value: number): number {
  return Math.min(1, Math.max(0, value));
}

function isUniqueConflict(error: unknown): boolean {
  return error instanceof Error && error.message.includes("UNIQUE constraint failed");
}
