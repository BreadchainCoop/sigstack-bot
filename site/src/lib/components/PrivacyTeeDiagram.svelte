<figure class="diagram diagram-panel" aria-labelledby="tee-diagram-title">
	<p id="tee-diagram-title" class="sr-only">
		A cloud host surrounds a sealed TEE. The host cannot read memory inside the sealed region where
		CipherSlate runs.
	</p>

	<svg viewBox="0 0 520 280" role="img" aria-hidden="true" class="canvas">
		<!-- Outer host -->
		<rect x="48" y="28" width="424" height="220" rx="10" class="card" />
		<text x="260" y="54" text-anchor="middle" class="label">Cloud host (Phala)</text>
		<text x="260" y="72" text-anchor="middle" class="tag">rents the machine · cannot read inside</text>

		<!-- Sealed TEE -->
		<rect x="120" y="100" width="280" height="120" rx="10" class="bubble bubble-sealed" />
		<text x="260" y="132" text-anchor="middle" class="who">Trusted Execution Environment</text>
		<text x="260" y="156" text-anchor="middle" class="msg">Encrypted memory · sealed by the chip</text>
		<text x="260" y="180" text-anchor="middle" class="msg">CipherSlate runs here</text>
		<text x="260" y="200" text-anchor="middle" class="hint">host blocked from reading plaintext</text>
	</svg>

	<figcaption class="footer diagram-footer">
		<div class="caption">
			<ul class="rules">
				<li><strong>Isolation:</strong> host and hypervisor cannot read TEE memory</li>
				<li><strong>Attestation:</strong> remote parties can verify which software is sealed inside</li>
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
