<script lang="ts">
	import '../app.css';
	import SiteHeader from '$lib/components/SiteHeader.svelte';
	import SiteFooter from '$lib/components/SiteFooter.svelte';
	import { getLocale } from '$lib/paraglide/runtime';
	import * as m from '$lib/paraglide/messages';
	import favicon from '$lib/assets/favicon.svg';

	let { children } = $props();

	const locale = $derived(getLocale());
	const showStubNote = $derived(locale === 'es' || locale === 'fr');
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
	<link rel="preconnect" href="https://fonts.googleapis.com" />
	<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous" />
	<link
		href="https://fonts.googleapis.com/css2?family=Source+Sans+3:ital,wght@0,400;0,600;0,700;0,800;1,400&display=swap"
		rel="stylesheet"
	/>
</svelte:head>

<SiteHeader />

{#if showStubNote}
	<p class="locale-note shell" role="status">{m.stub_locale_note()}</p>
{/if}

<main id="main" class="site-main shell">
	{@render children()}
</main>

<SiteFooter />

<style>
	.locale-note {
		margin: var(--space-3) auto 0;
		padding: var(--space-2) var(--space-4);
		background: var(--color-paper-1);
		border: 1px solid var(--color-paper-2);
		font-size: 0.9rem;
		color: var(--color-brown);
	}
</style>
