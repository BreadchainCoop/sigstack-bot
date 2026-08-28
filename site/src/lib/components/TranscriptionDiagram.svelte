<script lang="ts">
	import ProductCommandList from '$lib/components/ProductCommandList.svelte';
	import { transcriptionMenu } from '$lib/content/productMenus';
</script>

<figure class="diagram" aria-labelledby="tx-diagram-title">
	<p id="tx-diagram-title" class="sr-only">
		Voice transcription: a voice note in a Signal group becomes a quote-reply transcript from the
		bot.
	</p>

	<svg viewBox="0 0 520 280" role="img" aria-hidden="true" class="canvas">
		<g class="node">
			<rect x="80" y="24" width="360" height="220" rx="10" class="card" />
			<text x="260" y="52" text-anchor="middle" class="label">Mutual aid · voice on</text>
			<text x="260" y="70" text-anchor="middle" class="tag">speech → text</text>

			<!-- Voice note -->
			<rect x="108" y="92" width="220" height="52" rx="8" class="bubble" />
			<text x="120" y="110" class="who">Member</text>
			<text x="120" y="128" class="msg">🎙 Voice note · 0:18</text>

			<!-- Arrow hint -->
			<text x="260" y="168" text-anchor="middle" class="arrow">↓</text>

			<!-- Transcript reply -->
			<rect x="128" y="180" width="280" height="48" rx="8" class="bubble bubble-reply" />
			<text x="140" y="198" class="who">Bread Bot · quote-reply</text>
			<text x="140" y="214" class="msg">📝 Transcript: Supplies at door B</text>
		</g>
	</svg>

	<figcaption class="footer">
		<div class="caption">
			<ul class="rules">
				<li><strong>Opt-in:</strong> auto off until enabled; or quote <code>!transcribe</code></li>
				<li>
					<strong>Privacy-shaped STT:</strong> audio stripped of Signal metadata before Whisper
				</li>
				<li>
					<strong>Composes:</strong> transcript can fan into In-chat or Language Threads like a normal
					post
				</li>
			</ul>
		</div>
		<ProductCommandList id="tx-commands" menu={transcriptionMenu} />
	</figcaption>
</figure>

<style>
	.diagram {
		margin: var(--space-6) 0 0;
		padding: var(--space-5);
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
	}

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

	.arrow {
		fill: var(--muted);
		font-family: var(--font-sans);
		font-size: 14px;
	}

	.footer {
		display: flex;
		flex-wrap: wrap;
		justify-content: space-between;
		align-items: flex-start;
		gap: var(--space-4);
		margin-top: var(--space-4);
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
