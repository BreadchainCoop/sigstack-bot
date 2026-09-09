<script lang="ts">
	import {
		languageCountForFeature,
		languagesForFeature,
		LANGUAGE_FEATURE_LABELS,
		formatLanguageLine,
		type LanguageFeature
	} from '$lib/content/languages';

	let {
		id,
		defaultFeature = 'threads'
	}: {
		id: string;
		defaultFeature?: LanguageFeature;
	} = $props();

	let feature = $state<LanguageFeature>('threads');

	$effect.pre(() => {
		feature = defaultFeature;
	});

	const features: LanguageFeature[] = ['threads', 'inChatAuto', 'manual'];

	const filtered = $derived(languagesForFeature(feature));
	const meta = $derived(LANGUAGE_FEATURE_LABELS[feature]);
	const count = $derived(languageCountForFeature(feature));
</script>

<div class="catalog" aria-labelledby="{id}-title">
	<p id="{id}-title" class="callout">
		<strong>Different products, different language lists.</strong>
		Language Threads supports more languages than in-chat auto-translate. In-chat auto requires
		local text detection — Basque and Swahili are available in Language Threads and manual
		<code>!translate</code> only.
	</p>

	<label class="filter" for="{id}-feature">
		<span class="filter-label">Show languages for</span>
		<select id="{id}-feature" bind:value={feature}>
			{#each features as f (f)}
				<option value={f}>{LANGUAGE_FEATURE_LABELS[f].label}</option>
			{/each}
		</select>
	</label>

	<p class="meta">
		<span class="count">{count} languages</span>
		· {meta.hint}
		· In Signal: <code>{meta.botCommand}</code>
	</p>

	<ul class="list" aria-label="{meta.label} languages">
		{#each filtered as lang (lang.code)}
			<li>{formatLanguageLine(lang)}</li>
		{/each}
	</ul>
</div>

<style>
	.catalog {
		margin-top: var(--space-4);
		padding: var(--space-4);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: var(--surface);
	}

	.callout {
		margin: 0 0 var(--space-3);
		font-size: 0.95rem;
		color: var(--muted);
		line-height: 1.5;
	}

	.callout strong {
		color: var(--fg);
	}

	.filter {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-2);
		margin-bottom: var(--space-2);
	}

	.filter-label {
		font-size: 0.875rem;
		font-weight: 600;
		color: var(--fg);
	}

	select {
		font: inherit;
		padding: 0.35rem 0.6rem;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		background: var(--bg);
		color: var(--fg);
	}

	.meta {
		margin: 0 0 var(--space-3);
		font-size: 0.875rem;
		color: var(--muted);
	}

	.count {
		font-weight: 600;
		color: var(--fg);
	}

	code {
		font-size: 0.9em;
		color: var(--fg);
	}

	.list {
		margin: 0;
		padding: 0;
		list-style: none;
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(11rem, 1fr));
		gap: var(--space-1) var(--space-3);
		font-size: 0.875rem;
		color: var(--fg);
	}
</style>
