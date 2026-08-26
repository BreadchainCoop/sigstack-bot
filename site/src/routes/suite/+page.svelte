<script lang="ts">
	import { getLocale } from '$lib/paraglide/runtime';
	import { getContent } from '$lib/content';
	import Button from '$lib/components/Button.svelte';

	const { suite, meta } = $derived(getContent(getLocale()));
</script>

<svelte:head>
	<title>{suite.title} — {meta.siteName}</title>
	<meta name="description" content={suite.lead} />
</svelte:head>

<p class="eyebrow">Products</p>
<h1>{suite.title}</h1>
<p class="lead">{suite.lead}</p>

<div class="grid-3" style="margin-top: var(--space-6)">
	{#each suite.products as product (product.href)}
		<article class="panel">
			<h2>{product.title}</h2>
			<p>{product.body}</p>
			<Button href={product.href} variant="secondary">Open</Button>
		</article>
	{/each}
</div>
