<script lang="ts">
	import { resolve } from '$app/paths';
	import AppNav from '$lib/components/AppNav.svelte';
</script>

<svelte:head>
	<title>Locus Quickstart | Locus</title>
	<meta
		name="description"
		content="Quickstart guide for Locus: run MCP, gateway, SDK examples, and local development flows."
	/>
</svelte:head>

<div class="reading-veil"></div>
<AppNav active="quickstart" />

<main class="wrap">
	<section class="hero">
		<div class="kicker">Quickstart</div>
		<h1>Run Locus in minutes.</h1>
		<p class="lead">
			Pick your path: containerized MCP, gateway service mode, or source-driven local development.
			Everything below is tuned for fast first success.
		</p>
	</section>

	<section class="grid">
		<article class="card">
			<h3>MCP via image</h3>
			<p>Fastest path for assistant tools and non-code workflows.</p>
			<pre
				class="cmd">docker run --rm -i -v "$PWD/locus-data:/data" ghcr.io/entasislabs/locus-mcp:0.1.0</pre>
		</article>
		<article class="card">
			<h3>Gateway via image</h3>
			<p>Run HTTP and gRPC endpoints for application integration.</p>
			<pre
				class="cmd">docker run --rm -p 8080:8080 -p 8081:8081 -v "$PWD/locus-data:/data" ghcr.io/entasislabs/locus-gateway:2.0.0</pre>
		</article>
		<article class="card">
			<h3>Source (dev)</h3>
			<p>Local runtime for implementation and debugging work.</p>
			<pre class="cmd">LOCUS_MCP_IN_MEMORY=true cargo run --manifest-path locus-mcp/Cargo.toml
cargo run --manifest-path locus-gateway/Cargo.toml</pre>
		</article>
		<article class="card">
			<h3>SDK examples</h3>
			<p>Validate core memory composition and provider patterns.</p>
			<pre class="cmd">cargo run -p locus-sdk --example provider_registry_setup
cargo run -p locus-sdk --example memory_composition
cargo run -p locus-sdk --example recursive_composite_pipeline</pre>
		</article>
	</section>

	<section class="row">
		<article class="box">
			<h2>Release-safe baseline</h2>
			<ol class="list">
				<li>Run workspace checks: <code>cargo check --workspace</code>.</li>
				<li>Run tests: <code>cargo test --workspace</code>.</li>
				<li>Build SDK examples: <code>cargo check --examples -p locus-sdk</code>.</li>
				<li>Use mounted data paths for persistence separation.</li>
			</ol>
		</article>
		<article class="box">
			<h2>Read next</h2>
			<div class="links">
				<a href={resolve('/deployment-operations')}>Deployment and Ops page</a>
				<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
				<a href="/docs/deployment.md">Deployment Markdown</a>
				<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
				<a href="/docs/examples.md">Examples Markdown</a>
				<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
				<a href="/docs/integration.md">Integration Markdown</a>
			</div>
		</article>
	</section>
</main>

<footer class="wrap">
	Locus Quickstart · Apache-2.0 · <a href={resolve('/')}>Back to landing</a> ·
	<a href={resolve('/docs')}>Docs hub</a>
</footer>

<style>
	:global(body) {
		font-family: 'Syne', 'Avenir Next', sans-serif;
		background:
			radial-gradient(1200px 500px at 20% -10%, rgba(139, 110, 196, 0.15), transparent),
			radial-gradient(900px 500px at 80% 120%, rgba(77, 191, 160, 0.1), transparent), #04030d;
		color: rgba(255, 255, 255, 0.86);
		line-height: 1.7;
	}

	.reading-veil {
		position: fixed;
		inset: 0;
		pointer-events: none;
		z-index: 0;
		background:
			radial-gradient(circle at 50% 16%, rgba(8, 9, 12, 0.14), transparent 30%),
			linear-gradient(
				180deg,
				rgba(8, 9, 12, 0.22),
				rgba(8, 9, 12, 0.36) 45%,
				rgba(8, 9, 12, 0.48) 100%
			);
	}

	main,
	footer {
		position: relative;
		z-index: 1;
	}

	.wrap {
		max-width: 1100px;
		margin: 0 auto;
		padding: 0 24px;
	}

	.hero {
		padding: 122px 0 44px;
		animation: fadeUp 0.8s ease both;
	}

	.kicker {
		font-family: 'JetBrains Mono', monospace;
		font-size: 11px;
		letter-spacing: 0.18em;
		color: #4dbfa0;
		text-transform: uppercase;
	}

	h1 {
		font-family: 'DM Serif Display', Georgia, serif;
		font-size: clamp(40px, 7vw, 72px);
		line-height: 1.02;
		margin: 12px 0 14px;
	}

	.lead {
		max-width: 62ch;
		color: rgba(255, 255, 255, 0.62);
		font-size: 18px;
	}

	.grid {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 16px;
		margin: 28px 0 50px;
		animation: fadeUp 0.8s ease both;
		animation-delay: 0.1s;
	}

	.card,
	.box {
		background: #0c0a1e;
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: 8px;
		padding: 18px;
		backdrop-filter: blur(6px);
		transition:
			transform 0.2s,
			border-color 0.2s,
			background 0.2s;
	}

	.card:hover,
	.box:hover {
		transform: translateY(-2px);
		border-color: rgba(255, 255, 255, 0.26);
		background: rgba(18, 16, 42, 0.85);
	}

	.card h3 {
		font-family: 'DM Serif Display', Georgia, serif;
		font-size: 30px;
		margin-bottom: 8px;
	}

	.card p {
		color: rgba(255, 255, 255, 0.62);
	}

	.cmd {
		margin-top: 12px;
		background: #12102a;
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: 6px;
		padding: 12px;
		overflow: auto;
		font-family: 'JetBrains Mono', monospace;
		font-size: 13px;
		line-height: 1.55;
		color: rgba(255, 255, 255, 0.88);
	}

	.row {
		display: grid;
		grid-template-columns: 1.2fr 0.8fr;
		gap: 16px;
		margin-bottom: 40px;
		animation: fadeUp 0.8s ease both;
		animation-delay: 0.2s;
	}

	.box h2 {
		font-family: 'DM Serif Display', Georgia, serif;
		font-size: 34px;
		margin-bottom: 10px;
	}

	.list {
		padding-left: 18px;
		color: rgba(255, 255, 255, 0.62);
	}

	.list li {
		margin: 6px 0;
	}

	.links a {
		display: inline-block;
		margin: 4px 10px 0 0;
		padding: 8px 10px;
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: 6px;
		text-decoration: none;
		font-size: 13px;
		color: rgba(255, 255, 255, 0.62);
	}

	.links a:hover {
		color: rgba(255, 255, 255, 0.9);
		border-color: rgba(255, 255, 255, 0.3);
	}

	footer {
		padding: 26px 24px 40px;
		border-top: 1px solid rgba(255, 255, 255, 0.12);
		color: rgba(255, 255, 255, 0.62);
		font-size: 13px;
	}

	footer a {
		color: rgba(255, 255, 255, 0.82);
		text-decoration: none;
	}

	@keyframes fadeUp {
		from {
			opacity: 0;
			transform: translateY(12px);
		}
		to {
			opacity: 1;
			transform: none;
		}
	}

	@media (max-width: 900px) {
		.grid,
		.row {
			grid-template-columns: 1fr;
		}

		.lead {
			font-size: 16px;
		}
	}
</style>
