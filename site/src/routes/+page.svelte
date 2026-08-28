<script lang="ts">
	import { getLocale } from '$lib/paraglide/runtime';
	import { getContent } from '$lib/content';
	import Button from '$lib/components/Button.svelte';
	import * as m from '$lib/paraglide/messages';

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
	<div class="cta-row">
		<Button href="/get-started">{m.cta_start()}</Button>
		<Button href="/products" variant="ghost">{m.nav_products()}</Button>
	</div>
</section>
