<script lang="ts">
	import { getLocale } from '$lib/paraglide/runtime';
	import { getContent } from '$lib/content';
	import Button from '$lib/components/Button.svelte';
	import * as m from '$lib/paraglide/messages';

	const { pages, meta } = $derived(getContent(getLocale()));
	const page = $derived(pages.plans);
</script>

<svelte:head>
	<title>{page.title} — {meta.siteName}</title>
	<meta name="description" content={page.lead} />
</svelte:head>

<section class="page-stub plans-hero">
	<h1>{page.title}</h1>
	<p class="lead">{page.lead}</p>
</section>

<section class="section alpha-band panel" aria-labelledby="alpha-band-heading">
	<p class="eyebrow accent-eyebrow">{page.alphaBand.eyebrow}</p>
	<h2 id="alpha-band-heading">{page.alphaBand.title}</h2>
	<p class="lead alpha-lead">{page.alphaBand.lead}</p>
	<Button href="/alpha">{page.alphaBand.ctaLabel}</Button>
</section>

<section class="section paid" aria-labelledby="paid-heading">
	<p class="eyebrow accent-eyebrow">{page.paid.eyebrow}</p>
	<h2 id="paid-heading">{page.paid.title}</h2>
	<p class="lead">{page.paid.lead}</p>
	<p class="paid-note muted">{page.paid.note}</p>
	<div class="offer-grid paid-grid">
		{#each page.paid.offers as offer, i (offer.id)}
			<article class="panel offer" class:featured={offer.featured} style="--offer-i: {i}">
				<p class="eyebrow scope">{offer.badge}</p>
				<h3>{offer.name}</h3>
				<p class="offer-blurb">{offer.blurb}</p>
				<p class="price">
					<span class="amount">{offer.priceLabel}</span>{#if offer.period}<span class="period"
							>{offer.period}</span
						>{/if}
				</p>
				<Button href={offer.ctaHref}>{offer.ctaLabel}</Button>
			</article>
		{/each}
	</div>
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

	.alpha-band {
		margin: var(--space-6) 0 0;
		border-color: var(--accent);
		box-shadow: 0 4px 0 color-mix(in srgb, var(--accent) 25%, transparent);
	}

	.alpha-band h2 {
		margin: 0 0 var(--space-3);
		font-size: clamp(1.35rem, 2.5vw, 1.75rem);
	}

	.alpha-lead {
		margin-bottom: var(--space-4);
		max-width: 40rem;
	}

	.accent-eyebrow {
		color: var(--accent-text);
	}

	.paid {
		animation: paid-in 0.55s ease-out both;
	}

	.paid-note {
		max-width: 40rem;
		margin-bottom: var(--space-5);
		font-size: 0.95rem;
	}

	.offer-grid {
		display: grid;
		gap: var(--space-4);
	}

	@media (min-width: 640px) {
		.paid-grid {
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

	@keyframes paid-in {
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
		.paid,
		.offer {
			animation: none;
		}
	}
</style>
