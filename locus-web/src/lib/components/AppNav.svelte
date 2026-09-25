<script lang="ts">
	import { resolve } from '$app/paths';

	let { active = 'home' }: { active?: 'home' | 'quickstart' | 'deploy' | 'docs' } = $props();
</script>

<nav>
	<a href={resolve('/')} class="nl">
		<img src="/brand/locus-lockup-horizontal-white.svg" alt="Locus" />
	</a>
	<ul class="na">
		<li><span class="status-dot" aria-hidden="true"></span></li>
		<li>
			<a
				href={resolve('/')}
				class:active={active === 'home'}
				aria-current={active === 'home' ? 'page' : undefined}>Home</a
			>
		</li>
		<li>
			<a
				href={resolve('/quickstart')}
				class:active={active === 'quickstart'}
				aria-current={active === 'quickstart' ? 'page' : undefined}>Quickstart</a
			>
		</li>
		<li>
			<a
				href={resolve('/deployment-operations')}
				class:active={active === 'deploy'}
				aria-current={active === 'deploy' ? 'page' : undefined}>Deploy/Ops</a
			>
		</li>
		<li>
			<a
				href={resolve('/docs')}
				class:active={active === 'docs'}
				aria-current={active === 'docs' ? 'page' : undefined}>Docs</a
			>
		</li>
		<li>
			<a href="https://github.com/entasislabs/locus" target="_blank" rel="noopener noreferrer"
				>GitHub</a
			>
		</li>
	</ul>
</nav>

<style>
	nav {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		z-index: 100;
		padding: 18px 52px;
		display: flex;
		align-items: center;
		justify-content: space-between;
		backdrop-filter: blur(24px);
		background: rgba(4, 3, 13, 0.72);
		border-bottom: 1px solid rgba(255, 255, 255, 0.06);
	}

	.nl {
		display: flex;
		align-items: center;
		text-decoration: none;
	}

	.nl img {
		height: 32px;
		width: auto;
		display: block;
	}

	.na {
		display: flex;
		gap: 10px;
		list-style: none;
		align-items: center;
	}

	.status-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: #4dbfa0;
		opacity: 0.82;
		box-shadow: 0 0 12px rgba(77, 191, 160, 0.55);
	}

	.na a {
		font-size: 14px;
		font-weight: 500;
		letter-spacing: 0.02em;
		text-transform: none;
		color: rgba(255, 255, 255, 0.62);
		text-decoration: none;
		transition: all 0.2s;
		padding: 7px 10px;
		border-radius: 999px;
		border: 1px solid transparent;
	}

	.na a.active {
		color: rgba(255, 255, 255, 0.95);
		background: rgba(255, 255, 255, 0.08);
		border-color: transparent;
	}

	.na a:hover {
		color: rgba(255, 255, 255, 0.95);
		border-color: rgba(255, 255, 255, 0.12);
		background: rgba(255, 255, 255, 0.04);
	}

	@media (max-width: 980px) {
		nav {
			padding: 14px 24px;
		}

		.na {
			gap: 18px;
		}
	}

	@media (max-width: 760px) {
		nav {
			padding: 12px 14px;
			gap: 12px;
		}

		.nl img {
			height: 28px;
		}

		.na {
			gap: 10px;
			max-width: 72vw;
			overflow-x: auto;
			padding-bottom: 4px;
			justify-content: flex-end;
		}

		.na a {
			font-size: 13px;
			white-space: nowrap;
			letter-spacing: 0.02em;
		}
	}

	@media (max-width: 520px) {
		.na li:nth-child(3) {
			display: none;
		}
	}

	@media (max-width: 420px) {
		nav {
			padding: 10px 10px;
		}

		.na {
			max-width: 70vw;
			gap: 8px;
		}

		.na a {
			font-size: 13px;
			letter-spacing: 0.01em;
		}
	}
</style>
