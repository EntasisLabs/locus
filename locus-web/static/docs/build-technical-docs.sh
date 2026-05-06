#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DOCS_DIR="${ROOT_DIR}/docs"
BOOK_DIR="${DOCS_DIR}/book"
TECH_DIR="${DOCS_DIR}/technical"
API_DIR="${TECH_DIR}/api"

if ! command -v mdbook >/dev/null 2>&1; then
  echo "error: mdbook is not installed."
  echo "install: cargo install mdbook"
  exit 1
fi

if ! command -v mdbook-mermaid >/dev/null 2>&1; then
  echo "error: mdbook-mermaid is not installed."
  echo "install: cargo install mdbook-mermaid"
  exit 1
fi

echo "[1/4] Building mdBook..."
mdbook build "${BOOK_DIR}"

echo "[2/4] Building workspace rustdoc..."
cargo doc --workspace --no-deps

echo "[3/4] Staging API docs..."
rm -rf "${API_DIR}"
mkdir -p "${API_DIR}"
cp -R "${ROOT_DIR}/target/doc/." "${API_DIR}/"

echo "[4/4] Writing technical docs index..."
mkdir -p "${TECH_DIR}"
cat > "${TECH_DIR}/index.html" <<'HTML'
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<meta name="description" content="Locus technical documentation: mdBook guides and Rust API docs.">
<title>Locus Technical Docs</title>
<style>
:root{--bg:#070612;--panel:#131126;--line:rgba(255,255,255,.12);--txt:rgba(255,255,255,.9);--dim:rgba(255,255,255,.65);--accent:#4dbfa0;--serif:Georgia,serif;--ui:system-ui,-apple-system,Segoe UI,Roboto,sans-serif}
*{box-sizing:border-box;margin:0;padding:0}
body{font-family:var(--ui);background:radial-gradient(900px 600px at 15% -10%,rgba(77,191,160,.15),transparent),var(--bg);color:var(--txt);line-height:1.6}
.wrap{max-width:980px;margin:0 auto;padding:32px 20px 64px}
h1{font-family:var(--serif);font-size:clamp(34px,6vw,56px);line-height:1.05;margin-bottom:12px}
p.lead{font-size:18px;color:var(--dim);max-width:64ch}
.grid{margin-top:28px;display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:14px}
.card{display:block;text-decoration:none;color:inherit;background:var(--panel);border:1px solid var(--line);border-radius:10px;padding:18px;transition:transform .2s,border-color .2s}
.card:hover{transform:translateY(-2px);border-color:rgba(255,255,255,.28)}
.card h2{font-size:24px;margin-bottom:8px}
.card p{color:var(--dim)}
.list{margin-top:24px;background:var(--panel);border:1px solid var(--line);border-radius:10px;padding:18px}
.list h3{font-size:18px;margin-bottom:10px}
.list a{color:var(--accent);text-decoration:none}
.list a:hover{text-decoration:underline}
li{margin:6px 0 0 18px}
.back{margin-top:24px;display:inline-block;color:var(--dim);text-decoration:none}
.back:hover{color:var(--txt)}
@media (max-width:760px){.grid{grid-template-columns:1fr}}
</style>
</head>
<body>
  <main class="wrap">
    <h1>Locus Technical Docs</h1>
    <p class="lead">Narrative guides and API reference generated directly from source, designed for deep implementation work.</p>

    <section class="grid">
      <a class="card" href="book/index.html">
        <h2>Guides (mdBook)</h2>
        <p>Architecture, operations, integration, security, troubleshooting, and protocol references.</p>
      </a>
      <a class="card" href="api/index.html">
        <h2>API Reference (rustdoc)</h2>
        <p>Workspace crate APIs generated from Rust source docs for locus-core-rs, sdk, gateway, mcp, and cli.</p>
      </a>
    </section>

    <section class="list">
      <h3>Quick API Entrypoints</h3>
      <ul>
        <li><a href="api/locus_core_rs/index.html">locus_core_rs</a></li>
        <li><a href="api/locus_sdk/index.html">locus_sdk</a></li>
        <li><a href="api/locus_gateway/index.html">locus_gateway</a></li>
        <li><a href="api/locus_mcp/index.html">locus_mcp</a></li>
        <li><a href="api/locus_cli/index.html">locus_cli</a></li>
      </ul>
    </section>

    <a class="back" href="../index.html">Back to landing</a>
  </main>
</body>
</html>
HTML

echo "Done: ${TECH_DIR}/index.html"
echo "Next: serve docs folder, e.g. cd docs && python3 -m http.server 8080"
