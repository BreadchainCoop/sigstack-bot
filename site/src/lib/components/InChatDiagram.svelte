<script lang="ts">
	import ProductCommandList from '$lib/components/ProductCommandList.svelte';
	import { inChatMenu } from '$lib/content/productMenus';
</script>

<figure class="diagram diagram-panel" aria-labelledby="ic-diagram-title">
	<p id="ic-diagram-title" class="sr-only">
		In-chat translation: one bilingual Signal group. An English message stays in place; the bot
		quote-replies with a Spanish translation.
	</p>

	<svg viewBox="0 0 520 280" role="img" aria-hidden="true" class="canvas">
		<g class="node">
			<rect x="80" y="24" width="360" height="220" rx="10" class="card" />
			<text x="260" y="52" text-anchor="middle" class="label">Campaign · bilingual</text>
			<text x="260" y="70" text-anchor="middle" class="tag">same group</text>

			<!-- Original message -->
			<rect x="108" y="92" width="240" height="48" rx="8" class="bubble" />
			<text x="120" y="110" class="who">Organizer</text>
			<text x="120" y="126" class="msg">Meeting at the plaza at 3</text>

			<!-- Quote reply -->
			<rect x="128" y="156" width="264" height="64" rx="8" class="bubble bubble-reply" />
			<text x="140" y="174" class="who">Bread Bot · quote-reply</text>
			<text x="140" y="194" class="msg">🇪🇸 Reunión en la plaza a las 3</text>
			<text x="140" y="210" class="hint">original stays · translation below</text>
		</g>
	</svg>

	<figcaption class="footer diagram-footer">
		<div class="caption">
			<ul class="rules">
				<li><strong>Same group:</strong> translation is a quote-reply; the original stays</li>
				<li><strong>Pair:</strong> group-wide or personal auto for two languages</li>
				<li><strong>Manual:</strong> quote <code>!translate</code> anytime</li>
			</ul>
		</div>
		<ProductCommandList id="ic-commands" menu={inChatMenu} />
	</figcaption>
</figure>

<style>
	.canvas {
		display: block;
		width: 100%;
		max-width: 36rem;
		height: auto;
		margin-inline: auto;
	}

	.card {
		fill: var(--bg);
		stroke: var(--border);
		stroke-width: 1.25;
	}

	.label {
		fill: var(--fg);
		font-family: var(--font-sans);
		font-size: 12px;
		font-weight: 700;
	}

	.tag {
		fill: var(--accent-text);
		font-family: var(--font-sans);
		font-size: 10px;
		font-weight: 600;
		letter-spacing: 0.04em;
		text-transform: uppercase;
	}

	.bubble {
		fill: color-mix(in srgb, var(--accent) 12%, var(--surface));
		stroke: var(--border);
		stroke-width: 1;
	}

	.bubble-reply {
		fill: color-mix(in srgb, var(--accent-2) 14%, var(--surface));
	}

	.who {
		fill: var(--muted);
		font-family: var(--font-sans);
		font-size: 9px;
		font-weight: 700;
	}

	.msg {
		fill: var(--fg);
		font-family: var(--font-sans);
		font-size: 11px;
	}

	.hint {
		fill: var(--muted);
		font-family: var(--font-sans);
		font-size: 9px;
	}

	.caption {
		margin: 0;
		flex: 1 1 12rem;
		font-size: 0.95rem;
		color: var(--muted);
		max-width: 42rem;
	}

	.rules {
		margin: 0;
		padding-left: 1.15rem;
		display: grid;
		gap: var(--space-2);
	}

	.rules strong {
		color: var(--fg);
		font-weight: 600;
	}

	.rules code {
		font-size: 0.9em;
		color: var(--fg);
	}

	.sr-only {
		position: absolute;
		width: 1px;
		height: 1px;
		padding: 0;
		margin: -1px;
		overflow: hidden;
		clip: rect(0, 0, 0, 0);
		border: 0;
	}
</style>
