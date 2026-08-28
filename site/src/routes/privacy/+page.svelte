<script lang="ts">
	import { getLocale } from '$lib/paraglide/runtime';
	import { getContent } from '$lib/content';
	import { m } from '$lib/paraglide/messages.js';
	import PrivacyTeeDiagram from '$lib/components/PrivacyTeeDiagram.svelte';
	import PrivacyTrustDiagram from '$lib/components/PrivacyTrustDiagram.svelte';
	import PrivacyPromiseDiagram from '$lib/components/PrivacyPromiseDiagram.svelte';
	import Button from '$lib/components/Button.svelte';

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

{#each page.sections as section (section.id)}
	<section class="section" id={section.id}>
		<h2>{section.title}</h2>
		<p class="lead">{section.lead}</p>
		{#if section.id === 'what-is-a-tee'}
			<PrivacyTeeDiagram />
		{:else if section.id === 'how-cipherslate'}
			<PrivacyTrustDiagram />
		{:else if section.id === 'promise-vs-proof'}
			<PrivacyPromiseDiagram />
		{/if}
	</section>
{/each}

<div class="cta-row privacy-cta">
	<Button href="/privacy-policy">{m.footer_privacy_policy()}</Button>
	<Button href="/products" variant="ghost">{m.nav_products()}</Button>
</div>

<style>
	.privacy-cta {
		margin-top: var(--space-8);
		margin-bottom: var(--space-4);
	}
</style>
