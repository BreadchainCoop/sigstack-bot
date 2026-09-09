<script lang="ts">
	import { browser } from '$app/environment';
	import { page } from '$app/state';
	import { getLocale } from '$lib/paraglide/runtime';
	import { getContent } from '$lib/content';
	import { linkCommand, planLabelFromSku, readLinkCode } from '$lib/checkoutLanding';
	import Button from '$lib/components/Button.svelte';
	import { env } from '$env/dynamic/public';

	// Stripe Checkout should redirect here as:
	//   …/checkout/success/?code=<link_token>&plan=<plan_sku>
	// Query params are client-only (static prerender cannot vary by searchParams).
	const content = $derived(getContent(getLocale()));
	const copy = $derived(content.pages.checkoutSuccess);
	const meta = $derived(content.meta);

	const code = $derived(browser ? readLinkCode(page.url) : null);
	const planSku = $derived(browser ? page.url.searchParams.get('plan') : null);
	const planLabel = $derived(planLabelFromSku(planSku, content));
	const planLine = $derived(
		planLabel
			? copy.planPurchased.replace('{plan}', planLabel)
			: copy.planPurchasedGeneric
	);
	const portalUrl = $derived(env.PUBLIC_STRIPE_PORTAL_URL?.trim() || '');
</script>

<svelte:head>
	<title>{copy.title} — {meta.siteName}</title>
	<meta name="description" content={copy.lead} />
</svelte:head>

<p class="eyebrow">{copy.eyebrow}</p>
<h1>{copy.title}</h1>
<p class="lead">{copy.lead}</p>
<p class="plan-line">{planLine}</p>

{#if browser}
	{#if code}
		<section class="panel link-panel" aria-labelledby="link-heading">
			<h2 id="link-heading">{copy.linkHeading}</h2>
			<p>{copy.linkBody}</p>
			<p class="link-cmd"><code>{linkCommand(code)}</code></p>
		</section>
	{:else}
		<p class="missing muted" role="status">{copy.missingCode}</p>
	{/if}
{/if}

<div class="cta-row">
	{#if portalUrl}
		<Button href={portalUrl} external>{copy.portalCta}</Button>
	{:else}
		<span class="portal-stub muted">{copy.portalComingSoon}</span>
	{/if}
	<Button href="/get-started" variant={portalUrl ? 'ghost' : 'primary'}>{copy.getStartedCta}</Button>
</div>

<style>
	.plan-line {
		font-weight: 600;
		margin-bottom: var(--space-5);
	}

	.link-panel h2 {
		margin-top: 0;
		font-size: 1.15rem;
	}

	.link-cmd {
		margin: var(--space-4) 0 0;
	}

	.link-cmd code {
		font-size: 1.05rem;
		padding: 0.35em 0.55em;
	}

	.missing {
		max-width: 40rem;
		margin-bottom: var(--space-5);
	}

	.portal-stub {
		display: inline-flex;
		align-items: center;
		padding: 0.7rem 0;
		font-size: 0.95rem;
	}
</style>
