<figure class="diagram diagram-panel" aria-labelledby="promise-diagram-title">
	<p id="promise-diagram-title" class="sr-only">
		Comparison of a policy promise versus hardware-backed proof with in-chat verification.
	</p>

	<svg viewBox="0 0 520 280" role="img" aria-hidden="true" class="canvas">
		<rect x="40" y="24" width="440" height="220" rx="10" class="card" />
		<text x="260" y="52" text-anchor="middle" class="label">Trust model</text>
		<text x="260" y="70" text-anchor="middle" class="tag">policy vs hardware</text>

		<!-- Promise column -->
		<rect x="60" y="96" width="180" height="120" rx="8" class="bubble" />
		<text x="150" y="124" text-anchor="middle" class="who">Typical cloud bot</text>
		<text x="150" y="148" text-anchor="middle" class="msg">Privacy policy</text>
		<text x="150" y="168" text-anchor="middle" class="msg">Legal agreement</text>
		<text x="150" y="192" text-anchor="middle" class="hint">you trust the vendor</text>

		<!-- Proof column -->
		<rect x="280" y="96" width="180" height="120" rx="8" class="bubble bubble-sealed" />
		<text x="370" y="124" text-anchor="middle" class="who">CipherSlate</text>
		<text x="370" y="148" text-anchor="middle" class="msg">TEE isolation</text>
		<text x="370" y="168" text-anchor="middle" class="msg">!verify challenge</text>
		<text x="370" y="192" text-anchor="middle" class="hint">you can check the box</text>
	</svg>

	<figcaption class="footer diagram-footer">
		<div class="caption">
			<ul class="rules">
				<li><strong>Promise:</strong> written terms the host could still violate in practice</li>
				<li>
					<strong>Proof:</strong> sealed memory plus <code>!verify</code> for this CVM’s compose
				</li>
			</ul>
		</div>
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

	.bubble-sealed {
		fill: color-mix(in srgb, var(--accent) 22%, var(--surface));
		stroke: var(--accent);
		stroke-width: 1.5;
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
