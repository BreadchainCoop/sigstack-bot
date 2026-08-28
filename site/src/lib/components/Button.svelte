<script lang="ts">
	import type { Snippet } from 'svelte';
	import { resolve } from '$app/paths';
	import type { Pathname } from '$app/types';

	type Variant = 'primary' | 'ghost';

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

	const resolved = $derived.by(() => {
		if (external || href.startsWith('http')) return href;
		const hashIndex = href.indexOf('#');
		if (hashIndex === -1) return resolve(href as Pathname);
		const path = href.slice(0, hashIndex) || '/';
		const hash = href.slice(hashIndex);
		return `${resolve(path as Pathname)}${hash}`;
	});
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
