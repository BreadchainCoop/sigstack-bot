<script lang="ts">
	import type { Snippet } from 'svelte';
	import { resolve } from '$app/paths';
	import type { Pathname } from '$app/types';

	type Variant = 'primary' | 'ghost';

	let {
		href,
		variant = 'primary',
		children
	}: {
		href: string;
		variant?: Variant;
		children: Snippet;
	} = $props();
</script>

<a class="btn btn-{variant}" href={resolve(href as Pathname)}>
	{@render children()}
</a>
