<script lang="ts">
	import { getLocale } from '$lib/paraglide/runtime';
	import { getContent } from '$lib/content';
	import type { PlanOffer } from '$lib/content/types';
	import Button from '$lib/components/Button.svelte';
	import * as m from '$lib/paraglide/messages';

	const { pages, meta } = $derived(getContent(getLocale()));
	const page = $derived(pages.plans);

	function scopeLabel(scope: PlanOffer['scope']) {
		return page.scopeLabels[scope];
	}
</script>

<svelte:head>
	<title>{page.title} — {meta.siteName}</title>
	<meta name="description" content={page.lead} />
</svelte:head>

<section class="page-stub plans-hero">
	<h1>{page.title}</h1>
	<p class="lead">{page.lead}</p>
</section>

<section class="section bundle" aria-labelledby="bundle-heading">
	<p class="eyebrow accent-eyebrow">{page.bundle.eyebrow}</p>
	<h2 id="bundle-heading">{page.bundle.title}</h2>
	<p class="lead">{page.bundle.lead}</p>
	<p class="bundle-note muted">{page.bundle.note}</p>
	<div class="offer-grid bundle-grid">
		{#each page.bundle.offers as offer, i (offer.id)}
			<article class="panel offer" class:featured={offer.scope === 'group'} style="--offer-i: {i}">
				<p class="eyebrow scope">{scopeLabel(offer.scope)}</p>
				<h3>{offer.name}</h3>
				<p class="offer-blurb">{offer.blurb}</p>
				<p class="price">
					<span class="amount">{offer.priceLabel}</span><span class="period">{offer.period}</span>
				</p>
				<Button href={offer.ctaHref}>{offer.ctaLabel}</Button>
			</article>
		{/each}
	</div>
</section>

<section class="section a-la-carte" aria-labelledby="a-la-carte-heading">
	<h2 id="a-la-carte-heading">{page.aLaCarteHeading}</h2>
	<p class="lead">{page.aLaCarteLead}</p>

	{#each page.products as product (product.id)}
		<section class="product-block" id={product.id}>
			<h3 class="product-title">{product.title}</h3>
			<p class="product-lead muted">{product.lead}</p>
			<div class="offer-grid" class:single={product.offers.length === 1}>
				{#each product.offers as offer, i (offer.id)}
					<article class="panel offer" style="--offer-i: {i}">
						<p class="eyebrow scope">{scopeLabel(offer.scope)}</p>
						<h4 class="offer-name">{offer.name}</h4>
						<p class="offer-blurb">{offer.blurb}</p>
						<p class="price">
							<span class="amount">{offer.priceLabel}</span><span class="period">{offer.period}</span>
						</p>
						<Button href={offer.ctaHref} variant="ghost">{offer.ctaLabel}</Button>
					</article>
				{/each}
			</div>
		</section>
	{/each}
</section>

<p class="footnote muted">{page.footnote}</p>

<div class="cta-row plans-cta">
	<Button href="/get-started">{m.cta_start()}</Button>
	<Button href="/products" variant="ghost">{m.nav_products()}</Button>
</div>

<style>
	.plans-hero {
		padding-bottom: var(--space-6);
		border-bottom: 1px solid var(--border);
		margin-bottom: 0;
	}

	.accent-eyebrow {
		color: var(--accent-text);
	}

	.bundle {
		animation: bundle-in 0.55s ease-out both;
	}

	.bundle-note {
		max-width: 40rem;
		margin-bottom: var(--space-5);
		font-size: 0.95rem;
	}

	.offer-grid {
		display: grid;
		gap: var(--space-4);
	}

	@media (min-width: 640px) {
		.offer-grid:not(.single) {
			grid-template-columns: repeat(2, 1fr);
		}

		.bundle-grid {
			grid-template-columns: repeat(2, 1fr);
		}
	}

	.offer {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		animation: offer-in 0.45s ease-out both;
		animation-delay: calc(var(--offer-i, 0) * 60ms);
	}

	.offer.featured {
		border-color: var(--accent);
		box-shadow: 0 4px 0 color-mix(in srgb, var(--accent) 25%, transparent);
	}

	.offer .scope {
		margin-bottom: var(--space-2);
	}

	.offer-blurb {
		color: var(--muted);
		flex: 1;
		margin-bottom: var(--space-4);
	}

	.offer-name,
	.product-title {
		margin: 0 0 var(--space-2);
		font-size: 1.15rem;
		font-weight: 700;
		letter-spacing: -0.02em;
	}

	.product-block {
		margin-top: var(--space-6);
		scroll-margin-top: calc(var(--header-height) + var(--space-4));
	}

	.product-block:first-of-type {
		margin-top: var(--space-5);
	}

	.product-lead {
		max-width: 40rem;
		margin-bottom: var(--space-4);
	}

	.price {
		margin: 0 0 var(--space-4);
		line-height: 1;
	}

	.amount {
		font-size: clamp(1.75rem, 3vw, 2.25rem);
		font-weight: 700;
		letter-spacing: -0.03em;
		color: var(--fg);
	}

	.period {
		font-size: 1rem;
		font-weight: 500;
		color: var(--muted);
		margin-left: 0.15rem;
	}

	.footnote {
		max-width: 40rem;
		margin: var(--space-6) 0 0;
		font-size: 0.95rem;
	}

	.plans-cta {
		margin-top: var(--space-6);
		margin-bottom: var(--space-4);
	}

	@keyframes bundle-in {
		from {
			opacity: 0;
			transform: translateY(0.5rem);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	@keyframes offer-in {
		from {
			opacity: 0;
			transform: translateY(0.35rem);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.bundle,
		.offer {
			animation: none;
		}
	}
</style>
