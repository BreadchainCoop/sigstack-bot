<script lang="ts">
	import { getLocale } from '$lib/paraglide/runtime';
	import { getContent } from '$lib/content';
	import PathChooser from '$lib/components/PathChooser.svelte';
	import LanguageThreadsDiagram from '$lib/components/LanguageThreadsDiagram.svelte';
	import Button from '$lib/components/Button.svelte';
	import * as m from '$lib/paraglide/messages';
	import { ShieldCheck, Translate, Microphone } from 'phosphor-svelte';

	const content = $derived(getContent(getLocale()));
	const { pages, meta } = $derived(content);
	const page = $derived(pages.home);
</script>

<svelte:head>
	<title>{meta.siteName} — {meta.tagline}</title>
	<meta name="description" content={meta.description} />
	<meta property="og:title" content="{meta.siteName} — {meta.tagline}" />
	<meta property="og:description" content={meta.description} />
</svelte:head>

<section class="hero">
	<div>
		<h1>{page.title}</h1>
		<p class="lead">{page.lead}</p>
		<p class="not-chat">{page.notChat}</p>
		<div class="cta-row">
			<Button href="/get-started">{m.cta_start()}</Button>
			<Button href="/products" variant="ghost">{m.nav_products()}</Button>
		</div>
	</div>
	<div class="trust panel">
		<ul>
			<li>
				<ShieldCheck size={22} weight="bold" aria-hidden="true" />
				<span>Hardware TEE + <code>!verify</code> attestation</span>
			</li>
			<li>
				<Translate size={22} weight="bold" aria-hidden="true" />
				<span>Language Threads and in-chat translation</span>
			</li>
			<li>
				<Microphone size={22} weight="bold" aria-hidden="true" />
				<span>Voice notes → text via NEAR Whisper</span>
			</li>
		</ul>
	</div>
</section>

<PathChooser heading={page.pathsHeading} paths={page.paths} />

<section class="section teaser">
	<h2>{page.diagramHeading}</h2>
	<p class="lead">{page.diagramLead}</p>
	<LanguageThreadsDiagram />
</section>

<style>
	.not-chat {
		max-width: 40rem;
		color: var(--fg);
	}

	.trust ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		gap: var(--space-4);
	}

	.trust li {
		display: flex;
		gap: var(--space-3);
		align-items: flex-start;
		font-weight: 600;
		color: var(--fg);
	}

	.trust :global(svg) {
		flex-shrink: 0;
		color: var(--accent);
		margin-top: 0.15rem;
	}

	.teaser {
		padding-top: 0;
		border-top: none;
	}
</style>
