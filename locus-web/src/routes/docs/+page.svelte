<script lang="ts">
	import { resolve } from '$app/paths';
	import AppNav from '$lib/components/AppNav.svelte';
</script>

<svelte:head>
	<title>Documentation | Locus</title>
	<meta
		name="description"
		content="Locus documentation: how memory is stored, how the SDK is layered, and the guides for deploy, security, and STTP."
	/>
</svelte:head>

<div class="reading-veil"></div>
<AppNav active="docs" />

<main class="wrap">
	<section class="hero">
		<div class="kicker">Documentation</div>
		<h1>Find the page you need.</h1>
		<p class="lead">
			Start with a picture of how memory is stored, or how an SDK call moves through the stack. The
			guides below are the written versions of the same system.
		</p>
	</section>

	<section class="section" id="using-memory">
		<h2>Six operations</h2>
		<p class="lead">
			These are the ways Locus reads and writes memory. The same six exist on MCP, HTTP, gRPC, the
			SDK, and the CLI.
		</p>
		<div class="register">
			<article class="card">
				<h3>find</h3>
				<p>Filter, sort, and page through memories. No ranking. You name the scope.</p>
			</article>
			<article class="card">
				<h3>recall</h3>
				<p>Rank memories by how well they fit the current situation, not only by keyword.</p>
			</article>
			<article class="card">
				<h3>aggregate</h3>
				<p>Group memories across a time window and roll them into summaries.</p>
			</article>
			<article class="card">
				<h3>transform</h3>
				<p>Change many memories at once. Dry-run first when you want a preview.</p>
			</article>
			<article class="card">
				<h3>explain</h3>
				<p>Show why a memory was returned, or why it was not.</p>
			</article>
			<article class="card">
				<h3>schema</h3>
				<p>Ask the running system what it supports, so tools do not hardcode assumptions.</p>
			</article>
		</div>
	</section>

	<section class="section">
		<h2>A question in, one operation out</h2>
		<div class="register">
			<article class="card">
				<h3>find</h3>
				<p>"What did we work on last Tuesday?" Filter to that day.</p>
			</article>
			<article class="card">
				<h3>recall</h3>
				<p>"What is relevant to what I am doing right now?" Rank by fit, not just keywords.</p>
			</article>
			<article class="card">
				<h3>aggregate</h3>
				<p>"Summarize last month." Roll the sessions into one digest.</p>
			</article>
			<article class="card">
				<h3>explain</h3>
				<p>"Why that memory, and not the one from Thursday?" Show the decision.</p>
			</article>
			<article class="card">
				<h3>transform</h3>
				<p>"Preview an embedding backfill before writing." Dry-run first.</p>
			</article>
		</div>
	</section>

	<section class="section">
		<h2>Remembered is not the same as true</h2>
		<p class="lead">
			Locus does not make an AI human. A memory can be recent or old, strongly or weakly supported,
			useful or irrelevant, confirmed, uncertain, or outdated.
		</p>
		<article class="card">
			<p>
				Each memory stores four scores from 0 to 1: stability, friction, logic, and autonomy.
				Together they are an AVEC. Recall ranks by how close they are to the current situation. Psi
				is computed from them, so another system can check coherence without the original history.
			</p>
		</article>
	</section>

	<section class="section">
		<h2>Where it sits</h2>
		<p class="lead">
			Your app, Resonantia, or any STTP agent talks to Locus. Locus stores the memory. STTP is the
			record format underneath.
		</p>
	</section>

	<section class="section">
		<h2>Visual guides</h2>
		<div class="feature-grid">
			<article class="card">
				<div class="tag">Data model</div>
				<h3>How Locus stores memory</h3>
				<p>
					The records, the rules that keep a replay from writing duplicates, and how sync picks up
					where it left off.
				</p>
				<span class="pill">schema</span><span class="pill">idempotency</span><span class="pill"
					>sync</span
				>
				<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
				<a href="/docs/core-data-atlas.html">Open the data model</a>
			</article>
			<article class="card">
				<div class="tag">SDK</div>
				<h3>How a call moves</h3>
				<p>
					The SDK layers, how a provider is chosen, what the fallback does, and which limits stay on
					in production.
				</p>
				<span class="pill">layers</span><span class="pill">routing</span><span class="pill"
					>fallbacks</span
				>
				<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
				<a href="/docs/sdk-systems-map.html">Open the SDK map</a>
			</article>
		</div>
	</section>

	<section class="section">
		<article class="card">
			<div class="tag">Technical Documentation</div>
			<h3>Guides + API Reference</h3>
			<p>Guides and the Rust API reference, generated from the repo.</p>
			<span class="pill">guides</span><span class="pill">rustdoc</span><span class="pill"
				>API reference</span
			>
			<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
			<a href="/docs/technical/index.html">Open technical docs site</a>
		</article>
	</section>

	<section class="section">
		<div class="mini-grid">
			<article class="card">
				<div class="tag">Compatibility</div>
				<h3>Versioning Policy</h3>
				<p>What a version change means, and how long an old contract stays available.</p>
				<ul class="summary-list">
					<li>SemVer across core and SDK.</li>
					<li>
						Deprecated contracts remain for at least one release cycle unless safety requires
						immediate removal.
					</li>
					<li>Behavior changes are validated before release through repeatable checks.</li>
				</ul>
				<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
				<a href="/docs/versioning.md">Versioning guide</a>
			</article>
			<article class="card">
				<div class="tag">Security</div>
				<h3>Data Handling and Safety</h3>
				<p>Secrets, redaction, transport, and who can reach the database.</p>
				<ul class="summary-list">
					<li>No secrets in source control; rotate on incident or role change.</li>
					<li>Minimize and redact sensitive payloads before storage when possible.</li>
					<li>Use TLS transports and least-privilege DB access.</li>
				</ul>
				<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
				<a href="/docs/security.md">Security guide</a>
			</article>
			<article class="card">
				<div class="tag">Deployment</div>
				<h3>Runtime and Release Readiness</h3>
				<p>Local, CI, staging, and production, and what has to pass before a release.</p>
				<ul class="summary-list">
					<li>Local, CI, staging, and production runtime matrix is explicit.</li>
					<li>Workspace compile/tests and integration parity are release gates.</li>
					<li>Scope and provider settings stay explicit per environment.</li>
				</ul>
				<a href={resolve('/deployment-operations')}>Deployment and operations</a>
			</article>
		</div>
	</section>

	<section id="specs-register" class="section" style="margin-bottom: 36px">
		<div class="register">
			<article class="card">
				<div class="tag">Protocol</div>
				<h3>STTP record format</h3>
				<p>
					The four parts of a memory, the order they appear in, and the scores stored with each one.
				</p>
				<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
				<a href="/docs/sttp_typed_ir_language_spec.md">STTP language spec</a>
			</article>
			<article class="card">
				<div class="tag">Integration</div>
				<h3>Host contracts</h3>
				<p>What stays stable for the systems that call Locus while the internals change.</p>
				<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
				<a href="/docs/integration.md">Integration guide</a>
			</article>
			<article class="card">
				<div class="tag">Operations</div>
				<h3>What to watch</h3>
				<p>
					Ingestion health, retrieval latency, transform results, and whether sync is still moving.
				</p>
				<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
				<a href="/docs/operations.md">Operations guide</a>
			</article>
			<article class="card">
				<div class="tag">Troubleshooting</div>
				<h3>When something fails</h3>
				<p>Contain it, isolate the scope, check the behavior, then ship the fix.</p>
				<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
				<a href="/docs/troubleshooting.md">Troubleshooting guide</a>
			</article>
			<article class="card">
				<div class="tag">Examples</div>
				<h3>Worked examples</h3>
				<p>Reference workflows you can copy for common memory tasks.</p>
				<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
				<a href="/docs/examples.md">Examples</a>
			</article>
		</div>
	</section>
</main>

<footer class="wrap">
	Locus Docs · Apache-2.0 · <a href={resolve('/')}>Back to landing</a> ·
	<a href="https://resonantia.me" target="_blank" rel="noopener noreferrer">Resonantia</a>
</footer>

<style>
	:global(body) {
		font-family: 'Syne', 'Avenir Next', sans-serif;
		background:
			radial-gradient(1200px 600px at 20% -10%, rgba(77, 191, 160, 0.1), transparent),
			radial-gradient(900px 600px at 90% 120%, rgba(212, 148, 58, 0.09), transparent), #04030d;
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
	.tag {
		font-family: 'JetBrains Mono', monospace;
		font-size: 11px;
		letter-spacing: 0.18em;
		color: #4dbfa0;
		text-transform: uppercase;
	}

	.tag {
		font-size: 10px;
		letter-spacing: 0.14em;
		color: #d4943a;
	}

	h1,
	h2,
	h3 {
		font-family: 'DM Serif Display', Georgia, serif;
	}

	h1 {
		font-size: clamp(38px, 6.5vw, 68px);
		line-height: 1.04;
		margin: 12px 0 14px;
	}

	h2 {
		font-size: 30px;
		margin-bottom: 10px;
	}

	h3 {
		font-size: 24px;
		margin: 7px 0 8px;
	}

	.lead {
		max-width: 64ch;
		color: rgba(255, 255, 255, 0.62);
		font-size: 18px;
	}

	.section {
		margin-top: 18px;
		margin-bottom: 20px;
	}

	.feature-grid {
		display: grid;
		grid-template-columns: 1.2fr 0.8fr;
		gap: 14px;
	}

	.mini-grid {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: 14px;
	}

	.register {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 14px;
	}

	.card {
		background: #0d0a20;
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: 8px;
		padding: 16px;
		backdrop-filter: blur(6px);
		transition:
			transform 0.2s,
			border-color 0.2s,
			background 0.2s;
	}

	.card:hover {
		transform: translateY(-2px);
		border-color: rgba(255, 255, 255, 0.26);
		background: rgba(18, 16, 42, 0.85);
	}

	.card p,
	.summary-list {
		font-size: 14px;
		color: rgba(255, 255, 255, 0.62);
		line-height: 1.68;
	}

	.card a {
		display: inline-block;
		margin-top: 12px;
		padding: 8px 10px;
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: 6px;
		text-decoration: none;
		font-size: 12px;
		color: rgba(255, 255, 255, 0.62);
	}

	.card a:hover {
		color: rgba(255, 255, 255, 0.9);
		border-color: rgba(255, 255, 255, 0.3);
	}

	.pill {
		display: inline-block;
		margin-top: 8px;
		margin-right: 8px;
		padding: 4px 8px;
		border: 1px solid rgba(255, 255, 255, 0.12);
		border-radius: 999px;
		font-family: 'JetBrains Mono', monospace;
		font-size: 10px;
		letter-spacing: 0.08em;
		color: rgba(255, 255, 255, 0.62);
	}

	.summary-list {
		padding-left: 18px;
	}

	.summary-list li {
		margin: 5px 0;
	}

	footer {
		padding: 26px 24px 40px;
		border-top: 1px solid rgba(255, 255, 255, 0.12);
		color: rgba(255, 255, 255, 0.62);
		font-size: 13px;
		margin-top: 34px;
	}

	footer a {
		color: rgba(255, 255, 255, 0.82);
		text-decoration: none;
	}

	@media (max-width: 980px) {
		.feature-grid {
			grid-template-columns: 1fr;
		}

		.mini-grid,
		.register {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}

		.lead {
			font-size: 16px;
		}
	}

	@media (max-width: 720px) {
		.mini-grid,
		.register {
			grid-template-columns: 1fr;
		}
	}
</style>
