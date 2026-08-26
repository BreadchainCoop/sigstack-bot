<script lang="ts">
	import { getLocale } from '$lib/paraglide/runtime';
	import { getContent } from '$lib/content';
	import FaqList from '$lib/components/FaqList.svelte';

	const { faq: page, meta } = $derived(getContent(getLocale()));
</script>

<svelte:head>
	<title>{page.title} — {meta.siteName}</title>
	<meta name="description" content="Frequently asked questions about Bread Bot." />
</svelte:head>

<p class="eyebrow">Questions</p>
<h1>{page.title}</h1>
<FaqList items={page.items} />
