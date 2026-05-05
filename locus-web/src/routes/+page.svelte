<script lang="ts">
	import { resolve } from '$app/paths';
	import { onMount } from 'svelte';
	import AppNav from '$lib/components/AppNav.svelte';

	type Star = {
		x: number;
		y: number;
		r: number;
		a: number;
		tw: number;
		ts: number;
	};

	const technicalDocsHref = '/docs/technical/index.html';
	const rustdocHref = '/docs/rustdoc/index.html';
	const githubHref = 'https://github.com/entasislabs/locus';
	const resonantiaHref = 'https://resonantia.me';

	let starfieldCanvas: HTMLCanvasElement;

	onMount(() => {
		const canvas = starfieldCanvas;
		const ctx = canvas?.getContext('2d');
		if (!ctx) return;

		let width = 0;
		let height = 0;
		let stars: Star[] = [];
		let frameId: number | null = null;
		const reducedMotionQuery = window.matchMedia('(prefers-reduced-motion: reduce)');

		const resize = () => {
			width = canvas.width = window.innerWidth;
			height = canvas.height = window.innerHeight;
		};

		const makeStars = () => {
			stars = Array.from({ length: 180 }, () => ({
				x: Math.random() * width,
				y: Math.random() * height,
				r: Math.random() * 1.1,
				a: Math.random() * 0.55 + 0.08,
				tw: Math.random() * Math.PI * 2,
				ts: Math.random() * 0.008 + 0.003
			}));
		};

		const draw = () => {
			ctx.clearRect(0, 0, width, height);
			for (const star of stars) {
				star.tw += star.ts;
				const alpha = star.a * (0.55 + 0.45 * Math.sin(star.tw));
				ctx.beginPath();
				ctx.arc(star.x, star.y, star.r, 0, Math.PI * 2);
				ctx.fillStyle = `rgba(255,255,255,${alpha})`;
				ctx.fill();
			}
			frameId = requestAnimationFrame(draw);
		};

		const startStarfield = () => {
			if (reducedMotionQuery.matches) return;
			resize();
			makeStars();
			if (frameId === null) draw();
		};

		const stopStarfield = () => {
			if (frameId !== null) {
				cancelAnimationFrame(frameId);
				frameId = null;
			}
			ctx.clearRect(0, 0, width, height);
		};

		const handleResize = () => {
			resize();
			if (!reducedMotionQuery.matches) makeStars();
		};

		const handleReducedMotionChange = (event: MediaQueryListEvent) => {
			if (event.matches) {
				stopStarfield();
			} else {
				startStarfield();
			}
		};

		startStarfield();
		window.addEventListener('resize', handleResize);
		reducedMotionQuery.addEventListener('change', handleReducedMotionChange);

		const revealNodes = document.querySelectorAll('.sr');
		let observer: IntersectionObserver | null = null;
		if (reducedMotionQuery.matches) {
			revealNodes.forEach((node) => node.classList.add('in'));
		} else {
			observer = new IntersectionObserver(
				(entries) => {
					for (const entry of entries) {
						if (entry.isIntersecting) {
							entry.target.classList.add('in');
							observer?.unobserve(entry.target);
						}
					}
				},
				{ threshold: 0.1 }
			);
			revealNodes.forEach((node) => observer?.observe(node));
		}

		return () => {
			window.removeEventListener('resize', handleResize);
			reducedMotionQuery.removeEventListener('change', handleReducedMotionChange);
			observer?.disconnect();
			stopStarfield();
		};
	});
</script>

<svelte:head>
	<title>Locus — The memory layer for STTP agents</title>
	<meta
		name="description"
		content="Locus is the memory layer for STTP agents: typed, persistent, and verifiable context across sessions, models, and transports."
	/>
	<meta property="og:type" content="website" />
	<meta property="og:site_name" content="Locus" />
	<meta property="og:title" content="Locus — The memory layer for STTP agents" />
	<meta
		property="og:description"
		content="Typed, persistent, and verifiable memory infrastructure for STTP agents."
	/>
	<meta property="og:image" content="/locus_final_transparent.png" />
	<meta name="twitter:card" content="summary_large_image" />
	<meta name="twitter:title" content="Locus — The memory layer for STTP agents" />
	<meta
		name="twitter:description"
		content="Typed, persistent, and verifiable memory infrastructure for STTP agents."
	/>
	<meta name="twitter:image" content="/locus_final_transparent.png" />
	<link rel="icon" type="image/png" href="/locus_final_transparent.png" />
	<link rel="preconnect" href="https://fonts.googleapis.com" />
	<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous" />
	<link
		href="https://fonts.googleapis.com/css2?family=DM+Serif+Display:ital@0;1&family=Syne:wght@400;500;600;700;800&family=JetBrains+Mono:wght@300;400&display=swap"
		rel="stylesheet"
	/>
</svelte:head>

<canvas id="sf" bind:this={starfieldCanvas}></canvas>
<div class="nb"></div>
<AppNav active="home" />

<main>
	<section class="hero">
		<div
			class="hero-logo-wrap"
			style="opacity:1;animation:fu 1s .1s ease forwards;position:relative;display:inline-block;margin-bottom:8px"
		>
			<div
				style="position:absolute;inset:0;margin:auto;border-radius:50%;background:radial-gradient(ellipse,rgba(139,110,196,.28) 0%,rgba(77,191,160,.1) 50%,transparent 70%);filter:blur(48px);pointer-events:none"
			></div>
			<img
				src="/locus_final_transparent.png"
				alt="Locus"
				class="hero-logo"
				style="width:480px;height:480px;object-fit:contain;position:relative;z-index:1;filter:drop-shadow(0 0 60px rgba(139,110,196,.35)) drop-shadow(0 0 120px rgba(77,191,160,.15))"
			/>
		</div>
		<span class="hl" style="animation-delay:.3s">STTP memory infrastructure · Apache-2.0</span>
		<h1 class="ht" style="animation-delay:.5s">
			The ground<br />beneath<br /><em>the signal.</em>
		</h1>
		<p class="hs" style="animation-delay:.75s">
			Agents are stateless. Context evaporates. Locus gives cognitive state a coordinate - typed,
			persistent, and verifiable across every session, model, and transport.
		</p>
		<div class="ha" style="animation-delay:1s">
			<a href="#protocol" class="btn bp">See the architecture</a>
			<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
			<a href={technicalDocsHref} class="btn bg">Technical docs</a>
			<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
			<a href={rustdocHref} class="btn bg">Rust API docs</a>
			<a href={githubHref} class="btn bg" target="_blank" rel="noopener noreferrer">GitHub</a>
		</div>
	</section>

	<section class="ss" style="padding:120px 0 130px">
		<div class="wrap">
			<div class="tc">
				<div class="sr">
					<span class="ml">What Locus is</span>
					<h2 class="dh">Not a product.<br />Infrastructure.</h2>
					<p class="bp2" style="margin-bottom:22px">
						Locus is the standalone memory layer for the STTP protocol. It handles storage,
						retrieval, validation, and transport - so everything built on top can treat memory as a
						solved problem.
					</p>
					<p class="bp2">
						MCP server for assistants. HTTP/gRPC gateway for services. Rust SDK for in-process
						embedding. CLI for operators. One contract, every surface.
					</p>
				</div>
				<div class="sr" style="transition-delay:.15s">
					<div style="display:flex;flex-direction:column;gap:1px">
						<div class="stack-row">
							<span class="stack-key mcp">locus-mcp</span>
							<span class="stack-value">stdio · MCP tools for assistants + agents</span>
						</div>
						<div class="stack-row">
							<span class="stack-key gateway">locus-gateway</span>
							<span class="stack-value">HTTP + gRPC · deployable host for services</span>
						</div>
						<div class="stack-row">
							<span class="stack-key sdk">locus-sdk</span>
							<span class="stack-value">Rust · transport-agnostic in-process memory</span>
						</div>
						<div class="stack-row">
							<span class="stack-key cli">locus-cli</span>
							<span class="stack-value">terminal · operator workflows, no service code</span>
						</div>
					</div>
					<div style="margin-top:14px;text-align:center">
						<span class="stack-footer"
							>all backed by → locus-core · parser · validator · retrieval</span
						>
					</div>
				</div>
			</div>
		</div>
	</section>

	<section class="ss" id="protocol" style="padding:120px 0 140px">
		<div class="wrap">
			<div class="sr center-copy" style="margin-bottom:72px">
				<span class="ml" style="display:block">The protocol</span>
				<h2 class="dh">Every node is self-sufficient.</h2>
				<p class="bp2">
					Four ordered layers. Layer order is semantic - reordering alters meaning. Every field
					typed. Every confidence annotated. A stateless receiver needs nothing else.
				</p>
			</div>
			<div class="dc sr diagram-xl desktop-geom" style="transition-delay:.1s">
				<svg width="100%" viewBox="0 0 1020 290" style="display:block">
					<defs>
						<marker
							id="aw"
							viewBox="0 0 10 10"
							refX="8"
							refY="5"
							markerWidth="5"
							markerHeight="5"
							orient="auto"
						>
							<path
								d="M2 1L8 5L2 9"
								fill="none"
								stroke="rgba(255,255,255,.2)"
								stroke-width="1.5"
								stroke-linecap="round"
							/>
						</marker>
					</defs>
					<rect
						x="14"
						y="28"
						width="228"
						height="234"
						rx="3"
						fill="rgba(77,191,160,.05)"
						stroke="rgba(77,191,160,.28)"
						stroke-width="1"
					/>
					<rect x="14" y="28" width="228" height="2.5" fill="rgba(77,191,160,.55)" />
					<text
						x="30"
						y="52"
						font-family="'JetBrains Mono'"
						font-size="10"
						fill="rgba(77,191,160,.75)"
						letter-spacing=".1em">⊕⟨ PROVENANCE</text
					>
					<text x="30" y="80" font-family="'Syne'" font-size="11" fill="rgba(255,255,255,.3)"
						>trigger</text
					>
					<text
						x="110"
						y="80"
						font-family="'JetBrains Mono'"
						font-size="11"
						fill="rgba(77,191,160,.65)">manual</text
					>
					<text x="30" y="102" font-family="'Syne'" font-size="11" fill="rgba(255,255,255,.3)"
						>format</text
					>
					<text
						x="110"
						y="102"
						font-family="'JetBrains Mono'"
						font-size="10"
						fill="rgba(77,191,160,.65)">temporal_node</text
					>
					<text x="30" y="124" font-family="'Syne'" font-size="11" fill="rgba(255,255,255,.3)"
						>session</text
					>
					<text
						x="110"
						y="124"
						font-family="'JetBrains Mono'"
						font-size="11"
						fill="rgba(255,255,255,.38)">"abc-123"</text
					>
					<text x="30" y="155" font-family="'Syne'" font-size="10" fill="rgba(255,255,255,.18)"
						>attractor_config &#123;</text
					>
					<text
						x="44"
						y="173"
						font-family="'JetBrains Mono'"
						font-size="10"
						fill="rgba(255,255,255,.25)">stability: 0.90</text
					>
					<text
						x="44"
						y="190"
						font-family="'JetBrains Mono'"
						font-size="10"
						fill="rgba(255,255,255,.25)">friction: 0.20</text
					>
					<text
						x="44"
						y="207"
						font-family="'JetBrains Mono'"
						font-size="10"
						fill="rgba(255,255,255,.25)">logic: 0.98</text
					>
					<text
						x="44"
						y="224"
						font-family="'JetBrains Mono'"
						font-size="10"
						fill="rgba(255,255,255,.25)">autonomy: 0.85</text
					>
					<text x="30" y="242" font-family="'Syne'" font-size="10" fill="rgba(255,255,255,.18)"
						>&#125;</text
					>
					<text
						x="126"
						y="275"
						text-anchor="middle"
						font-family="'Syne'"
						font-size="9"
						fill="rgba(77,191,160,.4)"
						letter-spacing=".1em">ORIENTATION</text
					>
					<line
						x1="244"
						y1="145"
						x2="264"
						y2="145"
						stroke="rgba(255,255,255,.18)"
						stroke-width="1"
						marker-end="url(#aw)"
					/>
					<rect
						x="267"
						y="28"
						width="210"
						height="234"
						rx="3"
						fill="rgba(80,144,208,.05)"
						stroke="rgba(80,144,208,.28)"
						stroke-width="1"
					/>
					<rect x="267" y="28" width="210" height="2.5" fill="rgba(80,144,208,.5)" />
					<text
						x="283"
						y="52"
						font-family="'JetBrains Mono'"
						font-size="10"
						fill="rgba(80,144,208,.85)"
						letter-spacing=".1em">⦿⟨ ENVELOPE</text
					>
					<text x="283" y="80" font-family="'Syne'" font-size="11" fill="rgba(255,255,255,.3)"
						>timestamp</text
					>
					<text
						x="283"
						y="97"
						font-family="'JetBrains Mono'"
						font-size="10"
						fill="rgba(80,144,208,.6)">"2026-04-25T…"</text
					>
					<text x="283" y="120" font-family="'Syne'" font-size="11" fill="rgba(255,255,255,.3)"
						>tier</text
					>
					<text
						x="323"
						y="120"
						font-family="'JetBrains Mono'"
						font-size="11"
						fill="rgba(80,144,208,.75)">raw</text
					>
					<text x="283" y="148" font-family="'Syne'" font-size="10" fill="rgba(255,255,255,.18)"
						>user_avec &#123;</text
					>
					<text
						x="297"
						y="166"
						font-family="'JetBrains Mono'"
						font-size="10"
						fill="rgba(255,255,255,.25)">stability: 0.90</text
					>
					<text
						x="297"
						y="183"
						font-family="'JetBrains Mono'"
						font-size="10"
						fill="rgba(255,255,255,.25)">friction: 0.20</text
					>
					<text
						x="297"
						y="200"
						font-family="'JetBrains Mono'"
						font-size="10"
						fill="rgba(255,255,255,.25)">psi: 2.93</text
					>
					<text x="283" y="218" font-family="'Syne'" font-size="10" fill="rgba(255,255,255,.18)"
						>&#125;</text
					>
					<text
						x="372"
						y="275"
						text-anchor="middle"
						font-family="'Syne'"
						font-size="9"
						fill="rgba(80,144,208,.4)"
						letter-spacing=".1em">IDENTITY</text
					>
					<line
						x1="479"
						y1="145"
						x2="499"
						y2="145"
						stroke="rgba(255,255,255,.18)"
						stroke-width="1"
						marker-end="url(#aw)"
					/>
					<rect
						x="502"
						y="28"
						width="220"
						height="234"
						rx="3"
						fill="rgba(196,96,128,.05)"
						stroke="rgba(196,96,128,.28)"
						stroke-width="1"
					/>
					<rect x="502" y="28" width="220" height="2.5" fill="rgba(196,96,128,.5)" />
					<text
						x="518"
						y="52"
						font-family="'JetBrains Mono'"
						font-size="10"
						fill="rgba(196,96,128,.85)"
						letter-spacing=".1em">◈⟨ CONTENT</text
					>
					<text x="518" y="82" font-family="'Syne'" font-size="11" fill="rgba(255,255,255,.3)"
						>confidence-annotated fields:</text
					>
					<rect
						x="518"
						y="95"
						width="188"
						height="30"
						rx="2"
						fill="rgba(196,96,128,.07)"
						stroke="rgba(196,96,128,.2)"
						stroke-width="1"
					/>
					<text
						x="530"
						y="115"
						font-family="'JetBrains Mono'"
						font-size="11"
						fill="rgba(216,130,158,.85)">focus(.99): "parser"</text
					>
					<rect
						x="518"
						y="133"
						width="188"
						height="30"
						rx="2"
						fill="rgba(196,96,128,.07)"
						stroke="rgba(196,96,128,.2)"
						stroke-width="1"
					/>
					<text
						x="530"
						y="153"
						font-family="'JetBrains Mono'"
						font-size="11"
						fill="rgba(216,130,158,.85)">decision(.96): &#123;…&#125;</text
					>
					<rect
						x="518"
						y="171"
						width="188"
						height="30"
						rx="2"
						fill="rgba(196,96,128,.07)"
						stroke="rgba(196,96,128,.2)"
						stroke-width="1"
					/>
					<text
						x="530"
						y="191"
						font-family="'JetBrains Mono'"
						font-size="11"
						fill="rgba(216,130,158,.85)">insight(.88): "…"</text
					>
					<text x="518" y="222" font-family="'Syne'" font-size="10" fill="rgba(255,255,255,.18)"
						>confidence ∈ [0.0, 1.0] · max depth 5</text
					>
					<text
						x="612"
						y="275"
						text-anchor="middle"
						font-family="'Syne'"
						font-size="9"
						fill="rgba(196,96,128,.4)"
						letter-spacing=".1em">MEANING</text
					>
					<line
						x1="724"
						y1="145"
						x2="744"
						y2="145"
						stroke="rgba(255,255,255,.18)"
						stroke-width="1"
						marker-end="url(#aw)"
					/>
					<rect
						x="747"
						y="28"
						width="258"
						height="234"
						rx="3"
						fill="rgba(212,148,58,.05)"
						stroke="rgba(212,148,58,.28)"
						stroke-width="1"
					/>
					<rect x="747" y="28" width="258" height="2.5" fill="rgba(212,148,58,.5)" />
					<text
						x="763"
						y="52"
						font-family="'JetBrains Mono'"
						font-size="10"
						fill="rgba(212,148,58,.85)"
						letter-spacing=".1em">⍉⟨ METRICS</text
					>
					<text x="763" y="80" font-family="'Syne'" font-size="11" fill="rgba(255,255,255,.3)"
						>signal quality</text
					>
					<text
						x="763"
						y="100"
						font-family="'JetBrains Mono'"
						font-size="11"
						fill="rgba(212,148,58,.75)">rho: 0.95</text
					>
					<text
						x="763"
						y="118"
						font-family="'JetBrains Mono'"
						font-size="11"
						fill="rgba(212,148,58,.75)">kappa: 0.94</text
					>
					<text
						x="763"
						y="136"
						font-family="'JetBrains Mono'"
						font-size="11"
						fill="rgba(212,148,58,.75)">psi: 2.93</text
					>
					<text x="763" y="162" font-family="'Syne'" font-size="10" fill="rgba(255,255,255,.18)"
						>compression_avec &#123;</text
					>
					<text
						x="777"
						y="180"
						font-family="'JetBrains Mono'"
						font-size="10"
						fill="rgba(255,255,255,.25)">stability: 0.90</text
					>
					<text
						x="777"
						y="197"
						font-family="'JetBrains Mono'"
						font-size="10"
						fill="rgba(255,255,255,.25)">psi: 2.93</text
					>
					<text x="763" y="215" font-family="'Syne'" font-size="10" fill="rgba(255,255,255,.18)"
						>&#125;</text
					>
					<text
						x="876"
						y="275"
						text-anchor="middle"
						font-family="'Syne'"
						font-size="9"
						fill="rgba(212,148,58,.4)"
						letter-spacing=".1em">VERIFICATION</text
					>
				</svg>
			</div>
			<div class="dc sr mobile-geom" style="transition-delay:.1s">
				<div class="stack-diagram">
					<div class="stack-node" style="border-color:rgba(77,191,160,.35)">
						<div class="k" style="color:rgba(77,191,160,.8)">⊕ PROVENANCE</div>
						<div class="t">
							Trigger, response contract, lineage, and attractor baseline. Defines orientation for
							everything that follows.
						</div>
					</div>
					<div class="stack-arrow">↓</div>
					<div class="stack-node" style="border-color:rgba(80,144,208,.35)">
						<div class="k" style="color:rgba(80,144,208,.85)">⦿ ENVELOPE</div>
						<div class="t">
							Timestamp, tier, session identity, and dual AVEC state. Anchors the node in time and
							actor context.
						</div>
					</div>
					<div class="stack-arrow">↓</div>
					<div class="stack-node" style="border-color:rgba(196,96,128,.35)">
						<div class="k" style="color:rgba(196,96,128,.85)">◈ CONTENT</div>
						<div class="t">
							Confidence-annotated meaning payload. Typed fields preserve semantic structure across
							transport and time.
						</div>
					</div>
					<div class="stack-arrow">↓</div>
					<div class="stack-node" style="border-color:rgba(212,148,58,.35)">
						<div class="k" style="color:rgba(212,148,58,.85)">⍉ METRICS</div>
						<div class="t">
							Signal quality and compression profile (ρ, κ, ψ). Enables verification and drift-aware
							recall behavior.
						</div>
					</div>
				</div>
			</div>
			<p
				class="sr"
				style="margin-top:22px;font-family:var(--fm);font-size:11px;color:var(--text-faint);letter-spacing:.06em;transition-delay:.2s"
			>
				strict profile fails closed · tolerant profile recovers with diagnostics · additive
				evolution - old nodes always survive
			</p>
		</div>
	</section>

	<section class="ss" id="primitives" style="padding:120px 0 140px">
		<div class="wrap">
			<div class="center-copy sr" style="margin-bottom:48px">
				<span class="ml" style="display:block">Six primitives</span>
				<h2 class="dh">Like NAND gates - compose into anything.</h2>
				<p class="bp2">
					Each primitive is deterministic, transport-neutral, and policy-explicit. Combine them into
					any memory workflow.
				</p>
			</div>
			<div class="pg sr" style="transition-delay:.2s">
				<div class="pc">
					<div class="pn" style="color:rgba(77,191,160,.65)">01 · deterministic</div>
					<div class="pname">find</div>
					<p class="pdesc">Filter, sort, and paginate nodes with explicit scope.</p>
				</div>
				<div class="pc">
					<div class="pn" style="color:rgba(139,110,196,.7)">02 · resonant</div>
					<div class="pname">recall</div>
					<p class="pdesc">AVEC-driven ranked retrieval by current cognitive state.</p>
				</div>
				<div class="pc">
					<div class="pn" style="color:rgba(80,144,208,.7)">03 · temporal</div>
					<div class="pname">aggregate</div>
					<p class="pdesc">Grouped statistics and rollups across timeline windows.</p>
				</div>
				<div class="pc">
					<div class="pn" style="color:rgba(196,96,128,.7)">04 · controlled</div>
					<div class="pname">transform</div>
					<p class="pdesc">Bulk mutation with dry-run support and explicit boundaries.</p>
				</div>
				<div class="pc">
					<div class="pn" style="color:rgba(212,148,58,.7)">05 · transparent</div>
					<div class="pname">explain</div>
					<p class="pdesc">Stage-level visibility into retrieval decisions.</p>
				</div>
				<div class="pc">
					<div class="pn" style="color:rgba(255,255,255,.28)">06 · introspective</div>
					<div class="pname">schema</div>
					<p class="pdesc">Runtime capability discovery for dynamic planners and UIs.</p>
				</div>
			</div>
		</div>
	</section>

	<section class="ss" id="flow" style="padding:120px 0 140px">
		<div class="wrap">
			<div class="sr center-copy" style="margin-bottom:72px">
				<span class="ml" style="display:block">Memory in motion</span>
				<h2 class="dh">
					Context doesn't disappear.<br />It
					<em style="font-style:italic;color:rgba(255,255,255,.4)">compresses.</em>
				</h2>
				<p class="bp2">
					Raw sessions accumulate. Locus aggregates them into daily rollups, weekly summaries,
					monthly signals - each tier retaining the attractor fingerprint of what came before.
				</p>
			</div>
			<div class="dc sr diagram-xl desktop-geom" style="transition-delay:.1s;padding:48px 40px">
				<svg width="100%" viewBox="0 0 1020 400" style="display:block" id="alluvial-svg">
					<defs>
						<linearGradient id="flow-raw-daily" x1="0%" y1="0%" x2="100%" y2="0%">
							<stop offset="0%" stop-color="rgba(77,191,160,.55)" />
							<stop offset="100%" stop-color="rgba(80,144,208,.45)" />
						</linearGradient>
						<linearGradient id="flow-daily-weekly" x1="0%" y1="0%" x2="100%" y2="0%">
							<stop offset="0%" stop-color="rgba(80,144,208,.45)" />
							<stop offset="100%" stop-color="rgba(139,110,196,.45)" />
						</linearGradient>
						<linearGradient id="flow-weekly-monthly" x1="0%" y1="0%" x2="100%" y2="0%">
							<stop offset="0%" stop-color="rgba(139,110,196,.45)" />
							<stop offset="100%" stop-color="rgba(212,148,58,.5)" />
						</linearGradient>
						<linearGradient id="flow-raw-daily-b" x1="0%" y1="0%" x2="100%" y2="0%">
							<stop offset="0%" stop-color="rgba(196,96,128,.45)" />
							<stop offset="100%" stop-color="rgba(80,144,208,.3)" />
						</linearGradient>
						<linearGradient id="flow-daily-weekly-b" x1="0%" y1="0%" x2="100%" y2="0%">
							<stop offset="0%" stop-color="rgba(80,144,208,.3)" />
							<stop offset="100%" stop-color="rgba(139,110,196,.35)" />
						</linearGradient>
					</defs>

					<text
						x="90"
						y="22"
						text-anchor="middle"
						font-family="'JetBrains Mono'"
						font-size="10"
						fill="rgba(77,191,160,.65)"
						letter-spacing=".12em">RAW</text
					>
					<text
						x="90"
						y="36"
						text-anchor="middle"
						font-family="'Syne'"
						font-size="9"
						fill="rgba(255,255,255,.2)">sessions</text
					>
					<text
						x="330"
						y="22"
						text-anchor="middle"
						font-family="'JetBrains Mono'"
						font-size="10"
						fill="rgba(80,144,208,.65)"
						letter-spacing=".12em">DAILY</text
					>
					<text
						x="330"
						y="36"
						text-anchor="middle"
						font-family="'Syne'"
						font-size="9"
						fill="rgba(255,255,255,.2)">rolled up</text
					>
					<text
						x="610"
						y="22"
						text-anchor="middle"
						font-family="'JetBrains Mono'"
						font-size="10"
						fill="rgba(139,110,196,.65)"
						letter-spacing=".12em">WEEKLY</text
					>
					<text
						x="610"
						y="36"
						text-anchor="middle"
						font-family="'Syne'"
						font-size="9"
						fill="rgba(255,255,255,.2)">compressed</text
					>
					<text
						x="900"
						y="22"
						text-anchor="middle"
						font-family="'JetBrains Mono'"
						font-size="10"
						fill="rgba(212,148,58,.65)"
						letter-spacing=".12em">MONTHLY</text
					>
					<text
						x="900"
						y="36"
						text-anchor="middle"
						font-family="'Syne'"
						font-size="9"
						fill="rgba(255,255,255,.2)">signal only</text
					>

					<line
						x1="200"
						y1="48"
						x2="200"
						y2="370"
						stroke="rgba(255,255,255,.05)"
						stroke-width="1"
						stroke-dasharray="3 4"
					/>
					<line
						x1="470"
						y1="48"
						x2="470"
						y2="370"
						stroke="rgba(255,255,255,.05)"
						stroke-width="1"
						stroke-dasharray="3 4"
					/>
					<line
						x1="760"
						y1="48"
						x2="760"
						y2="370"
						stroke="rgba(255,255,255,.05)"
						stroke-width="1"
						stroke-dasharray="3 4"
					/>

					<rect
						x="20"
						y="60"
						width="140"
						height="44"
						rx="2"
						fill="rgba(77,191,160,.09)"
						stroke="rgba(77,191,160,.4)"
						stroke-width="1"
					/>
					<text
						x="34"
						y="79"
						font-family="'JetBrains Mono'"
						font-size="9"
						fill="rgba(77,191,160,.75)">session · 04-21</text
					>
					<text x="34" y="93" font-family="'Syne'" font-size="9" fill="rgba(255,255,255,.28)"
						>parser hardening · ψ 2.93</text
					>

					<rect
						x="20"
						y="114"
						width="140"
						height="44"
						rx="2"
						fill="rgba(77,191,160,.09)"
						stroke="rgba(77,191,160,.35)"
						stroke-width="1"
					/>
					<text
						x="34"
						y="133"
						font-family="'JetBrains Mono'"
						font-size="9"
						fill="rgba(77,191,160,.75)">session · 04-22</text
					>
					<text x="34" y="147" font-family="'Syne'" font-size="9" fill="rgba(255,255,255,.28)"
						>grammar update · ψ 2.87</text
					>

					<rect
						x="20"
						y="168"
						width="140"
						height="44"
						rx="2"
						fill="rgba(77,191,160,.09)"
						stroke="rgba(77,191,160,.3)"
						stroke-width="1"
					/>
					<text
						x="34"
						y="187"
						font-family="'JetBrains Mono'"
						font-size="9"
						fill="rgba(77,191,160,.65)">session · 04-23</text
					>
					<text x="34" y="201" font-family="'Syne'" font-size="9" fill="rgba(255,255,255,.28)"
						>retrieval tuning · ψ 2.91</text
					>

					<rect
						x="20"
						y="222"
						width="140"
						height="44"
						rx="2"
						fill="rgba(196,96,128,.09)"
						stroke="rgba(196,96,128,.4)"
						stroke-width="1"
					/>
					<text
						x="34"
						y="241"
						font-family="'JetBrains Mono'"
						font-size="9"
						fill="rgba(196,96,128,.75)">session · 04-24</text
					>
					<text x="34" y="255" font-family="'Syne'" font-size="9" fill="rgba(255,255,255,.28)"
						>sdk design · ψ 2.78</text
					>

					<rect
						x="20"
						y="276"
						width="140"
						height="44"
						rx="2"
						fill="rgba(196,96,128,.09)"
						stroke="rgba(196,96,128,.35)"
						stroke-width="1"
					/>
					<text
						x="34"
						y="295"
						font-family="'JetBrains Mono'"
						font-size="9"
						fill="rgba(196,96,128,.65)">session · 04-25</text
					>
					<text x="34" y="309" font-family="'Syne'" font-size="9" fill="rgba(255,255,255,.28)"
						>transport layer · ψ 2.82</text
					>

					<path
						d="M160 82 C210 82 220 148 270 148"
						fill="none"
						stroke="url(#flow-raw-daily)"
						stroke-width="18"
						stroke-linecap="round"
						opacity=".55"
					/>
					<path
						d="M160 136 C210 136 220 148 270 148"
						fill="none"
						stroke="url(#flow-raw-daily)"
						stroke-width="12"
						stroke-linecap="round"
						opacity=".4"
					/>
					<path
						d="M160 190 C210 190 220 148 270 148"
						fill="none"
						stroke="url(#flow-raw-daily)"
						stroke-width="10"
						stroke-linecap="round"
						opacity=".35"
					/>
					<path
						d="M160 244 C210 244 220 268 270 268"
						fill="none"
						stroke="url(#flow-raw-daily-b)"
						stroke-width="18"
						stroke-linecap="round"
						opacity=".5"
					/>
					<path
						d="M160 298 C210 298 220 268 270 268"
						fill="none"
						stroke="url(#flow-raw-daily-b)"
						stroke-width="14"
						stroke-linecap="round"
						opacity=".4"
					/>

					<rect
						x="270"
						y="104"
						width="160"
						height="88"
						rx="2"
						fill="rgba(80,144,208,.1)"
						stroke="rgba(80,144,208,.45)"
						stroke-width="1"
					/>
					<text
						x="284"
						y="126"
						font-family="'JetBrains Mono'"
						font-size="9"
						fill="rgba(80,144,208,.8)">daily · 04-21 to 04-23</text
					>
					<text x="284" y="143" font-family="'Syne'" font-size="9" fill="rgba(255,255,255,.28)"
						>3 sessions merged</text
					>
					<text x="284" y="160" font-family="'Syne'" font-size="9" fill="rgba(255,255,255,.22)"
						>focus: parser · grammar</text
					>
					<text x="284" y="177" font-family="'Syne'" font-size="9" fill="rgba(255,255,255,.35)"
						>ψ̄ 2.90 · κ 0.94</text
					>

					<rect
						x="270"
						y="230"
						width="160"
						height="70"
						rx="2"
						fill="rgba(80,144,208,.08)"
						stroke="rgba(80,144,208,.35)"
						stroke-width="1"
					/>
					<text
						x="284"
						y="252"
						font-family="'JetBrains Mono'"
						font-size="9"
						fill="rgba(80,144,208,.7)">daily · 04-24 to 04-25</text
					>
					<text x="284" y="269" font-family="'Syne'" font-size="9" fill="rgba(255,255,255,.28)"
						>2 sessions merged</text
					>
					<text x="284" y="286" font-family="'Syne'" font-size="9" fill="rgba(255,255,255,.35)"
						>ψ̄ 2.80 · κ 0.91</text
					>

					<path
						d="M430 148 C458 148 462 200 510 200"
						fill="none"
						stroke="url(#flow-daily-weekly)"
						stroke-width="28"
						stroke-linecap="round"
						opacity=".5"
					/>
					<path
						d="M430 265 C458 265 462 200 510 200"
						fill="none"
						stroke="url(#flow-daily-weekly-b)"
						stroke-width="20"
						stroke-linecap="round"
						opacity=".4"
					/>

					<rect
						x="510"
						y="130"
						width="160"
						height="140"
						rx="2"
						fill="rgba(139,110,196,.1)"
						stroke="rgba(139,110,196,.5)"
						stroke-width="1"
					/>
					<text
						x="524"
						y="154"
						font-family="'JetBrains Mono'"
						font-size="9"
						fill="rgba(139,110,196,.85)">weekly · Apr W4</text
					>
					<text x="524" y="171" font-family="'Syne'" font-size="9" fill="rgba(255,255,255,.28)"
						>5 sessions → 2 rollups</text
					>
					<text x="524" y="190" font-family="'Syne'" font-size="9" fill="rgba(255,255,255,.22)"
						>dominant: parser work</text
					>
					<text x="524" y="207" font-family="'Syne'" font-size="9" fill="rgba(255,255,255,.22)"
						>secondary: sdk design</text
					>
					<text x="524" y="228" font-family="'Syne'" font-size="9" fill="rgba(255,255,255,.35)"
						>ψ̄ 2.87 · κ 0.93 · ρ 0.95</text
					>
					<text
						x="524"
						y="254"
						font-family="'JetBrains Mono'"
						font-size="8"
						fill="rgba(139,110,196,.5)">avec preserved ↓</text
					>

					<path
						d="M670 200 C720 200 740 210 800 210"
						fill="none"
						stroke="url(#flow-weekly-monthly)"
						stroke-width="32"
						stroke-linecap="round"
						opacity=".45"
					/>

					<rect
						x="800"
						y="150"
						width="190"
						height="120"
						rx="2"
						fill="rgba(212,148,58,.09)"
						stroke="rgba(212,148,58,.5)"
						stroke-width="1"
					/>
					<text
						x="816"
						y="174"
						font-family="'JetBrains Mono'"
						font-size="9"
						fill="rgba(212,148,58,.85)">monthly · April 2026</text
					>
					<text x="816" y="191" font-family="'Syne'" font-size="9" fill="rgba(255,255,255,.28)"
						>protocol + infra work</text
					>
					<text x="816" y="210" font-family="'Syne'" font-size="9" fill="rgba(255,255,255,.22)"
						>signal: high logic · low friction</text
					>
					<text x="816" y="231" font-family="'Syne'" font-size="9" fill="rgba(255,255,255,.35)"
						>ψ̄ 2.88 · compressed</text
					>
					<rect x="816" y="246" width="18" height="10" rx="1" fill="rgba(212,148,58,.4)" />
					<rect x="838" y="242" width="18" height="14" rx="1" fill="rgba(212,148,58,.55)" />
					<rect x="860" y="239" width="18" height="17" rx="1" fill="rgba(212,148,58,.65)" />
					<rect x="882" y="244" width="18" height="12" rx="1" fill="rgba(212,148,58,.45)" />

					<circle r="4" fill="rgba(77,191,160,.7)" opacity="0">
						<animateMotion dur="2.8s" repeatCount="indefinite" begin="0s">
							<mpath href="#path-a1" />
						</animateMotion>
						<animate attributeName="opacity" values="0;1;1;0" dur="2.8s" repeatCount="indefinite" />
					</circle>
					<path id="path-a1" d="M160 82 C210 82 220 148 270 148" fill="none" />

					<circle r="3.5" fill="rgba(80,144,208,.7)" opacity="0">
						<animateMotion dur="3.2s" repeatCount="indefinite" begin="0.6s">
							<mpath href="#path-a2" />
						</animateMotion>
						<animate
							attributeName="opacity"
							values="0;1;1;0"
							dur="3.2s"
							repeatCount="indefinite"
							begin="0.6s"
						/>
					</circle>
					<path id="path-a2" d="M430 148 C458 148 462 200 510 200" fill="none" />

					<circle r="4.5" fill="rgba(139,110,196,.7)" opacity="0">
						<animateMotion dur="3.6s" repeatCount="indefinite" begin="1.2s">
							<mpath href="#path-a3" />
						</animateMotion>
						<animate
							attributeName="opacity"
							values="0;1;1;0"
							dur="3.6s"
							repeatCount="indefinite"
							begin="1.2s"
						/>
					</circle>
					<path id="path-a3" d="M670 200 C720 200 740 210 800 210" fill="none" />

					<text
						x="510"
						y="390"
						font-family="'DM Serif Display'"
						font-style="italic"
						font-size="13"
						fill="rgba(255,255,255,.1)">information compresses · attractor state persists</text
					>
				</svg>
			</div>
			<div class="dc sr mobile-geom" style="transition-delay:.1s;padding:22px 18px">
				<div class="flow-stack">
					<div class="flow-tier" style="border-color:rgba(77,191,160,.35)">
						<div class="th">
							<span style="color:rgba(77,191,160,.85)">Raw</span><span>sessions</span>
						</div>
						<div class="td">Individual session nodes retain full detail and highest variance.</div>
						<div class="flow-meter">
							<div style="width:94%;background:rgba(77,191,160,.5)"></div>
						</div>
					</div>
					<div class="stack-arrow">↓ roll up</div>
					<div class="flow-tier" style="border-color:rgba(80,144,208,.35)">
						<div class="th">
							<span style="color:rgba(80,144,208,.85)">Daily</span><span>merged</span>
						</div>
						<div class="td">
							Nearby sessions compress into daily summaries while preserving attractor signature.
						</div>
						<div class="flow-meter">
							<div style="width:80%;background:rgba(80,144,208,.5)"></div>
						</div>
					</div>
					<div class="stack-arrow">↓ compress</div>
					<div class="flow-tier" style="border-color:rgba(139,110,196,.35)">
						<div class="th">
							<span style="color:rgba(139,110,196,.85)">Weekly</span><span>condensed</span>
						</div>
						<div class="td">
							Pattern-level memory remains: sustained work streams and strategic pivots.
						</div>
						<div class="flow-meter">
							<div style="width:64%;background:rgba(139,110,196,.5)"></div>
						</div>
					</div>
					<div class="stack-arrow">↓ distill</div>
					<div class="flow-tier" style="border-color:rgba(212,148,58,.35)">
						<div class="th">
							<span style="color:rgba(212,148,58,.85)">Monthly</span><span>signal</span>
						</div>
						<div class="td">
							Only durable trend survives with lower noise and long-horizon recall readiness.
						</div>
						<div class="flow-meter">
							<div style="width:48%;background:rgba(212,148,58,.55)"></div>
						</div>
					</div>
				</div>
			</div>
		</div>
	</section>

	<section class="ss" id="avec" style="padding:120px 0 140px">
		<div class="wrap">
			<div class="tc">
				<div class="sr">
					<span class="ml">Cognitive state</span>
					<h2 class="dh">The attractor<br />vector.</h2>
					<p class="bp2" style="margin-bottom:28px">
						Every node carries an AVEC - a four-dimensional fingerprint of cognitive state at the
						moment of encoding.
					</p>
					<div style="font-family:var(--fm);font-size:12px;color:var(--text-faint)">
						<div class="ab">
							<span style="color:rgba(77,191,160,.7)">stability</span>
							<div class="at">
								<div class="af" style="width:90%;background:rgba(77,191,160,.4)"></div>
							</div>
							<span>0.90</span>
						</div>
						<div class="ab">
							<span style="color:rgba(196,96,128,.7)">friction</span>
							<div class="at">
								<div class="af" style="width:20%;background:rgba(196,96,128,.4)"></div>
							</div>
							<span>0.20</span>
						</div>
						<div class="ab">
							<span style="color:rgba(80,144,208,.7)">logic</span>
							<div class="at">
								<div class="af" style="width:98%;background:rgba(80,144,208,.4)"></div>
							</div>
							<span>0.98</span>
						</div>
						<div class="ab">
							<span style="color:rgba(139,110,196,.7)">autonomy</span>
							<div class="at">
								<div class="af" style="width:85%;background:rgba(139,110,196,.4)"></div>
							</div>
							<span>0.85</span>
						</div>
					</div>
				</div>
				<div class="sr" style="transition-delay:.15s">
					<div class="dc">
						<p class="bp2" style="margin:0">
							ψ coherence is derived - not stored. A receiving agent computes it independently,
							verifying drift without shared history.
						</p>
					</div>
				</div>
			</div>
		</div>
	</section>

	<section class="ss" style="padding:100px 0 100px">
		<div class="wrap sr center-copy">
			<span class="ml" style="display:block">Part of the ecosystem</span>
			<h2 class="dh">
				Built on <em style="font-style:italic;color:rgba(255,255,255,.3)">STTP</em>. Powers
				<a
					href={resonantiaHref}
					target="_blank"
					rel="noopener noreferrer"
					style="color:var(--teal);text-decoration:none;font-style:normal">Resonantia</a
				>.
			</h2>
			<p class="bp2" style="margin:0 auto 48px">
				Locus is the infrastructure STTP agents stand on. Any agent, tool, or service that speaks
				STTP can use Locus as its memory layer.
			</p>
			<div
				class="eco-stack"
				style="display:inline-flex;flex-direction:column;align-items:stretch;gap:1px;font-family:var(--fm);font-size:11px;color:var(--text-faint);margin-bottom:56px;min-width:360px"
			>
				<div
					style="padding:12px 28px;background:var(--nebula);border:1px solid var(--mist2);border-radius:2px 2px 0 0;color:rgba(77,191,160,.6);text-align:center"
				>
					resonantia · your app · any STTP agent
				</div>
				<div
					style="padding:12px 28px;background:var(--surface);border:1px solid var(--mist2);text-align:center;color:rgba(139,110,196,.7)"
				>
					locus - memory layer
				</div>
				<div
					style="padding:12px 28px;background:var(--nebula);border:1px solid var(--mist2);border-radius:0 0 2px 2px;text-align:center;color:rgba(255,255,255,.25)"
				>
					⏣ STTP · typed IR · cognitive state
				</div>
			</div>
			<div class="cta-row" style="display:flex;gap:14px;justify-content:center">
				<a href={githubHref} class="btn bp" target="_blank" rel="noopener noreferrer"
					>View on GitHub</a
				>
				<a href={resonantiaHref} class="btn bg" target="_blank" rel="noopener noreferrer"
					>See Resonantia</a
				>
			</div>
		</div>
	</section>
</main>

<footer>
	<div>
		<span style="font-family:var(--fd);font-style:italic;font-size:15px;color:var(--text-faint)"
			>Locus</span
		>
		&nbsp;·&nbsp; Apache-2.0 &nbsp;·&nbsp;
		<a href={githubHref} target="_blank" rel="noopener noreferrer">entasislabs/locus</a>
	</div>
	<div style="display:flex;gap:24px;flex-wrap:wrap">
		<a href={resolve('/quickstart')}>Quickstart</a>
		<a href={resolve('/deployment-operations')}>Deploy/Ops</a>
		<a href={resolve('/docs')}>Docs</a>
		<a href={githubHref} target="_blank" rel="noopener noreferrer">GitHub</a>
		<a href={resonantiaHref} target="_blank" rel="noopener noreferrer">Resonantia</a>
	</div>
</footer>

<style>
	:root {
		--void: #04030d;
		--deep: #080717;
		--nebula: #0c0a1e;
		--surface: #110f28;
		--rim: #1c1935;
		--mist: rgba(255, 255, 255, 0.055);
		--mist2: rgba(255, 255, 255, 0.09);
		--star: rgba(255, 255, 255, 0.88);
		--text: rgba(255, 255, 255, 0.82);
		--text-dim: rgba(255, 255, 255, 0.56);
		--text-faint: rgba(255, 255, 255, 0.32);
		--teal: #4dbfa0;
		--purple: #8b6ec4;
		--fd: 'DM Serif Display', Georgia, serif;
		--fu: 'Syne', sans-serif;
		--fm: 'JetBrains Mono', monospace;
	}

	:global(body) {
		background: var(--void);
		color: var(--text);
		font-family: var(--fu);
		font-size: 16px;
		line-height: 1.72;
		overflow-x: hidden;
	}

	#sf {
		position: fixed;
		inset: 0;
		z-index: 0;
		pointer-events: none;
	}

	.nb {
		position: fixed;
		inset: 0;
		z-index: 0;
		pointer-events: none;
		overflow: hidden;
	}

	.nb::before {
		content: '';
		position: absolute;
		width: 1000px;
		height: 800px;
		top: -300px;
		left: -250px;
		background: radial-gradient(ellipse, rgba(139, 110, 196, 0.15) 0%, transparent 65%);
		animation: nd1 32s ease-in-out infinite alternate;
	}

	.nb::after {
		content: '';
		position: absolute;
		width: 800px;
		height: 700px;
		bottom: -100px;
		right: -200px;
		background: radial-gradient(ellipse, rgba(77, 191, 160, 0.1) 0%, transparent 60%);
		animation: nd2 24s ease-in-out infinite alternate;
	}

	main {
		position: relative;
		z-index: 1;
	}

	.hero {
		min-height: 100vh;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		text-align: center;
		padding: 130px 48px 100px;
	}

	.hl {
		font-family: var(--fm);
		font-size: 12px;
		font-weight: 300;
		letter-spacing: 0.22em;
		text-transform: uppercase;
		color: var(--teal);
		margin-bottom: 32px;
		opacity: 0;
		animation: fu 0.8s 0.2s ease forwards;
	}

	.ht {
		font-family: var(--fd);
		font-size: clamp(58px, 9vw, 118px);
		line-height: 0.95;
		letter-spacing: -0.02em;
		color: var(--star);
		margin-bottom: 24px;
		opacity: 0;
		animation: fu 1s 0.4s ease forwards;
	}

	.ht em {
		font-style: italic;
		color: transparent;
		-webkit-text-stroke: 1px rgba(255, 255, 255, 0.35);
	}

	.hs {
		font-size: clamp(17px, 2.1vw, 21px);
		color: var(--text-dim);
		max-width: 560px;
		line-height: 1.82;
		margin-bottom: 52px;
		opacity: 0;
		animation: fu 1s 0.65s ease forwards;
	}

	.ha {
		display: flex;
		gap: 14px;
		opacity: 0;
		animation: fu 1s 0.9s ease forwards;
	}

	.sr {
		opacity: 0;
		transform: translateY(28px);
		transition:
			opacity 0.7s ease,
			transform 0.7s ease;
	}

	:global(.sr.in) {
		opacity: 1;
		transform: none;
	}

	section[id] {
		scroll-margin-top: 92px;
	}

	.wrap {
		max-width: 1100px;
		margin: 0 auto;
		padding: 0 52px;
	}

	.ss {
		border-top: 1px solid var(--mist);
	}

	.ml {
		font-family: var(--fm);
		font-size: 11px;
		letter-spacing: 0.22em;
		text-transform: uppercase;
		color: var(--teal);
		display: block;
		margin-bottom: 14px;
	}

	.dh {
		font-family: var(--fd);
		font-size: clamp(38px, 5vw, 62px);
		line-height: 1.08;
		color: var(--star);
		margin-bottom: 18px;
	}

	.bp2 {
		font-size: 17px;
		color: var(--text-dim);
		line-height: 1.82;
		max-width: 640px;
	}

	.center-copy {
		max-width: 760px;
		margin: 0 auto;
		text-align: center;
	}

	.tc {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 72px;
		align-items: center;
	}

	.stack-row {
		display: flex;
		align-items: center;
		gap: 14px;
		padding: 20px 24px;
		background: var(--nebula);
		border: 1px solid var(--mist2);
	}

	.stack-key {
		font-family: var(--fm);
		font-size: 12px;
		letter-spacing: 0.14em;
		min-width: 108px;
	}

	.stack-value {
		font-size: 13px;
		color: var(--text-dim);
	}

	.stack-footer {
		font-family: var(--fm);
		font-size: 12px;
		color: var(--text-faint);
		letter-spacing: 0.1em;
	}

	.mcp {
		color: rgba(77, 191, 160, 0.7);
	}

	.gateway {
		color: rgba(80, 144, 208, 0.8);
	}

	.sdk {
		color: rgba(139, 110, 196, 0.8);
	}

	.cli {
		color: rgba(212, 148, 58, 0.7);
	}

	.dc {
		background: var(--nebula);
		border: 1px solid var(--mist2);
		border-radius: 3px;
		padding: 36px;
	}

	.stack-diagram {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.stack-node {
		background: var(--nebula);
		border: 1px solid var(--mist2);
		border-radius: 3px;
		padding: 14px 14px 12px;
	}

	.stack-node .k {
		font-family: var(--fm);
		font-size: 9px;
		letter-spacing: 0.13em;
		text-transform: uppercase;
	}

	.stack-node .t {
		font-size: 14px;
		color: var(--text-faint);
		margin-top: 8px;
		line-height: 1.62;
	}

	.stack-arrow {
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--text-faint);
		font-family: var(--fm);
		font-size: 11px;
		letter-spacing: 0.12em;
	}

	.mobile-geom {
		display: none;
	}

	.diagram-xl {
		overflow-x: visible;
	}

	.pg {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 1px;
		background: var(--mist);
		border: 1px solid var(--mist2);
		border-radius: 3px;
	}

	.pc {
		background: var(--nebula);
		padding: 34px 30px;
	}

	.pn {
		font-family: var(--fm);
		font-size: 12px;
		letter-spacing: 0.18em;
		margin-bottom: 10px;
	}

	.pname {
		font-family: var(--fd);
		font-size: 24px;
		color: var(--star);
		margin-bottom: 8px;
	}

	.pdesc {
		font-size: 15px;
		color: var(--text-dim);
		line-height: 1.74;
	}

	.flow-stack {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.flow-tier {
		background: var(--nebula);
		border: 1px solid var(--mist2);
		border-radius: 3px;
		padding: 14px;
	}

	.flow-tier .th {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 8px;
	}

	.flow-tier .th span:first-child {
		font-family: var(--fm);
		font-size: 11px;
		letter-spacing: 0.12em;
		text-transform: uppercase;
	}

	.flow-tier .th span:last-child {
		font-family: var(--fm);
		font-size: 11px;
		color: var(--text-faint);
	}

	.flow-tier .td {
		font-size: 14px;
		color: var(--text-dim);
		line-height: 1.64;
	}

	.flow-meter {
		height: 4px;
		background: var(--rim);
		border-radius: 999px;
		overflow: hidden;
		margin-top: 10px;
	}

	.flow-meter > div {
		height: 100%;
		border-radius: 999px;
	}

	.ab {
		display: flex;
		gap: 14px;
		align-items: center;
		margin-bottom: 10px;
	}

	.ab span:first-child {
		font-family: var(--fm);
		font-size: 11px;
		min-width: 72px;
	}

	.at {
		flex: 1;
		height: 3px;
		background: var(--rim);
		border-radius: 2px;
		overflow: hidden;
	}

	.af {
		height: 100%;
		border-radius: 2px;
	}

	.ab span:last-child {
		font-family: var(--fm);
		font-size: 11px;
		color: var(--text-faint);
		min-width: 32px;
		text-align: right;
	}

	.btn {
		font-family: var(--fu);
		font-size: 12px;
		font-weight: 600;
		letter-spacing: 0.1em;
		text-transform: uppercase;
		text-decoration: none;
		padding: 16px 30px;
		border-radius: 2px;
		transition: all 0.2s;
		cursor: pointer;
		border: none;
	}

	.bp {
		background: var(--purple);
		color: #fff;
	}

	.bp:hover {
		background: #a080d8;
		transform: translateY(-1px);
		box-shadow: 0 8px 28px rgba(139, 110, 196, 0.45);
	}

	.bg {
		background: transparent;
		color: var(--text-dim);
		border: 1px solid rgba(255, 255, 255, 0.18);
	}

	.bg:hover {
		color: var(--star);
		border-color: rgba(255, 255, 255, 0.45);
	}

	footer {
		position: relative;
		z-index: 1;
		border-top: 1px solid var(--mist);
		padding: 44px 52px;
		display: flex;
		align-items: center;
		justify-content: space-between;
		font-size: 13px;
		color: var(--text-faint);
	}

	footer a {
		color: var(--text-dim);
		text-decoration: none;
	}

	footer a:hover {
		color: var(--star);
	}

	@media (max-width: 980px) {
		.hero {
			padding: 116px 24px 84px;
		}

		.wrap {
			padding: 0 24px;
		}

		.tc {
			grid-template-columns: 1fr;
			gap: 40px;
		}

		.pg {
			grid-template-columns: 1fr 1fr;
		}

		footer {
			padding: 34px 24px;
			flex-direction: column;
			align-items: flex-start;
			gap: 12px;
		}
	}

	@media (max-width: 760px) {
		section[id] {
			scroll-margin-top: 80px;
		}

		.hero {
			min-height: auto;
			padding: 108px 16px 64px;
		}

		.hero-logo {
			width: min(84vw, 340px) !important;
			height: auto !important;
		}

		.ht {
			font-size: clamp(48px, 15.5vw, 70px);
			line-height: 1;
			margin-bottom: 16px;
		}

		.hs {
			font-size: 17px;
			line-height: 1.64;
			max-width: 34ch;
			margin-bottom: 30px;
		}

		.ha {
			flex-direction: column;
			width: 100%;
			max-width: 320px;
			align-items: center;
		}

		.btn {
			width: auto;
			text-align: center;
			font-size: 11px;
			padding: 11px 16px;
			min-width: 180px;
		}

		.wrap {
			padding: 0 16px;
		}

		.pg {
			grid-template-columns: 1fr;
		}

		.diagram-xl {
			overflow-x: auto;
			overflow-y: hidden;
			-webkit-overflow-scrolling: touch;
		}

		.diagram-xl svg {
			min-width: 860px;
		}

		.desktop-geom {
			display: none !important;
		}

		.mobile-geom {
			display: block;
		}

		.eco-stack {
			min-width: 0 !important;
			width: 100% !important;
		}

		footer {
			align-items: center;
			text-align: center;
		}
	}

	@keyframes nd1 {
		to {
			transform: translate(80px, 50px) scale(1.1);
		}
	}

	@keyframes nd2 {
		to {
			transform: translate(-50px, -70px) scale(1.15);
		}
	}

	@keyframes fu {
		from {
			opacity: 0;
			transform: translateY(20px);
		}
		to {
			opacity: 1;
			transform: none;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		:global(html) {
			scroll-behavior: auto;
		}

		*,
		*::before,
		*::after {
			animation: none !important;
			transition: none !important;
		}

		.sr {
			opacity: 1;
			transform: none;
		}
	}
</style>
