<script lang="ts">
	import type { Snippet } from 'svelte';
	import { resolve } from '$app/paths';
	import type { Pathname } from '$app/types';

	type Variant = 'primary' | 'ghost';

	let {
		href,
		variant = 'primary',
		children,
		external = false,
		onclick,
		disabled = false,
		type = 'button'
	}: {
		/** When set, renders an anchor. Omit for a `<button>`. */
		href?: string;
		variant?: Variant;
		children: Snippet;
		external?: boolean;
		onclick?: (e: MouseEvent) => void;
		disabled?: boolean;
		type?: 'button' | 'submit';
	} = $props();

	const resolved = $derived.by(() => {
		if (!href) return '';
		if (external || href.startsWith('http')) return href;
		const hashIndex = href.indexOf('#');
		if (hashIndex === -1) return resolve(href as Pathname);
		const path = href.slice(0, hashIndex) || '/';
		const hash = href.slice(hashIndex);
		return `${resolve(path as Pathname)}${hash}`;
	});
</script>

{#if href}
	{#if external || href.startsWith('http')}
		<a class="btn btn-{variant}" href={resolved} target="_blank" rel="noopener noreferrer">
			{@render children()}
		</a>
	{:else}
		<a class="btn btn-{variant}" href={resolved}>
			{@render children()}
		</a>
	{/if}
{:else}
	<button class="btn btn-{variant}" {type} {disabled} {onclick}>
		{@render children()}
	</button>
{/if}
