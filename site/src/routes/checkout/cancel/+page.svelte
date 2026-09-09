<script lang="ts">
	import { getLocale } from '$lib/paraglide/runtime';
	import { getContent } from '$lib/content';
	import Button from '$lib/components/Button.svelte';

	const { pages, meta } = $derived(getContent(getLocale()));
	const copy = $derived(pages.checkoutCancel);
</script>

<svelte:head>
	<title>{copy.title} — {meta.siteName}</title>
	<meta name="description" content={copy.lead} />
</svelte:head>

<p class="eyebrow">{copy.eyebrow}</p>
<h1>{copy.title}</h1>
<p class="lead">{copy.lead}</p>

<div class="cta-row">
	<Button href="/plans">{copy.plansCta}</Button>
	<Button href="/get-started" variant="ghost">{copy.getStartedCta}</Button>
</div>
