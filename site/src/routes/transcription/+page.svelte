<script lang="ts">
	import { getLocale } from '$lib/paraglide/runtime';
	import { getContent } from '$lib/content';
	import CommandTable from '$lib/components/CommandTable.svelte';
	import ProductDiagram from '$lib/components/ProductDiagram.svelte';
	import Button from '$lib/components/Button.svelte';

	const { transcription: page, meta } = $derived(getContent(getLocale()));
</script>

<svelte:head>
	<title>{page.title} — {meta.siteName}</title>
	<meta name="description" content={page.lead} />
</svelte:head>

<p class="eyebrow">Voice</p>
<h1>{page.title}</h1>
<p class="lead">{page.lead}</p>
<p>{page.when}</p>

<ProductDiagram kind="transcription" caption={page.diagramCaption} />

<h2>Message flow</h2>
<ul>
	{#each page.flow as line (line)}
		<li>{line}</li>
	{/each}
</ul>

<h2>Commands</h2>
<CommandTable rows={page.commands} />

<div class="cta-row">
	<Button href="/get-started">Get started</Button>
	<Button href="/privacy" variant="secondary">Privacy details</Button>
</div>
