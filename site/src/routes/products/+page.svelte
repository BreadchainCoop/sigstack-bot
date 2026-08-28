<script lang="ts">
	import { getLocale } from '$lib/paraglide/runtime';
	import { getContent } from '$lib/content';
	import LanguageThreadsDiagram from '$lib/components/LanguageThreadsDiagram.svelte';
	import InChatDiagram from '$lib/components/InChatDiagram.svelte';
	import TranscriptionDiagram from '$lib/components/TranscriptionDiagram.svelte';

	const { pages, meta } = $derived(getContent(getLocale()));
	const page = $derived(pages.products);
</script>

<svelte:head>
	<title>{page.title} — {meta.siteName}</title>
</svelte:head>

<section class="page-stub">
	<h1>{page.title}</h1>
	<p class="lead">{page.lead}</p>
</section>

{#each page.sections as section (section.id)}
	<section class="section" id={section.id}>
		<h2>{section.title}</h2>
		<p class="lead">{section.lead}</p>
		{#if section.id === 'language-threads'}
			<LanguageThreadsDiagram />
		{:else if section.id === 'in-chat'}
			<InChatDiagram />
		{:else if section.id === 'transcription'}
			<TranscriptionDiagram />
		{/if}
	</section>
{/each}
