<script lang="ts">
	import { getLocale } from '$lib/paraglide/runtime';
	import { getContent } from '$lib/content';

	const { legalPrivacy: page, meta } = $derived(getContent(getLocale()));
</script>

<svelte:head>
	<title>{page.title} — {meta.siteName}</title>
</svelte:head>

<p class="eyebrow">Updated {page.updated}</p>
<h1>{page.title}</h1>

{#each page.sections as section (section.title)}
	<section style="margin-top: var(--space-6)">
		<h2>{section.title}</h2>
		<p>{section.body}</p>
	</section>
{/each}
