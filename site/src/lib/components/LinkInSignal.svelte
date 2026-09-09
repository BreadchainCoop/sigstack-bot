<script lang="ts">
	import { copyLinkCommand, linkCommand } from '$lib/checkoutLanding';
	import Button from '$lib/components/Button.svelte';

	let {
		code,
		heading,
		body,
		steps,
		copyLabel,
		copyDoneLabel,
		messageCta,
		signalLinkMissing,
		signalUsernameLink
	}: {
		code: string;
		heading: string;
		body: string;
		steps: string[];
		copyLabel: string;
		copyDoneLabel: string;
		messageCta: string;
		signalLinkMissing: string;
		signalUsernameLink: string;
	} = $props();

	let copied = $state(false);
	let copyTimer: ReturnType<typeof setTimeout> | undefined;

	async function onCopy() {
		const ok = await copyLinkCommand(code);
		if (!ok) return;
		copied = true;
		clearTimeout(copyTimer);
		copyTimer = setTimeout(() => {
			copied = false;
		}, 2000);
	}
</script>

<section class="panel link-panel" aria-labelledby="link-heading">
	<h2 id="link-heading">{heading}</h2>
	<p>{body}</p>
	{#if steps.length}
		<ol class="link-steps">
			{#each steps as step (step)}
				<li>{step}</li>
			{/each}
		</ol>
	{/if}
	<p class="link-cmd"><code>{linkCommand(code)}</code></p>
	<div class="cta-row link-actions">
		<button type="button" class="btn btn-ghost" onclick={onCopy}>
			{copied ? copyDoneLabel : copyLabel}
		</button>
		{#if signalUsernameLink}
			<Button href={signalUsernameLink} external>{messageCta}</Button>
		{:else}
			<p class="missing-link muted" role="status">{signalLinkMissing}</p>
		{/if}
	</div>
</section>

<style>
	.link-panel h2 {
		margin-top: 0;
		font-size: 1.15rem;
	}

	.link-steps {
		margin: var(--space-3) 0 0;
		padding-left: 1.25rem;
		color: var(--muted);
		display: grid;
		gap: var(--space-2);
	}

	.link-cmd {
		margin: var(--space-4) 0 0;
	}

	.link-cmd code {
		font-size: 1.05rem;
		padding: 0.35em 0.55em;
	}

	.link-actions {
		margin-top: var(--space-4);
		align-items: center;
		flex-wrap: wrap;
		gap: var(--space-3);
	}

	.missing-link {
		margin: 0;
		max-width: 28rem;
		font-size: 0.95rem;
	}
</style>
