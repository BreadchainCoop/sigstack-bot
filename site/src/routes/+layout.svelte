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
		href="https://fonts.googleapis.com/css2?family=DM+Sans:ital,opsz,wght@0,9..40,400;0,9..40,500;0,9..40,600;0,9..40,700;1,9..40,400&display=swap"
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
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		font-size: 0.9rem;
		color: var(--muted);
	}
</style>
