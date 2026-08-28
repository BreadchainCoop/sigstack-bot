<script lang="ts">
	import { getLocale } from '$lib/paraglide/runtime';
	import { getContent } from '$lib/content';

	const { pages, meta } = $derived(getContent(getLocale()));
	const page = $derived(pages.privacy);
</script>

<svelte:head>
	<title>{page.title} — {meta.siteName}</title>
</svelte:head>

<section class="page-stub">
	<h1>{page.title}</h1>
	<p class="lead">{page.lead}</p>
</section>
