<script lang="ts">
	import { resolve } from '$app/paths';
	import AppNav from '$lib/components/AppNav.svelte';
</script>

<svelte:head>
	<title>Locus Docs Hub | Locus</title>
	<meta
		name="description"
		content="Locus docs hub: quickstart, deployment, integration, architecture, operations, and protocol references."
	/>
</svelte:head>

<div class="reading-veil"></div>
<AppNav active="docs" />

<main class="wrap">
	<section class="hero">
		<div class="kicker">Consolidated Specifications</div>
		<h1>One page. Full signal.</h1>
		<p class="lead">
			Explore architecture, operations, and policy in one clean experience designed for fast
			understanding and confident decisions.
		</p>
	</section>

	<section class="section">
		<h2>Spec Experiences</h2>
		<div class="feature-grid">
			<article class="card">
				<div class="tag">Architecture Experience</div>
				<h3>Core Data Atlas</h3>
				<p>
					A visual walkthrough of data relationships, reliability rules, synchronization behavior,
					and how it all stays stable at scale.
				</p>
				<span class="pill">schema</span><span class="pill">idempotency</span><span class="pill"
					>sync semantics</span
				>
				<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
				<a href="/docs/core-data-atlas.html">Open data atlas</a>
			</article>
			<article class="card">
				<div class="tag">Architecture Experience</div>
				<h3>SDK Systems Map</h3>
				<p>
					An interactive map of the SDK: service flow, provider routing, fallback behavior, and
					operational guardrails.
				</p>
				<span class="pill">service topology</span><span class="pill">routing policy</span><span
					class="pill">guardrails</span
				>
				<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
				<a href="/docs/sdk-systems-map.html">Open systems map</a>
			</article>
		</div>
	</section>

	<section class="section">
		<article class="card">
			<div class="tag">Technical Documentation</div>
			<h3>Guides + API Reference</h3>
			<p>
				Open the generated technical docs site with mdBook guides and rustdoc API reference in one
				place.
			</p>
			<span class="pill">mdBook</span><span class="pill">rustdoc</span><span class="pill"
				>source-of-truth docs</span
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
				<p>
					Versioning stays predictable, with clear migration guidance whenever behavior changes.
				</p>
				<ul class="summary-list">
					<li>SemVer across core and SDK.</li>
					<li>
						Deprecated contracts remain for at least one release cycle unless safety requires
						immediate removal.
					</li>
					<li>Behavior changes are validated before release through repeatable checks.</li>
				</ul>
			</article>
			<article class="card">
				<div class="tag">Security</div>
				<h3>Data Handling and Safety</h3>
				<p>
					Security is default-first: sensitive data, scoped access, and clean operational hygiene.
				</p>
				<ul class="summary-list">
					<li>No secrets in source control; rotate on incident or role change.</li>
					<li>Minimize and redact sensitive payloads before storage when possible.</li>
					<li>Use TLS transports and least-privilege DB access.</li>
				</ul>
			</article>
			<article class="card">
				<div class="tag">Deployment</div>
				<h3>Runtime and Release Readiness</h3>
				<p>
					Deploy with clear environment profiles and strict release gates before production rollout.
				</p>
				<ul class="summary-list">
					<li>Local, CI, staging, and production runtime matrix is explicit.</li>
					<li>Workspace compile/tests and integration parity are release gates.</li>
					<li>Scope and provider settings stay explicit per environment.</li>
				</ul>
			</article>
		</div>
	</section>

	<section id="specs-register" class="section" style="margin-bottom: 36px">
		<div class="register">
			<article class="card">
				<div class="tag">Protocol</div>
				<h3>STTP and Structure</h3>
				<p>
					Layer order, typed fields, and confidence signals create a reliable memory contract across
					systems.
				</p>
			</article>
			<article class="card">
				<div class="tag">Integration</div>
				<h3>Host Contracts</h3>
				<p>Host-facing contracts stay stable while internals evolve safely and incrementally.</p>
			</article>
			<article class="card">
				<div class="tag">Operations</div>
				<h3>Runbook Baseline</h3>
				<p>
					Track ingestion health, retrieval latency, transform outcomes, and sync continuity to keep
					operations steady.
				</p>
			</article>
			<article class="card">
				<div class="tag">Troubleshooting</div>
				<h3>Failure Discipline</h3>
				<p>Contain quickly, isolate scope, verify behavior, and ship fixes with confidence.</p>
			</article>
			<article class="card">
				<div class="tag">Examples</div>
				<h3>Implementation Patterns</h3>
				<p>Reference workflows make advanced usage repeatable across teams and environments.</p>
			</article>
		</div>
	</section>
</main>

<footer class="wrap">
	Locus Docs Hub · Apache-2.0 · <a href={resolve('/')}>Back to landing</a>
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
