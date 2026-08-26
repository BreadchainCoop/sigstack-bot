<script lang="ts">
	import { getLocale } from '$lib/paraglide/runtime';
	import { getContent } from '$lib/content';
	import Button from '$lib/components/Button.svelte';

	const { plans: page, meta } = $derived(getContent(getLocale()));
</script>

<svelte:head>
	<title>{page.title} — {meta.siteName}</title>
	<meta name="description" content={page.lead} />
</svelte:head>

<p class="eyebrow">Coming soon</p>
<h1>{page.title}</h1>
<p class="lead">{page.lead}</p>
<p>{page.body}</p>

<div class="cta-row">
	<Button href="/get-started">Learn the products</Button>
	<Button
		href="https://github.com/BreadchainCoop/sigstack-bot/issues/13"
		variant="secondary"
		external>Follow checkout issue #13</Button
	>
</div>
