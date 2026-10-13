# locus-durable-object

Cloudflare Durable Object store for Locus. One object is one SQLite database, the same boundary as a file opened by `locus-sqlite-adapter`. The tables match: `temporal_node`, `calibration`, `sync_checkpoint`, and `semantic_tag_index`.

Address the object with `env.LOCUS_STORE.getByName(storeName)`. A single global object would serialize every tenant, so production sharding should put each database on its own name. Global resonance and scope rekey run inside that one database.

HTTP entry, for hosts that are not on the same Worker:

```bash
curl -s http://localhost:8787/health
curl -s -X POST http://localhost:8787/stores/locus \
  -H 'content-type: application/json' \
  -d '{"method":"queryNodes","args":[{"sessionId":"demo","limit":10}]}'
```

The method name is one of the `LocusStore` RPC methods (`upsertNode`, `getByResonance`, `purgeSession`, `syncNodeTags`, and the rest of the node and tag surface).

```bash
npm test
npx wrangler dev
```

`npm test` runs the store against Node's built-in SQLite. It does not boot a Worker.
