<script lang="ts">
	import { getLocale } from '$lib/paraglide/runtime';
	import { getContent } from '$lib/content';

	const content = $derived(getContent(getLocale()));
	const { pages, meta } = $derived(content);
	const page = $derived(pages.home);
</script>

<svelte:head>
	<title>{meta.siteName} — {meta.tagline}</title>
	<meta name="description" content={meta.description} />
	<meta property="og:title" content={meta.siteName} />
	<meta property="og:description" content={meta.description} />
</svelte:head>

<section class="page-stub">
	<h1>{page.title}</h1>
	<p class="lead">{page.lead}</p>
</section>
