<figure class="diagram diagram-panel" aria-labelledby="privacy-flow-title">
	<p id="privacy-flow-title" class="sr-only">
		Signal group messages enter a sealed Phala TEE, then NEAR AI private inference, and replies
		return to the group.
	</p>

	<svg viewBox="0 0 520 280" role="img" aria-hidden="true" class="canvas">
		<rect x="40" y="24" width="440" height="220" rx="10" class="card" />
		<text x="260" y="52" text-anchor="middle" class="label">Processing path</text>
		<text x="260" y="70" text-anchor="middle" class="tag">plaintext only inside sealed boxes</text>

		<!-- Signal -->
		<rect x="60" y="100" width="110" height="88" rx="8" class="bubble" />
		<text x="115" y="128" text-anchor="middle" class="who">Signal group</text>
		<text x="115" y="150" text-anchor="middle" class="msg">E2E chat</text>
		<text x="115" y="168" text-anchor="middle" class="msg">+ voice notes</text>

		<text x="185" y="148" text-anchor="middle" class="arrow">→</text>

		<!-- Phala TEE -->
		<rect x="205" y="100" width="110" height="88" rx="8" class="bubble bubble-sealed" />
		<text x="260" y="128" text-anchor="middle" class="who">Phala TEE</text>
		<text x="260" y="150" text-anchor="middle" class="msg">CipherSlate</text>
		<text x="260" y="168" text-anchor="middle" class="msg">decrypts here</text>

		<text x="330" y="148" text-anchor="middle" class="arrow">→</text>

		<!-- NEAR AI -->
		<rect x="350" y="100" width="110" height="88" rx="8" class="bubble bubble-reply" />
		<text x="405" y="128" text-anchor="middle" class="who">NEAR AI TEE</text>
		<text x="405" y="150" text-anchor="middle" class="msg">Translate</text>
		<text x="405" y="168" text-anchor="middle" class="msg">+ Whisper STT</text>
	</svg>

	<figcaption class="footer diagram-footer">
		<div class="caption">
			<ul class="rules">
				<li><strong>Decrypt in TEE:</strong> Signal plaintext exists only inside the sealed bot</li>
				<li>
					<strong>Strip metadata:</strong> outbound voice has no phone, group id, or filename
				</li>
				<li><strong>Return to Signal:</strong> translations and transcripts post back in-group</li>
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
