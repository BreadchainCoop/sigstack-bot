<script lang="ts">
	import { getLocale } from '$lib/paraglide/runtime';
	import { getContent } from '$lib/content';
	import PathChooser from '$lib/components/PathChooser.svelte';
	import ProductDiagram from '$lib/components/ProductDiagram.svelte';
	import Button from '$lib/components/Button.svelte';
	import * as m from '$lib/paraglide/messages';
	import { ShieldCheck, Translate, Microphone } from 'phosphor-svelte';

	const content = $derived(getContent(getLocale()));
	const { landing, meta } = $derived(content);
</script>

<svelte:head>
	<title>{meta.siteName} — {meta.tagline}</title>
	<meta name="description" content={meta.description} />
	<meta property="og:title" content="{meta.siteName} — {meta.tagline}" />
	<meta property="og:description" content={meta.description} />
</svelte:head>

<section class="hero">
	<div>
		<p class="eyebrow">{landing.eyebrow}</p>
		<h1>{landing.title}</h1>
		<p class="lead">{landing.lead}</p>
		<p>{landing.notChat}</p>
		<div class="cta-row">
			<Button href="/get-started">{m.cta_start()}</Button>
			<Button href="/how-it-works" variant="secondary">{m.cta_how()}</Button>
		</div>
	</div>
	<div class="trust panel">
		<ul>
			<li>
				<ShieldCheck size={22} weight="bold" aria-hidden="true" />
				<span>Hardware TEE + <code class="cmd">!verify</code> attestation</span>
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

<PathChooser heading={landing.pathsHeading} paths={landing.paths} />

<section class="stack" style="margin-top: var(--space-7)">
	<h2>See Language Threads</h2>
	<p class="lead">Diagrams first — so organizers can picture fan-out before learning commands.</p>
	<ProductDiagram kind="language-threads" caption={content.languageThreads.diagramCaption} />
</section>

<style>
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
		color: var(--color-ink);
	}

	.trust :global(svg) {
		flex-shrink: 0;
		color: var(--color-primary-jade);
		margin-top: 0.15rem;
	}
</style>
