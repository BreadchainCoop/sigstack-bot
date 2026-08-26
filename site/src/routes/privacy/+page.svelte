<script lang="ts">
	import { getLocale } from '$lib/paraglide/runtime';
	import { getContent } from '$lib/content';

	const { privacy: page, meta } = $derived(getContent(getLocale()));
</script>

<svelte:head>
	<title>{page.title} — {meta.siteName}</title>
	<meta name="description" content={page.lead} />
</svelte:head>

<p class="eyebrow">Trust</p>
<h1>{page.title}</h1>
<p class="lead">{page.lead}</p>

<div class="grid-2" style="margin-top: var(--space-6)">
	{#each page.points as point (point.title)}
		<article class="panel">
			<h2>{point.title}</h2>
			<p>{point.body}</p>
		</article>
	{/each}
</div>

<p class="panel" style="margin-top: var(--space-6)">{page.honest}</p>
