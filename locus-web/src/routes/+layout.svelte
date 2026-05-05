<script lang="ts">
	import { page } from '$app/state';
	import favicon from '$lib/assets/favicon.svg';

	let { children } = $props();

	const siteName = 'Locus';
	const siteUrl = 'https://locus.black';
	const defaultOgImage = `${siteUrl}/locus_final_transparent.png`;
	const canonicalHref = $derived(`${siteUrl}${page.url.pathname}`);
	const organizationJsonLd = JSON.stringify({
		'@context': 'https://schema.org',
		'@type': 'Organization',
		name: 'Locus',
		url: siteUrl,
		logo: defaultOgImage,
		sameAs: ['https://github.com/entasislabs/locus']
	});
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
	<meta name="robots" content="index, follow" />
	<link rel="canonical" href={canonicalHref} />
	<meta property="og:site_name" content={siteName} />
	<meta property="og:url" content={canonicalHref} />
	<meta property="og:image" content={defaultOgImage} />
	<meta name="twitter:image" content={defaultOgImage} />
	<meta name="twitter:site" content="@entasislabs" />
	<meta name="twitter:creator" content="@entasislabs" />
	<script type="application/ld+json">{organizationJsonLd}</script>
</svelte:head>

{@render children()}
