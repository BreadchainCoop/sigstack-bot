<script lang="ts">
	import { browser } from '$app/environment';
	import { page } from '$app/state';
	import { env } from '$env/dynamic/public';
	import { getLocale } from '$lib/paraglide/runtime';
	import { getContent } from '$lib/content';
	import { LINK_CODE_MAX_LENGTH, readLinkCode } from '$lib/checkoutLanding';
	import { resolveSignalUsernameLink } from '$lib/signalUsernameLink';
	import LinkInSignal from '$lib/components/LinkInSignal.svelte';

	const { pages, meta } = $derived(getContent(getLocale()));
	const copy = $derived(pages.alpha);

	// Query params are client-only (static prerender cannot vary by searchParams).
	const codeFromUrl = $derived(browser ? readLinkCode(page.url) : null);
	const signalUsernameLink = $derived(
		resolveSignalUsernameLink(env.PUBLIC_SIGNAL_USERNAME_TOKEN)
	);
	let draft = $state('');
	let revealedCode = $state<string | null>(null);
	let error = $state<string | null>(null);
	let hydratedFromUrl = $state(false);

	$effect(() => {
		if (!browser || hydratedFromUrl || !codeFromUrl) return;
		draft = codeFromUrl;
		revealedCode = codeFromUrl;
		hydratedFromUrl = true;
	});

	function onSubmit(e: Event) {
		e.preventDefault();
		const trimmed = draft.trim();
		if (!trimmed) {
			error = copy.errorEmpty;
			return;
		}
		if (trimmed.length > LINK_CODE_MAX_LENGTH) {
			error = copy.errorTooLong;
			return;
		}
		error = null;
		revealedCode = trimmed;
	}
</script>

<svelte:head>
	<title>{copy.title} — {meta.siteName}</title>
	<meta name="description" content={copy.lead} />
</svelte:head>

<p class="eyebrow">{copy.eyebrow}</p>
<h1>{copy.title}</h1>
<p class="lead">{copy.lead}</p>

<form class="alpha-form" onsubmit={onSubmit}>
	<label class="field" for="alpha-code">{copy.codeLabel}</label>
	<input
		id="alpha-code"
		name="code"
		type="text"
		autocomplete="off"
		spellcheck="false"
		placeholder={copy.codePlaceholder}
		bind:value={draft}
		aria-invalid={error ? 'true' : undefined}
		aria-describedby={error ? 'alpha-code-error' : undefined}
	/>
	{#if error}
		<p id="alpha-code-error" class="field-error" role="alert">{error}</p>
	{/if}
	<div class="cta-row form-actions">
		<button type="submit" class="btn btn-primary">{copy.submitLabel}</button>
	</div>
</form>

{#if revealedCode}
	<div class="link-reveal">
		<LinkInSignal
			code={revealedCode}
			heading={copy.linkHeading}
			body={copy.linkBody}
			copyLabel={copy.copyLabel}
			copyDoneLabel={copy.copyDoneLabel}
			messageCta={copy.messageCta}
			signalLinkMissing={copy.signalLinkMissing}
			{signalUsernameLink}
			nextStep={copy.enableNext}
		/>
	</div>
{/if}

<style>
	.alpha-form {
		max-width: 28rem;
		margin-top: var(--space-5);
	}

	.field {
		display: block;
		font-weight: 600;
		margin-bottom: var(--space-2);
	}

	input {
		display: block;
		width: 100%;
		padding: 0.7rem 0.85rem;
		font-family: var(--font-sans);
		font-size: 1rem;
		color: var(--fg);
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
	}

	input:focus-visible {
		outline: 2px solid var(--accent-2);
		outline-offset: 2px;
	}

	.field-error {
		color: var(--accent-text);
		font-size: 0.95rem;
		margin: var(--space-2) 0 0;
	}

	.form-actions {
		margin-top: var(--space-4);
	}

	.link-reveal {
		margin-top: var(--space-6);
		max-width: 40rem;
	}
</style>
