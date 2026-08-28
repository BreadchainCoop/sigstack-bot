<script lang="ts">
	let {
		id,
		menu,
		label = 'command list'
	}: {
		id: string;
		menu: string;
		label?: string;
	} = $props();

	let open = $state(false);
	const panelId = $derived(`${id}-panel`);

	function onKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && open) {
			open = false;
		}
	}
</script>

<svelte:window onkeydown={onKeydown} />

<div class="command-list">
	<button
		type="button"
		class="btn btn-ghost compact"
		aria-expanded={open}
		aria-controls={panelId}
		onclick={() => (open = !open)}
	>
		{label}
	</button>
	{#if open}
		<pre id={panelId} class="menu" role="region" aria-label={label}>{menu}</pre>
	{/if}
</div>

<style>
	/* Participate in parent .footer flex so the open panel can span full width. */
	.command-list {
		display: contents;
	}

	.command-list :global(.btn.compact) {
		padding: 0.45rem 0.85rem;
		font-size: 0.875rem;
		white-space: nowrap;
		align-self: flex-start;
	}

	.menu {
		flex: 1 1 100%;
		margin: 0;
		padding: var(--space-4);
		box-sizing: border-box;
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		color: var(--fg);
		font-family: ui-monospace, 'SF Mono', Menlo, Consolas, monospace;
		font-size: 0.8rem;
		line-height: 1.45;
		white-space: pre-wrap;
		overflow-x: auto;
		max-width: 100%;
	}

	/* Sync with --bp-md (640px) in app.css */
	@media (max-width: 639px) {
		.menu {
			font-size: 0.75rem;
			padding: var(--space-3);
		}
	}
</style>
