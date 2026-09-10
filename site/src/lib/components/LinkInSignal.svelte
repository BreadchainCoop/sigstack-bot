<script lang="ts">
	import { copyLinkCommand, linkCommand } from '$lib/checkoutLanding';
	import Button from '$lib/components/Button.svelte';

	let {
		code,
		heading,
		body,
		copyLabel,
		copyDoneLabel,
		messageCta,
		signalLinkMissing,
		signalUsernameLink,
		nextStep
	}: {
		code: string;
		heading: string;
		body: string;
		copyLabel: string;
		copyDoneLabel: string;
		messageCta: string;
		signalLinkMissing: string;
		signalUsernameLink: string;
		nextStep?: string;
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
	<div class="cmd-row">
		<code class="cmd">{linkCommand(code)}</code>
		<button
			type="button"
			class="copy-icon"
			onclick={onCopy}
			aria-label={copied ? copyDoneLabel : copyLabel}
			title={copied ? copyDoneLabel : copyLabel}
		>
			{#if copied}
				<svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
					<path
						fill="currentColor"
						d="M9.55 18.2 4.8 13.45l1.4-1.4 3.35 3.3L17.8 7.1l1.4 1.4z"
					/>
				</svg>
			{:else}
				<svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
					<path
						fill="currentColor"
						d="M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z"
					/>
				</svg>
			{/if}
		</button>
		<span class="visually-hidden" aria-live="polite">{copied ? copyDoneLabel : ''}</span>
	</div>
	<div class="cta-row link-actions">
		{#if signalUsernameLink}
			<Button href={signalUsernameLink} external>{messageCta}</Button>
		{:else}
			<p class="missing-link muted" role="status">{signalLinkMissing}</p>
		{/if}
	</div>
	{#if nextStep}
		<p class="next-step muted">{nextStep}</p>
	{/if}
</section>

<style>
	.link-panel h2 {
		margin-top: 0;
		font-size: 1.15rem;
	}

	.cmd-row {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		margin: var(--space-4) 0 0;
		flex-wrap: wrap;
	}

	.cmd {
		font-size: 1.05rem;
		padding: 0.35em 0.55em;
	}

	.copy-icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 2.25rem;
		height: 2.25rem;
		padding: 0;
		color: var(--fg);
		background: transparent;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		cursor: pointer;
	}

	.copy-icon:hover {
		border-color: var(--muted);
	}

	.copy-icon:focus-visible {
		outline: 2px solid var(--accent-2);
		outline-offset: 2px;
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

	.next-step {
		margin: var(--space-4) 0 0;
		max-width: 36rem;
		font-size: 0.95rem;
	}

	.visually-hidden {
		position: absolute;
		width: 1px;
		height: 1px;
		padding: 0;
		margin: -1px;
		overflow: hidden;
		clip: rect(0, 0, 0, 0);
		white-space: nowrap;
		border: 0;
	}
</style>
