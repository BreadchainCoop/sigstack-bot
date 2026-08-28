<script lang="ts">
	/** Contextual Language Threads architecture animation (main hub + sidecars). */
</script>

<figure class="diagram" aria-labelledby="lt-diagram-title">
	<p id="lt-diagram-title" class="sr-only">
		Language Threads: a multilingual main Signal group fans out to language sidecars. Same-language
		messages relay; other languages are translated.
	</p>

	<svg viewBox="0 0 640 420" role="img" aria-hidden="true" class="canvas">
		<!-- connectors (drawn under nodes) -->
		<g class="wires" fill="none" stroke-width="1.5" stroke-linecap="round">
			<path class="wire wire-es" d="M320 118 L160 200" />
			<path class="wire wire-en" d="M320 118 L320 200" />
			<path class="wire wire-fr" d="M320 118 L480 200" />
			<!-- pulse dots traveling along paths -->
			<circle class="pulse pulse-es" r="4" />
			<circle class="pulse pulse-en" r="4" />
			<circle class="pulse pulse-fr" r="4" />
		</g>

		<!-- Main hub -->
		<g class="node main">
			<rect x="200" y="36" width="240" height="82" rx="10" class="card" />
			<text x="320" y="62" text-anchor="middle" class="label">Main · multilingual</text>
			<foreignObject x="214" y="72" width="212" height="36">
				<div xmlns="http://www.w3.org/1999/xhtml" class="bubble bubble-main">
					<span class="who">Organizer</span>
					<span class="msg msg-main">Hola equipo — meeting at 3</span>
				</div>
			</foreignObject>
		</g>

		<!-- Spanish sidecar -->
		<g class="node side side-es">
			<rect x="48" y="220" width="180" height="120" rx="10" class="card" />
			<text x="138" y="246" text-anchor="middle" class="label">Spanish · Stacked</text>
			<text x="138" y="264" text-anchor="middle" class="tag tag-relay">relay</text>
			<foreignObject x="60" y="276" width="156" height="50">
				<div xmlns="http://www.w3.org/1999/xhtml" class="bubble bubble-es">
					<span class="who">Organizer</span>
					<span class="msg">Hola equipo — meeting at 3</span>
				</div>
			</foreignObject>
		</g>

		<!-- English sidecar -->
		<g class="node side side-en">
			<rect x="230" y="220" width="180" height="120" rx="10" class="card" />
			<text x="320" y="246" text-anchor="middle" class="label">English · Stacked</text>
			<text x="320" y="264" text-anchor="middle" class="tag tag-translate">translate</text>
			<foreignObject x="242" y="276" width="156" height="50">
				<div xmlns="http://www.w3.org/1999/xhtml" class="bubble bubble-en">
					<span class="who">Organizer</span>
					<span class="msg">Hi team — meeting at 3</span>
				</div>
			</foreignObject>
		</g>

		<!-- French sidecar -->
		<g class="node side side-fr">
			<rect x="412" y="220" width="180" height="120" rx="10" class="card" />
			<text x="502" y="246" text-anchor="middle" class="label">French · Stacked</text>
			<text x="502" y="264" text-anchor="middle" class="tag tag-translate">translate</text>
			<foreignObject x="424" y="276" width="156" height="50">
				<div xmlns="http://www.w3.org/1999/xhtml" class="bubble bubble-fr">
					<span class="who">Organizer</span>
					<span class="msg">Salut l’équipe — réunion à 15h</span>
				</div>
			</foreignObject>
		</g>

		<!-- legend -->
		<g class="legend" transform="translate(48, 368)">
			<circle cx="6" cy="0" r="4" class="leg-dot relay" />
			<text x="16" y="4" class="leg-text">same language → relay</text>
			<circle cx="200" cy="0" r="4" class="leg-dot translate" />
			<text x="210" y="4" class="leg-text">other language → translate</text>
		</g>
	</svg>

	<figcaption class="caption">
		One multilingual main Signal group; monolingual members join language sidecars. The bot fans out
		each human message — relay when the language matches, translate otherwise.
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
		height: auto;
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
		font-family: var(--font-sans);
		font-size: 10px;
		font-weight: 600;
		letter-spacing: 0.04em;
		text-transform: uppercase;
		opacity: 0;
	}

	.tag-relay {
		fill: var(--accent-text);
	}

	.tag-translate {
		fill: var(--accent-text);
	}

	.wires path {
		stroke: var(--border);
	}

	.pulse {
		fill: var(--accent);
		opacity: 0;
	}

	.bubble {
		font-family: var(--font-sans);
		font-size: 11px;
		line-height: 1.3;
		color: var(--fg);
		background: color-mix(in srgb, var(--accent) 10%, var(--surface));
		border: 1px solid var(--border);
		border-radius: 8px;
		padding: 6px 8px;
		opacity: 0;
		transform: translateY(4px);
	}

	.who {
		display: block;
		font-size: 9px;
		font-weight: 700;
		color: var(--muted);
		margin-bottom: 2px;
	}

	.msg {
		display: block;
	}

	.leg-text {
		fill: var(--muted);
		font-family: var(--font-sans);
		font-size: 11px;
	}

	.leg-dot.relay {
		fill: var(--accent);
	}

	.leg-dot.translate {
		fill: var(--accent-2);
	}

	.caption {
		margin: var(--space-4) 0 0;
		font-size: 0.95rem;
		color: var(--muted);
		max-width: 42rem;
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

	/* Animation timeline (~8s loop) */
	@keyframes fade-in-up {
		0%,
		8% {
			opacity: 0;
			transform: translateY(6px);
		}
		14%,
		78% {
			opacity: 1;
			transform: translateY(0);
		}
		88%,
		100% {
			opacity: 0;
			transform: translateY(-2px);
		}
	}

	@keyframes fade-in-late {
		0%,
		28% {
			opacity: 0;
			transform: translateY(6px);
		}
		38%,
		78% {
			opacity: 1;
			transform: translateY(0);
		}
		88%,
		100% {
			opacity: 0;
			transform: translateY(-2px);
		}
	}

	@keyframes tag-in {
		0%,
		22% {
			opacity: 0;
		}
		30%,
		78% {
			opacity: 1;
		}
		88%,
		100% {
			opacity: 0;
		}
	}

	@keyframes wire-glow {
		0%,
		18% {
			stroke: var(--border);
			stroke-width: 1.5;
		}
		28%,
		55% {
			stroke: var(--accent);
			stroke-width: 2;
		}
		70%,
		100% {
			stroke: var(--border);
			stroke-width: 1.5;
		}
	}

	@keyframes pulse-es {
		0%,
		16% {
			opacity: 0;
			offset-distance: 0%;
		}
		18% {
			opacity: 1;
		}
		32% {
			opacity: 1;
			offset-distance: 100%;
		}
		34%,
		100% {
			opacity: 0;
			offset-distance: 100%;
		}
	}

	@keyframes pulse-en {
		0%,
		20% {
			opacity: 0;
			offset-distance: 0%;
		}
		22% {
			opacity: 1;
		}
		36% {
			opacity: 1;
			offset-distance: 100%;
		}
		38%,
		100% {
			opacity: 0;
			offset-distance: 100%;
		}
	}

	@keyframes pulse-fr {
		0%,
		24% {
			opacity: 0;
			offset-distance: 0%;
		}
		26% {
			opacity: 1;
		}
		40% {
			opacity: 1;
			offset-distance: 100%;
		}
		42%,
		100% {
			opacity: 0;
			offset-distance: 100%;
		}
	}

	@keyframes card-flash {
		0%,
		30% {
			stroke: var(--border);
		}
		36%,
		48% {
			stroke: var(--accent);
		}
		58%,
		100% {
			stroke: var(--border);
		}
	}

	.bubble-main {
		animation: fade-in-up 8s ease-in-out infinite;
	}

	.bubble-es,
	.bubble-en,
	.bubble-fr {
		animation: fade-in-late 8s ease-in-out infinite;
	}

	.tag {
		animation: tag-in 8s ease-in-out infinite;
	}

	.wire {
		animation: wire-glow 8s ease-in-out infinite;
	}

	.side-es .card,
	.side-en .card,
	.side-fr .card {
		animation: card-flash 8s ease-in-out infinite;
	}

	.side-en .card {
		animation-delay: 0.15s;
	}

	.side-fr .card {
		animation-delay: 0.3s;
	}

	.pulse-es {
		offset-path: path('M320 118 L160 200');
		animation: pulse-es 8s linear infinite;
	}

	.pulse-en {
		offset-path: path('M320 118 L320 200');
		animation: pulse-en 8s linear infinite;
	}

	.pulse-fr {
		offset-path: path('M320 118 L480 200');
		animation: pulse-fr 8s linear infinite;
	}

	.pulse-en {
		fill: var(--accent-2);
	}

	.pulse-fr {
		fill: var(--accent-2);
	}

	@media (prefers-reduced-motion: reduce) {
		.bubble,
		.tag,
		.wire,
		.pulse,
		.side .card {
			animation: none !important;
		}

		.bubble,
		.tag {
			opacity: 1;
			transform: none;
		}

		.pulse {
			opacity: 0;
		}
	}
</style>
