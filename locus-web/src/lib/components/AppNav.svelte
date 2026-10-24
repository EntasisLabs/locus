<script lang="ts">
	import { resolve } from '$app/paths';
	import { onMount } from 'svelte';

	let { active = 'home' }: { active?: 'home' | 'quickstart' | 'deploy' | 'docs' } = $props();

	let open = $state(false);
	let menuButton: HTMLButtonElement | undefined = $state();

	function closeMenu(restoreFocus = false) {
		if (!open) return;
		open = false;
		if (restoreFocus) menuButton?.focus();
	}

	function onKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') closeMenu(true);
	}

	onMount(() => {
		const query = window.matchMedia('(max-width: 760px)');
		const onChange = () => {
			if (!query.matches) open = false;
		};
		query.addEventListener('change', onChange);
		return () => query.removeEventListener('change', onChange);
	});
</script>

<svelte:window onkeydown={onKeydown} />

<nav>
	<a href={resolve('/')} class="nl" onclick={() => closeMenu()}>
		<img src="/brand/locus-symbol-white.svg" alt="Locus" />
	</a>
	<button
		type="button"
		class="menu-btn"
		bind:this={menuButton}
		aria-expanded={open}
		aria-controls="site-menu"
		aria-label={open ? 'Close menu' : 'Open menu'}
		onclick={() => (open = !open)}
	>
		<span class="bars" aria-hidden="true"></span>
	</button>
	<ul id="site-menu" class="na" class:open>
		<li><span class="status-dot" aria-hidden="true"></span></li>
		<li>
			<a
				href={resolve('/')}
				class:active={active === 'home'}
				aria-current={active === 'home' ? 'page' : undefined}
				onclick={() => closeMenu()}>Home</a
			>
		</li>
		<li>
			<a
				href={resolve('/quickstart')}
				class:active={active === 'quickstart'}
				aria-current={active === 'quickstart' ? 'page' : undefined}
				onclick={() => closeMenu()}>Quickstart</a
			>
		</li>
		<li>
			<a
				href={resolve('/deployment-operations')}
				class:active={active === 'deploy'}
				aria-current={active === 'deploy' ? 'page' : undefined}
				onclick={() => closeMenu()}>Deploy/Ops</a
			>
		</li>
		<li>
			<a
				href={resolve('/docs')}
				class:active={active === 'docs'}
				aria-current={active === 'docs' ? 'page' : undefined}
				onclick={() => closeMenu()}>Docs</a
			>
		</li>
		<li>
			<a
				href="https://github.com/entasislabs/locus"
				target="_blank"
				rel="noopener noreferrer"
				onclick={() => closeMenu()}>GitHub</a
			>
		</li>
	</ul>
</nav>
{#if open}
	<button type="button" class="scrim" aria-label="Close menu" onclick={() => closeMenu(true)}
	></button>
{/if}

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

	.menu-btn {
		display: none;
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

	.scrim {
		display: none;
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
			padding: 8px 14px;
			gap: 12px;
		}

		.menu-btn {
			display: inline-flex;
			align-items: center;
			justify-content: center;
			width: 44px;
			height: 44px;
			margin-right: -6px;
			padding: 0;
			border: 0;
			border-radius: 10px;
			background: transparent;
			color: rgba(255, 255, 255, 0.92);
			cursor: pointer;
		}

		.menu-btn:focus-visible {
			outline: 2px solid #4dbfa0;
			outline-offset: 2px;
		}

		.bars,
		.bars::before,
		.bars::after {
			display: block;
			width: 18px;
			height: 1.5px;
			border-radius: 1px;
			background: currentColor;
		}

		.bars {
			position: relative;
		}

		.bars::before,
		.bars::after {
			content: '';
			position: absolute;
			left: 0;
		}

		.bars::before {
			top: -6px;
		}

		.bars::after {
			top: 6px;
		}

		.menu-btn[aria-expanded='true'] .bars {
			background: transparent;
		}

		.menu-btn[aria-expanded='true'] .bars::before {
			top: 0;
			transform: rotate(45deg);
		}

		.menu-btn[aria-expanded='true'] .bars::after {
			top: 0;
			transform: rotate(-45deg);
		}

		.na {
			display: none;
			position: absolute;
			top: 100%;
			left: 0;
			right: 0;
			flex-direction: column;
			align-items: stretch;
			gap: 2px;
			margin: 0;
			padding: 8px 12px 14px;
			background: rgba(8, 6, 18, 0.98);
			border-bottom: 1px solid rgba(255, 255, 255, 0.08);
			max-width: none;
			overflow: visible;
		}

		.na.open {
			display: flex;
		}

		.na > li:first-child {
			display: none;
		}

		.na li {
			width: 100%;
		}

		.na a {
			display: block;
			font-size: 16px;
			line-height: 1.3;
			padding: 12px 14px;
			border-radius: 10px;
		}

		.scrim {
			display: block;
			position: fixed;
			inset: 0;
			z-index: 90;
			border: 0;
			padding: 0;
			background: rgba(0, 0, 0, 0.45);
			cursor: pointer;
		}
	}
</style>
