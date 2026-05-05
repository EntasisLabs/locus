#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WEB_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$WEB_ROOT/.." && pwd)"

cd "$REPO_ROOT"

echo "Generating workspace Rust docs..."
cargo doc --workspace --no-deps

RUSTDOC_OUT="$WEB_ROOT/static/docs/rustdoc"
TECHNICAL_API_OUT="$WEB_ROOT/static/docs/technical/api"

echo "Syncing Rust docs into static/docs/rustdoc and static/docs/technical/api..."
mkdir -p "$RUSTDOC_OUT" "$TECHNICAL_API_OUT"
rsync -a --delete "$REPO_ROOT/target/doc/" "$RUSTDOC_OUT/"
rsync -a --delete "$REPO_ROOT/target/doc/" "$TECHNICAL_API_OUT/"

create_index_page() {
	local out_dir="$1"
	cat > "$out_dir/index.html" <<'HTML'
<!doctype html>
<html lang="en">
<head>
	<meta charset="utf-8" />
	<meta name="viewport" content="width=device-width, initial-scale=1" />
	<title>Locus Rust API Docs</title>
	<meta http-equiv="refresh" content="0; url=./locus_core/index.html" />
	<style>
		body { font-family: system-ui, -apple-system, Segoe UI, Roboto, sans-serif; margin: 2rem; line-height: 1.5; }
		h1 { margin-bottom: 0.5rem; }
		ul { padding-left: 1.25rem; }
	</style>
</head>
<body>
	<h1>Locus Rust API Docs</h1>
	<p>If you are not redirected automatically, choose a crate:</p>
	<ul>
		<li><a href="./locus_core/index.html">locus_core</a></li>
		<li><a href="./locus_sdk/index.html">locus_sdk</a></li>
		<li><a href="./locus_mcp/index.html">locus_mcp</a></li>
		<li><a href="./locus_gateway/index.html">locus_gateway</a></li>
		<li><a href="./locus_cli/index.html">locus_cli</a></li>
	</ul>
</body>
</html>
HTML
}

create_index_page "$RUSTDOC_OUT"
create_index_page "$TECHNICAL_API_OUT"

echo "Rust docs available at /docs/rustdoc/index.html"
