<script lang="ts">
	import type { FaqItem } from '$lib/content/types';

	let { items }: { items: FaqItem[] } = $props();
</script>

<div class="faq">
	{#each items as item (item.q)}
		<details>
			<summary>{item.q}</summary>
			<p>{item.a}</p>
		</details>
	{/each}
</div>
