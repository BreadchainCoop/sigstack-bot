<script lang="ts">
	/** Contextual Language Threads architecture animation (main hub + sidecars). */
</script>

<figure class="diagram" aria-labelledby="lt-diagram-title">
	<p id="lt-diagram-title" class="sr-only">
		Language Threads: a multilingual main Signal group fans out to language sidecars. Same-language
		messages relay; other languages are translated.
	</p>

	<svg viewBox="0 0 640 420" role="img" aria-hidden="true" class="canvas">
		<g class="wires" fill="none" stroke-width="1.5" stroke-linecap="round">
			<path class="wire wire-es" d="M320 118 L160 200" />
			<path class="wire wire-en" d="M320 118 L320 200" />
			<path class="wire wire-fr" d="M320 118 L480 200" />
			<circle class="pulse pulse-es" r="4" />
			<circle class="pulse pulse-en" r="4" />
			<circle class="pulse pulse-fr" r="4" />
		</g>

		<!-- Main hub -->
		<g class="node main">
			<rect x="200" y="36" width="240" height="82" rx="10" class="card" />
			<text x="320" y="60" text-anchor="middle" class="label">Main · multilingual</text>
			<rect x="218" y="72" width="204" height="34" rx="8" class="bubble bubble-main" />
			<text x="230" y="86" class="who">Organizer</text>
			<text x="230" y="100" class="msg msg-main">Hola equipo — meeting at 3</text>
		</g>

		<!-- Spanish sidecar (relay) -->
		<g class="node side side-es">
			<rect x="48" y="220" width="180" height="120" rx="10" class="card" />
			<text x="138" y="246" text-anchor="middle" class="label">Spanish · Stacked</text>
			<text x="138" y="264" text-anchor="middle" class="tag tag-relay">relay</text>
			<rect x="60" y="278" width="156" height="44" rx="8" class="bubble bubble-es" />
			<text x="72" y="294" class="who">Organizer</text>
			<text x="72" y="310" class="msg">Hola equipo — meeting at 3</text>
		</g>

		<!-- English sidecar (translate) -->
		<g class="node side side-en">
			<rect x="230" y="220" width="180" height="120" rx="10" class="card" />
			<text x="320" y="246" text-anchor="middle" class="label">English · Stacked</text>
			<text x="320" y="264" text-anchor="middle" class="tag tag-translate">translate</text>
			<rect x="242" y="278" width="156" height="44" rx="8" class="bubble bubble-en" />
			<text x="254" y="294" class="who">Organizer</text>
			<text x="254" y="310" class="msg">Hi team — meeting at 3</text>
		</g>

		<!-- French sidecar (translate) -->
		<g class="node side side-fr">
			<rect x="412" y="220" width="180" height="120" rx="10" class="card" />
			<text x="502" y="246" text-anchor="middle" class="label">French · Stacked</text>
			<text x="502" y="264" text-anchor="middle" class="tag tag-translate">translate</text>
			<rect x="424" y="278" width="156" height="44" rx="8" class="bubble bubble-fr" />
			<text x="436" y="294" class="who">Organizer</text>
			<text x="436" y="310" class="msg">Salut l'equipe — reunion 15h</text>
		</g>

		<g class="legend" transform="translate(48, 372)">
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
		fill: var(--accent-text);
		opacity: 0;
	}

	.wires path {
		stroke: var(--border);
	}

	.pulse {
		fill: var(--accent);
		opacity: 0;
	}

	.bubble {
		fill: color-mix(in srgb, var(--accent) 12%, var(--surface));
		stroke: var(--border);
		stroke-width: 1;
		opacity: 0;
	}

	.who {
		fill: var(--muted);
		font-family: var(--font-sans);
		font-size: 9px;
		font-weight: 700;
		opacity: 0;
	}

	.msg {
		fill: var(--fg);
		font-family: var(--font-sans);
		font-size: 11px;
		opacity: 0;
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

	@keyframes fade-bubble {
		0%,
		8% {
			opacity: 0;
		}
		14%,
		78% {
			opacity: 1;
		}
		88%,
		100% {
			opacity: 0;
		}
	}

	@keyframes fade-late {
		0%,
		28% {
			opacity: 0;
		}
		38%,
		78% {
			opacity: 1;
		}
		88%,
		100% {
			opacity: 0;
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

	.bubble-main,
	.main .who,
	.main .msg {
		animation: fade-bubble 8s ease-in-out infinite;
	}

	.bubble-es,
	.bubble-en,
	.bubble-fr,
	.side .who,
	.side .msg {
		animation: fade-late 8s ease-in-out infinite;
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
		fill: var(--accent-2);
		animation: pulse-en 8s linear infinite;
	}

	.pulse-fr {
		offset-path: path('M320 118 L480 200');
		fill: var(--accent-2);
		animation: pulse-fr 8s linear infinite;
	}

	@media (prefers-reduced-motion: reduce) {
		.bubble,
		.who,
		.msg,
		.tag,
		.wire,
		.pulse,
		.side .card {
			animation: none !important;
		}

		.bubble,
		.who,
		.msg,
		.tag {
			opacity: 1;
		}

		.pulse {
			opacity: 0;
		}
	}
</style>
