<script lang="ts">
	import type { PathCard } from '$lib/content/types';
	import Button from './Button.svelte';
	import { ArrowRight } from 'phosphor-svelte';

	let {
		paths,
		heading,
		selectedId = $bindable()
	}: {
		paths: PathCard[];
		heading: string;
		selectedId: string;
	} = $props();

	function select(id: string) {
		selectedId = id;
	}

	function onCardKeydown(event: KeyboardEvent, id: string) {
		if (event.key === 'Enter' || event.key === ' ') {
			event.preventDefault();
			select(id);
			return;
		}

		const i = paths.findIndex((p) => p.id === id);
		if (i < 0) return;

		let next = -1;
		if (event.key === 'ArrowRight' || event.key === 'ArrowDown') {
			next = (i + 1) % paths.length;
		} else if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') {
			next = (i - 1 + paths.length) % paths.length;
		}
		if (next < 0) return;

		event.preventDefault();
		const nextId = paths[next]?.id;
		if (!nextId) return;
		select(nextId);
		const radios = (event.currentTarget as HTMLElement)
			.closest('[role="radiogroup"]')
			?.querySelectorAll<HTMLElement>('[role="radio"]');
		radios?.[next]?.focus();
	}
</script>

<section class="stack path-chooser" aria-labelledby="path-chooser-heading">
	<h2 id="path-chooser-heading">{heading}</h2>
	<div class="grid-3" role="radiogroup" aria-labelledby="path-chooser-heading">
		{#each paths as path (path.id)}
			{@const selected = path.id === selectedId}
			<div
				class="panel"
				class:selected
				role="radio"
				aria-checked={selected}
				tabindex={selected ? 0 : -1}
				onclick={() => select(path.id)}
				onkeydown={(e) => onCardKeydown(e, path.id)}
			>
				{#if path.primary}
					<p class="eyebrow">Recommended</p>
				{/if}
				<h3>{path.title}</h3>
				<p>{path.blurb}</p>
				<div class="cta">
					<Button href={path.href} variant={selected ? 'primary' : 'ghost'}>
						Learn more
						<ArrowRight size={18} weight="bold" aria-hidden="true" />
					</Button>
				</div>
			</div>
		{/each}
	</div>
</section>

<style>
	.path-chooser {
		margin-bottom: 0;
	}

	.panel {
		display: flex;
		flex-direction: column;
		height: 100%;
		cursor: pointer;
		transition:
			border-color 0.15s ease,
			box-shadow 0.15s ease;
	}

	.panel:hover {
		border-color: color-mix(in srgb, var(--accent) 55%, var(--border));
	}

	.panel.selected {
		border-color: var(--accent);
		box-shadow: 0 4px 0 color-mix(in srgb, var(--accent) 25%, transparent);
	}

	.panel:focus-visible {
		outline: 2px solid var(--accent-2);
		outline-offset: 2px;
	}

	.panel p:not(.eyebrow) {
		color: var(--muted);
		flex: 1;
	}

	.cta {
		position: relative;
		z-index: 1;
		margin-top: var(--space-4);
	}
</style>
