<script lang="ts">
	import { resolve } from '$app/paths';
	import AppNav from '$lib/components/AppNav.svelte';
</script>

<svelte:head>
	<title>Locus Deployment and Operations | Locus</title>
	<meta
		name="description"
		content="Deployment and operations guide for Locus runtime modes, readiness checks, observability, and incident response."
	/>
</svelte:head>

<div class="reading-veil"></div>
<AppNav active="deploy" />

<main class="wrap">
	<section class="hero">
		<div class="kicker">Deployment and Operations</div>
		<h1>Run reliably. Recover fast.</h1>
		<p class="lead">
			This page condenses deployment modes, release readiness, observability priorities, and
			incident runbook patterns for production Locus usage.
		</p>
	</section>

	<section class="card" style="margin-bottom: 16px">
		<h2>Environment matrix</h2>
		<table class="table">
			<thead>
				<tr><th>Environment</th><th>Storage</th><th>Host Profile</th><th>Primary Goal</th></tr>
			</thead>
			<tbody>
				<tr
					><td>Local Development</td><td>In-memory or embedded Surreal</td><td>Single process</td
					><td>Fast iteration</td></tr
				>
				<tr
					><td>CI Validation</td><td>In-memory fixtures</td><td>Test jobs only</td><td
						>Regression detection</td
					></tr
				>
				<tr
					><td>Staging</td><td>Remote SurrealDB</td><td>Gateway and MCP parity checks</td><td
						>Release rehearsal</td
					></tr
				>
				<tr
					><td>Production</td><td>Remote SurrealDB restricted access</td><td
						>Managed gateway and MCP deploys</td
					><td>Reliability and auditability</td></tr
				>
			</tbody>
		</table>
	</section>

	<section class="grid">
		<article class="card">
			<div class="sub">Release readiness</div>
			<h2>Checklist</h2>
			<ol class="list">
				<li>Workspace compile and tests pass.</li>
				<li>SDK examples execute in smoke path.</li>
				<li>Host integrations confirm contract compatibility.</li>
				<li>Changelog and migration notes updated.</li>
				<li>Security checklist reviewed for secrets/logging hygiene.</li>
			</ol>
			<pre class="cmd">cargo check --workspace --examples
cargo test -p locus-core --lib
cargo test -p locus-sdk</pre>
		</article>
		<article class="card">
			<div class="sub">Health signals</div>
			<h2>Observe first</h2>
			<ul class="list">
				<li>Parse success ratio</li>
				<li>Validation failure ratio</li>
				<li>Recall latency percentiles</li>
				<li>Transform success/failure rates</li>
				<li>Embedding backfill throughput</li>
			</ul>
		</article>
	</section>

	<section class="stack">
		<article class="card">
			<div class="sub">Incident runbook A</div>
			<h2>Parser spike</h2>
			<ol class="list">
				<li>Capture failing payloads.</li>
				<li>Validate four-layer ordering and key format.</li>
				<li>Compare behavior with last known-good release.</li>
				<li>Rollback host version if release-correlated.</li>
			</ol>
		</article>
		<article class="card">
			<div class="sub">Incident runbook B</div>
			<h2>Retrieval regression</h2>
			<ol class="list">
				<li>Capture request payload + scoring settings + path.</li>
				<li>Run explain workflow with same request.</li>
				<li>Compare fallback behavior against baseline.</li>
				<li>Patch defaults only with regression tests.</li>
			</ol>
		</article>
		<article class="card">
			<div class="sub">Incident runbook C</div>
			<h2>Transform instability</h2>
			<ol class="list">
				<li>Switch to dry-run.</li>
				<li>Validate selected nodes and provider capabilities.</li>
				<li>Reduce batch size and rerun with checkpoints.</li>
				<li>Resume full run only after parity.</li>
			</ol>
		</article>
		<article class="card">
			<div class="sub">Deep references</div>
			<h2>Primary docs</h2>
			<div class="links">
				<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
				<a href="/docs/deployment.md">Deployment Markdown</a>
				<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
				<a href="/docs/operations.md">Operations Markdown</a>
				<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
				<a href="/docs/integration.md">Integration Markdown</a>
				<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
				<a href="/docs/security.md">Security Markdown</a>
			</div>
		</article>
	</section>
</main>

<footer class="wrap">
	Locus Deployment and Operations · Apache-2.0 · <a href={resolve('/')}>Back to landing</a> ·
	<a href={resolve('/docs')}>Docs hub</a>
</footer>

<style>
	:global(body) {
		font-family: 'Syne', 'Avenir Next', sans-serif;
		background:
			radial-gradient(1200px 600px at 80% -10%, rgba(77, 191, 160, 0.09), transparent),
			radial-gradient(900px 600px at 10% 120%, rgba(212, 148, 58, 0.09), transparent), #04030d;
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
		max-width: 1120px;
		margin: 0 auto;
		padding: 0 24px;
	}

	.hero {
		padding: 120px 0 38px;
	}

	.kicker,
	.sub {
		font-family: 'JetBrains Mono', monospace;
		font-size: 11px;
		letter-spacing: 0.18em;
		color: #4dbfa0;
		text-transform: uppercase;
	}

	.sub {
		font-size: 10px;
		letter-spacing: 0.16em;
		color: #d4943a;
		margin-bottom: 8px;
	}

	h1,
	h2 {
		font-family: 'DM Serif Display', Georgia, serif;
		line-height: 1.04;
	}

	h1 {
		font-size: clamp(38px, 6.5vw, 68px);
		margin: 12px 0 14px;
	}

	h2 {
		font-size: 32px;
		margin-bottom: 10px;
	}

	.lead {
		max-width: 64ch;
		color: rgba(255, 255, 255, 0.62);
		font-size: 18px;
	}

	.card {
		background: #0d0a20;
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: 8px;
		padding: 18px;
		backdrop-filter: blur(6px);
	}

	.table {
		width: 100%;
		border-collapse: collapse;
		font-size: 14px;
	}

	.table th,
	.table td {
		border: 1px solid rgba(255, 255, 255, 0.12);
		padding: 10px 12px;
		text-align: left;
	}

	.table th {
		color: rgba(255, 255, 255, 0.85);
		font-weight: 600;
	}

	.table td,
	.list {
		color: rgba(255, 255, 255, 0.62);
	}

	.grid {
		display: grid;
		grid-template-columns: 1.3fr 0.7fr;
		gap: 16px;
		margin-top: 26px;
	}

	.stack {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 16px;
		margin: 16px 0 46px;
	}

	.list {
		padding-left: 18px;
	}

	.list li {
		margin: 6px 0;
	}

	.cmd {
		background: #120f28;
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: 6px;
		padding: 12px;
		overflow: auto;
		font-family: 'JetBrains Mono', monospace;
		font-size: 13px;
		line-height: 1.55;
		color: rgba(255, 255, 255, 0.88);
		margin-top: 10px;
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

	@media (max-width: 900px) {
		.grid,
		.stack {
			grid-template-columns: 1fr;
		}

		.lead {
			font-size: 16px;
		}
	}
</style>
