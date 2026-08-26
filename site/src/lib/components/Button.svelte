<script lang="ts">
	import type { Snippet } from 'svelte';
	import { resolve } from '$app/paths';
	import type { Pathname } from '$app/types';

	type Variant = 'primary' | 'secondary' | 'ghost';

	let {
		href,
		variant = 'primary',
		children,
		external = false
	}: {
		href: string;
		variant?: Variant;
		children: Snippet;
		external?: boolean;
	} = $props();

	const resolved = $derived(
		external || href.startsWith('http') ? href : resolve(href as Pathname)
	);
</script>

{#if external || href.startsWith('http')}
	<a class="btn btn-{variant}" href={resolved} target="_blank" rel="noopener noreferrer">
		{@render children()}
	</a>
{:else}
	<a class="btn btn-{variant}" href={resolved}>
		{@render children()}
	</a>
{/if}

<style>
	.btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 0.4rem;
		padding: 0.7rem 1.15rem;
		font-family: var(--font-body);
		font-weight: 700;
		font-size: 0.95rem;
		text-decoration: none;
		border: 2px solid transparent;
		cursor: pointer;
		transition:
			background 0.15s ease,
			color 0.15s ease,
			border-color 0.15s ease;
	}

	.btn-primary {
		background: var(--color-orange-2);
		color: var(--color-white);
		border-color: var(--color-orange-2);
	}

	.btn-primary:hover {
		background: var(--color-brown);
		border-color: var(--color-brown);
		color: var(--color-white);
	}

	.btn-secondary {
		background: transparent;
		color: var(--color-ink);
		border-color: var(--color-ink);
	}

	.btn-secondary:hover {
		background: var(--color-ink);
		color: var(--color-white);
	}

	.btn-ghost {
		background: transparent;
		color: var(--color-primary-jade);
		border-color: transparent;
		padding-inline: 0.35rem;
	}

	.btn-ghost:hover {
		color: var(--color-jade-2);
	}
</style>
