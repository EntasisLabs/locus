# locus-sqlite-adapter

SQLite implementation of the Locus `NodeStore` and `SemanticIndexStore` contracts.

The file contains the same four tables as the Surreal schema: `temporal_node`, `calibration`, `sync_checkpoint`, and `semantic_tag_index`. Timestamps are stored as unix milliseconds so ordering does not depend on RFC3339 text width. JSON columns hold tags, links, metadata, and embeddings.

```rust
use std::sync::Arc;

use locus_core_rs::{NodeStoreInitializer, SemanticIndexStoreInitializer};
use locus_sqlite_adapter::{SqliteDatabase, SqliteNodeStore, SqliteSemanticIndexStore};

# async fn example() -> anyhow::Result<()> {
let db = Arc::new(SqliteDatabase::open("data/locus.sqlite")?);
let nodes = Arc::new(SqliteNodeStore::new(db.clone()));
let tags = Arc::new(SqliteSemanticIndexStore::new(db));
nodes.initialize_async().await?;
tags.initialize_async().await?;
# Ok(())
# }
```

Hosts select this backend with `LOCUS_STORAGE=sqlite`, `LOCUS_MCP_STORAGE=sqlite`, or `LOCUS_GATEWAY_BACKEND=sqlite`. The file path comes from `LOCUS_SQLITE_PATH`, `LOCUS_MCP_SQLITE_PATH`, or `LOCUS_GATEWAY_SQLITE_PATH`.
