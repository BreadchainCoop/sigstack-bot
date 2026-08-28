<script lang="ts">
	import { getLocale } from '$lib/paraglide/runtime';
	import { getContent } from '$lib/content';

	const { legalLicense: page, meta } = $derived(getContent(getLocale()));
</script>

<svelte:head>
	<title>{page.title} — {meta.siteName}</title>
</svelte:head>

<p class="eyebrow">Version dated {page.updated}</p>
<h1>{page.title}</h1>
<p class="copyright">{page.copyright}</p>
<p>{page.overview}</p>

<pre class="license-text">{page.fullText}</pre>

<style>
	.copyright {
		font-weight: 600;
		margin: var(--space-4) 0 var(--space-3);
	}

	.license-text {
		margin: var(--space-6) 0 0;
		padding: var(--space-5);
		overflow-x: auto;
		white-space: pre-wrap;
		word-break: break-word;
		font-family: var(--font-mono, ui-monospace, SFMono-Regular, Menlo, monospace);
		font-size: 0.8rem;
		line-height: 1.45;
		color: var(--fg);
		background: var(--surface);
		border: 1px solid var(--border);
	}
</style>
