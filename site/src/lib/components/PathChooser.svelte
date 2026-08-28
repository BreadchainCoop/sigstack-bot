<script lang="ts">
	import type { PathCard } from '$lib/content/types';
	import Button from './Button.svelte';
	import { ArrowRight } from 'phosphor-svelte';

	let { paths, heading }: { paths: PathCard[]; heading: string } = $props();
</script>

<section class="stack path-chooser" aria-labelledby="path-chooser-heading">
	<h2 id="path-chooser-heading">{heading}</h2>
	<div class="grid-3">
		{#each paths as path (path.id)}
			<article class="panel" class:primary={path.primary}>
				{#if path.primary}
					<p class="eyebrow">Recommended</p>
				{/if}
				<h3>{path.title}</h3>
				<p>{path.blurb}</p>
				<Button href={path.href} variant={path.primary ? 'primary' : 'ghost'}>
					Learn more
					<ArrowRight size={18} weight="bold" aria-hidden="true" />
				</Button>
			</article>
		{/each}
	</div>
</section>

<style>
	.path-chooser {
		margin-bottom: var(--space-7);
	}

	.panel.primary {
		border-color: var(--accent);
		box-shadow: 0 4px 0 color-mix(in srgb, var(--accent) 25%, transparent);
	}

	.panel p:not(.eyebrow) {
		color: var(--muted);
	}
</style>
