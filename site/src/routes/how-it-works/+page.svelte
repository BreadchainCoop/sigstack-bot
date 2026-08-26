<script lang="ts">
	import { getLocale } from '$lib/paraglide/runtime';
	import { getContent } from '$lib/content';

	const { howItWorks, meta } = $derived(getContent(getLocale()));
</script>

<svelte:head>
	<title>{howItWorks.title} — {meta.siteName}</title>
	<meta name="description" content={howItWorks.lead} />
</svelte:head>

<p class="eyebrow">Guide</p>
<h1>{howItWorks.title}</h1>
<p class="lead">{howItWorks.lead}</p>
<p>{howItWorks.oneBot}</p>

{#each howItWorks.sections as section (section.title)}
	<section class="stack" style="margin-top: var(--space-7)">
		<h2>{section.title}</h2>
		<ol class="steps">
			{#each section.steps as step (step.title)}
				<li>
					<strong>{step.title}</strong>
					<p>{step.body}</p>
				</li>
			{/each}
		</ol>
	</section>
{/each}
