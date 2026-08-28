<script lang="ts">
	import { getLocale } from '$lib/paraglide/runtime';
	import { getContent } from '$lib/content';
	import CommandTable from '$lib/components/CommandTable.svelte';
	import Button from '$lib/components/Button.svelte';

	const { pages, meta } = $derived(getContent(getLocale()));
	const page = $derived(pages.getStarted);
</script>

<svelte:head>
	<title>{page.title} — {meta.siteName}</title>
	<meta name="description" content={page.lead} />
</svelte:head>

<p class="eyebrow">{page.eyebrow}</p>
<h1>{page.title}</h1>
<p class="lead">{page.lead}</p>

<ol class="steps" style="margin-top: var(--space-6)">
	{#each page.steps as step (step.title)}
		<li>
			<strong>{step.title}</strong>
			<p>{step.body}</p>
		</li>
	{/each}
</ol>

<h2 style="margin-top: var(--space-7)">{page.hubCommandsHeading}</h2>
<CommandTable rows={page.hubCommands} />

<div class="cta-row">
	<Button href="/products#language-threads">Language Threads</Button>
	<Button href="/privacy" variant="ghost">Privacy</Button>
	<Button href="https://github.com/BreadchainCoop/sigstack-bot/issues" variant="ghost" external
		>GitHub issues</Button
	>
</div>
