<script lang="ts">
	/** Language Threads: main→sidecars, then sidecar reply→main + other sidecars. */
	import ProductCommandList from '$lib/components/ProductCommandList.svelte';
	import { threadsMenu } from '$lib/content/productMenus';
</script>

<figure class="diagram diagram-panel" aria-labelledby="lt-diagram-title">
	<p id="lt-diagram-title" class="sr-only">
		Language Threads animation. First a message from the multilingual main fans out to language
		sidecars. Then a Spanish sidecar reply relays into main and translates into the other sidecars.
	</p>

	<!-- Wide: hub-and-spoke (sync show/hide with --bp-lg / 980px) -->
	<svg viewBox="0 0 780 520" role="img" aria-hidden="true" class="canvas layout-wide">
		<g class="wires" fill="none" stroke-width="1.5" stroke-linecap="round">
			<path class="wire wire-hub-es" d="M390 166 L120 250" />
			<path class="wire wire-hub-en" d="M390 166 L390 250" />
			<path class="wire wire-hub-fr" d="M390 166 L660 250" />
			<path class="wire wire-es-en" d="M210 340 L300 340" />
			<path class="wire wire-es-fr" d="M210 358 L570 358" />
		</g>

		<circle class="pulse pulse-down-es pulse-wide" r="4" />
		<circle class="pulse pulse-down-en pulse-wide" r="4" />
		<circle class="pulse pulse-down-fr pulse-wide" r="4" />
		<circle class="pulse pulse-up-main pulse-wide" r="4" />
		<circle class="pulse pulse-across-en pulse-wide" r="4" />
		<circle class="pulse pulse-across-fr pulse-wide" r="4" />

		<g class="node main">
			<rect x="270" y="20" width="240" height="138" rx="10" class="card card-main" />
			<text x="390" y="42" text-anchor="middle" class="label">Main · multilingual</text>
			<text x="390" y="58" text-anchor="middle" class="tag tag-main-p2">relay from thread</text>

			<g class="phase phase1-main">
				<rect x="288" y="68" width="204" height="34" rx="8" class="bubble" />
				<text x="300" y="82" class="who">Organizer</text>
				<text x="300" y="96" class="msg">Hola equipo — meeting at 3</text>
			</g>
			<g class="phase phase2-main">
				<rect x="288" y="110" width="204" height="34" rx="8" class="bubble" />
				<text x="300" y="124" class="who">Ana · Spanish</text>
				<text x="300" y="138" class="msg">¿A qué hora exactamente?</text>
			</g>
		</g>

		<g class="node side side-es">
			<rect x="30" y="270" width="180" height="168" rx="10" class="card card-es" />
			<text x="120" y="294" text-anchor="middle" class="label">Spanish · Stacked</text>
			<text x="120" y="312" text-anchor="middle" class="tag tag-es-p1">relay</text>
			<text x="120" y="312" text-anchor="middle" class="tag tag-es-p2">reply</text>

			<g class="phase phase1-es">
				<rect x="42" y="326" width="156" height="40" rx="8" class="bubble" />
				<text x="54" y="342" class="who">Organizer</text>
				<text x="54" y="356" class="msg">Hola equipo — meeting at 3</text>
			</g>
			<g class="phase phase2-es">
				<rect x="42" y="376" width="156" height="46" rx="8" class="bubble" />
				<text x="54" y="392" class="who">Ana</text>
				<text x="54" y="408" class="msg">¿A qué hora exactamente?</text>
			</g>
		</g>

		<g class="node side side-en">
			<rect x="300" y="270" width="180" height="168" rx="10" class="card card-en" />
			<text x="390" y="294" text-anchor="middle" class="label">English · Stacked</text>
			<text x="390" y="312" text-anchor="middle" class="tag tag-both">translate</text>

			<g class="phase phase1-en">
				<rect x="312" y="326" width="156" height="40" rx="8" class="bubble" />
				<text x="324" y="342" class="who">Organizer</text>
				<text x="324" y="356" class="msg">Hi team — meeting at 3</text>
			</g>
			<g class="phase phase2-en">
				<rect x="312" y="376" width="156" height="46" rx="8" class="bubble" />
				<text x="324" y="392" class="who">Ana</text>
				<text x="324" y="408" class="msg">What time exactly?</text>
			</g>
		</g>

		<g class="node side side-fr">
			<rect x="570" y="270" width="180" height="168" rx="10" class="card card-fr" />
			<text x="660" y="294" text-anchor="middle" class="label">French · Stacked</text>
			<text x="660" y="312" text-anchor="middle" class="tag tag-both">translate</text>

			<g class="phase phase1-fr">
				<rect x="582" y="326" width="156" height="40" rx="8" class="bubble" />
				<text x="594" y="342" class="who">Organizer</text>
				<text x="594" y="356" class="msg">Salut l'equipe — reunion 15h</text>
			</g>
			<g class="phase phase2-fr">
				<rect x="582" y="376" width="156" height="46" rx="8" class="bubble" />
				<text x="594" y="392" class="who">Ana</text>
				<text x="594" y="408" class="msg">A quelle heure exactement?</text>
			</g>
		</g>

		<g class="legend" transform="translate(30, 460)">
			<circle cx="6" cy="0" r="4" class="leg-dot relay" />
			<text x="16" y="4" class="leg-text">same language → relay</text>
			<circle cx="200" cy="0" r="4" class="leg-dot translate" />
			<text x="210" y="4" class="leg-text">other language → translate</text>
			<text x="16" y="24" class="leg-text">Loop: main posts, then a Spanish thread replies</text>
		</g>
	</svg>

	<!-- Narrow: stacked cards for readable labels on phones -->
	<svg viewBox="0 0 360 820" role="img" aria-hidden="true" class="canvas layout-narrow">
		<g class="wires" fill="none" stroke-width="1.5" stroke-linecap="round">
			<path class="wire wire-hub-es" d="M170 154 L170 200" />
			<path class="wire wire-hub-en" d="M300 154 L300 400" />
			<path class="wire wire-hub-fr" d="M60 154 L60 600" />
			<path class="wire wire-es-en" d="M180 368 L180 400" />
			<path class="wire wire-es-fr" d="M60 368 L60 600 L180 600" />
		</g>

		<circle class="pulse pulse-down-es pulse-narrow" r="4" />
		<circle class="pulse pulse-down-en pulse-narrow" r="4" />
		<circle class="pulse pulse-down-fr pulse-narrow" r="4" />
		<circle class="pulse pulse-up-main pulse-narrow" r="4" />
		<circle class="pulse pulse-across-en pulse-narrow" r="4" />
		<circle class="pulse pulse-across-fr pulse-narrow" r="4" />

		<g class="node main">
			<rect x="40" y="16" width="280" height="138" rx="10" class="card card-main" />
			<text x="180" y="40" text-anchor="middle" class="label">Main · multilingual</text>
			<text x="180" y="56" text-anchor="middle" class="tag tag-main-p2">relay from thread</text>

			<g class="phase phase1-main">
				<rect x="56" y="68" width="248" height="34" rx="8" class="bubble" />
				<text x="68" y="82" class="who">Organizer</text>
				<text x="68" y="96" class="msg">Hola equipo — meeting at 3</text>
			</g>
			<g class="phase phase2-main">
				<rect x="56" y="110" width="248" height="34" rx="8" class="bubble" />
				<text x="68" y="124" class="who">Ana · Spanish</text>
				<text x="68" y="138" class="msg">¿A qué hora exactamente?</text>
			</g>
		</g>

		<g class="node side side-es">
			<rect x="40" y="200" width="280" height="168" rx="10" class="card card-es" />
			<text x="180" y="224" text-anchor="middle" class="label">Spanish · Stacked</text>
			<text x="180" y="242" text-anchor="middle" class="tag tag-es-p1">relay</text>
			<text x="180" y="242" text-anchor="middle" class="tag tag-es-p2">reply</text>

			<g class="phase phase1-es">
				<rect x="56" y="256" width="248" height="40" rx="8" class="bubble" />
				<text x="68" y="272" class="who">Organizer</text>
				<text x="68" y="286" class="msg">Hola equipo — meeting at 3</text>
			</g>
			<g class="phase phase2-es">
				<rect x="56" y="306" width="248" height="46" rx="8" class="bubble" />
				<text x="68" y="322" class="who">Ana</text>
				<text x="68" y="338" class="msg">¿A qué hora exactamente?</text>
			</g>
		</g>

		<g class="node side side-en">
			<rect x="40" y="400" width="280" height="168" rx="10" class="card card-en" />
			<text x="180" y="424" text-anchor="middle" class="label">English · Stacked</text>
			<text x="180" y="442" text-anchor="middle" class="tag tag-both">translate</text>

			<g class="phase phase1-en">
				<rect x="56" y="456" width="248" height="40" rx="8" class="bubble" />
				<text x="68" y="472" class="who">Organizer</text>
				<text x="68" y="486" class="msg">Hi team — meeting at 3</text>
			</g>
			<g class="phase phase2-en">
				<rect x="56" y="506" width="248" height="46" rx="8" class="bubble" />
				<text x="68" y="522" class="who">Ana</text>
				<text x="68" y="538" class="msg">What time exactly?</text>
			</g>
		</g>

		<g class="node side side-fr">
			<rect x="40" y="600" width="280" height="168" rx="10" class="card card-fr" />
			<text x="180" y="624" text-anchor="middle" class="label">French · Stacked</text>
			<text x="180" y="642" text-anchor="middle" class="tag tag-both">translate</text>

			<g class="phase phase1-fr">
				<rect x="56" y="656" width="248" height="40" rx="8" class="bubble" />
				<text x="68" y="672" class="who">Organizer</text>
				<text x="68" y="686" class="msg">Salut l'equipe — reunion 15h</text>
			</g>
			<g class="phase phase2-fr">
				<rect x="56" y="706" width="248" height="46" rx="8" class="bubble" />
				<text x="68" y="722" class="who">Ana</text>
				<text x="68" y="738" class="msg">A quelle heure exactement?</text>
			</g>
		</g>

		<g class="legend" transform="translate(24, 788)">
			<circle cx="6" cy="0" r="4" class="leg-dot relay" />
			<text x="16" y="4" class="leg-text">same language → relay</text>
			<circle cx="6" cy="18" r="4" class="leg-dot translate" />
			<text x="16" y="22" class="leg-text">other language → translate</text>
		</g>
	</svg>

	<figcaption class="footer diagram-footer">
		<div class="caption">
			<ul class="rules">
				<li><strong>Main → sidecars:</strong> relay matching language, translate the rest</li>
				<li><strong>Sidecar → main:</strong> relay only (main stays multilingual)</li>
				<li><strong>Sidecar → other sidecars:</strong> translate</li>
			</ul>
		</div>
		<ProductCommandList id="lt-commands" menu={threadsMenu} />
	</figcaption>
</figure>

<style>
	.canvas {
		display: block;
		width: 100%;
		height: auto;
	}

	.layout-narrow {
		display: none;
	}

	/* Sync with --bp-lg (980px): stacked geometry below desktop nav */
	@media (max-width: 979px) {
		.layout-wide {
			display: none;
		}

		.layout-narrow {
			display: block;
			max-width: 24rem;
			margin-inline: auto;
		}
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

	.wire-es-en,
	.wire-es-fr {
		stroke-dasharray: 4 4;
	}

	.pulse {
		fill: var(--accent);
		opacity: 0;
	}

	.bubble {
		fill: color-mix(in srgb, var(--accent) 12%, var(--surface));
		stroke: var(--border);
		stroke-width: 1;
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

	.phase {
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

	/* 16s: 0–45% main→threads, 45–100% Spanish reply */

	@keyframes phase1-show {
		0%,
		4% {
			opacity: 0;
		}
		8%,
		90% {
			opacity: 1;
		}
		96%,
		100% {
			opacity: 0;
		}
	}

	@keyframes phase1-late {
		0%,
		12% {
			opacity: 0;
		}
		18%,
		90% {
			opacity: 1;
		}
		96%,
		100% {
			opacity: 0;
		}
	}

	@keyframes phase2-show {
		0%,
		48% {
			opacity: 0;
		}
		54%,
		90% {
			opacity: 1;
		}
		96%,
		100% {
			opacity: 0;
		}
	}

	@keyframes phase2-late {
		0%,
		58% {
			opacity: 0;
		}
		64%,
		90% {
			opacity: 1;
		}
		96%,
		100% {
			opacity: 0;
		}
	}

	@keyframes tag-p1 {
		0%,
		10% {
			opacity: 0;
		}
		16%,
		48% {
			opacity: 1;
		}
		54%,
		100% {
			opacity: 0;
		}
	}

	@keyframes tag-p2 {
		0%,
		56% {
			opacity: 0;
		}
		62%,
		90% {
			opacity: 1;
		}
		96%,
		100% {
			opacity: 0;
		}
	}

	@keyframes tag-both-phases {
		0%,
		10% {
			opacity: 0;
		}
		16%,
		90% {
			opacity: 1;
		}
		96%,
		100% {
			opacity: 0;
		}
	}

	@keyframes wire-hub-es-both {
		0%,
		8% {
			stroke: var(--border);
			stroke-width: 1.5;
		}
		14%,
		32% {
			stroke: var(--accent);
			stroke-width: 2;
		}
		40%,
		52% {
			stroke: var(--border);
			stroke-width: 1.5;
		}
		58%,
		78% {
			stroke: var(--accent);
			stroke-width: 2;
		}
		88%,
		100% {
			stroke: var(--border);
			stroke-width: 1.5;
		}
	}

	@keyframes wire-hub-down {
		0%,
		8% {
			stroke: var(--border);
			stroke-width: 1.5;
		}
		14%,
		32% {
			stroke: var(--accent);
			stroke-width: 2;
		}
		40%,
		100% {
			stroke: var(--border);
			stroke-width: 1.5;
		}
	}

	@keyframes wire-cross {
		0%,
		56% {
			stroke: var(--border);
			stroke-width: 1.5;
		}
		62%,
		80% {
			stroke: var(--accent-2);
			stroke-width: 2;
		}
		90%,
		100% {
			stroke: var(--border);
			stroke-width: 1.5;
		}
	}

	@keyframes pulse-down {
		0%,
		10% {
			opacity: 0;
			offset-distance: 0%;
		}
		12% {
			opacity: 1;
		}
		22% {
			opacity: 1;
			offset-distance: 100%;
		}
		24%,
		100% {
			opacity: 0;
			offset-distance: 100%;
		}
	}

	@keyframes pulse-up {
		0%,
		52% {
			opacity: 0;
			offset-distance: 0%;
		}
		54% {
			opacity: 1;
		}
		64% {
			opacity: 1;
			offset-distance: 100%;
		}
		66%,
		100% {
			opacity: 0;
			offset-distance: 100%;
		}
	}

	@keyframes pulse-across {
		0%,
		58% {
			opacity: 0;
			offset-distance: 0%;
		}
		60% {
			opacity: 1;
		}
		70% {
			opacity: 1;
			offset-distance: 100%;
		}
		72%,
		100% {
			opacity: 0;
			offset-distance: 100%;
		}
	}

	@keyframes flash-es-both {
		0%,
		14% {
			stroke: var(--border);
		}
		18%,
		28% {
			stroke: var(--accent);
		}
		36%,
		60% {
			stroke: var(--border);
		}
		64%,
		74% {
			stroke: var(--accent);
		}
		82%,
		100% {
			stroke: var(--border);
		}
	}

	@keyframes flash-en-both {
		0%,
		15% {
			stroke: var(--border);
		}
		19%,
		29% {
			stroke: var(--accent);
		}
		37%,
		62% {
			stroke: var(--border);
		}
		66%,
		76% {
			stroke: var(--accent);
		}
		84%,
		100% {
			stroke: var(--border);
		}
	}

	@keyframes flash-fr-both {
		0%,
		16% {
			stroke: var(--border);
		}
		20%,
		30% {
			stroke: var(--accent);
		}
		38%,
		64% {
			stroke: var(--border);
		}
		68%,
		78% {
			stroke: var(--accent);
		}
		86%,
		100% {
			stroke: var(--border);
		}
	}

	@keyframes flash-main-p2 {
		0%,
		58% {
			stroke: var(--border);
		}
		62%,
		72% {
			stroke: var(--accent);
		}
		80%,
		100% {
			stroke: var(--border);
		}
	}

	.phase1-main {
		animation: phase1-show 16s ease-in-out infinite;
	}

	.phase1-es,
	.phase1-en,
	.phase1-fr {
		animation: phase1-late 16s ease-in-out infinite;
	}

	.phase2-es {
		animation: phase2-show 16s ease-in-out infinite;
	}

	.phase2-main,
	.phase2-en,
	.phase2-fr {
		animation: phase2-late 16s ease-in-out infinite;
	}

	.tag-es-p1 {
		animation: tag-p1 16s ease-in-out infinite;
	}

	.tag-es-p2,
	.tag-main-p2 {
		animation: tag-p2 16s ease-in-out infinite;
	}

	.tag-both {
		animation: tag-both-phases 16s ease-in-out infinite;
	}

	.wire-hub-es {
		animation: wire-hub-es-both 16s ease-in-out infinite;
	}

	.wire-hub-en,
	.wire-hub-fr {
		animation: wire-hub-down 16s ease-in-out infinite;
	}

	.wire-es-en,
	.wire-es-fr {
		animation: wire-cross 16s ease-in-out infinite;
	}

	.card-es {
		animation: flash-es-both 16s ease-in-out infinite;
	}

	.card-en {
		animation: flash-en-both 16s ease-in-out infinite;
	}

	.card-fr {
		animation: flash-fr-both 16s ease-in-out infinite;
	}

	.card-main {
		animation: flash-main-p2 16s ease-in-out infinite;
	}

	/* Wide geometry offset-paths */
	.pulse-wide.pulse-down-es {
		offset-path: path('M390 166 L120 250');
		animation: pulse-down 16s linear infinite;
	}

	.pulse-wide.pulse-down-en {
		offset-path: path('M390 166 L390 250');
		fill: var(--accent-2);
		animation: pulse-down 16s linear infinite;
		animation-delay: 0.2s;
	}

	.pulse-wide.pulse-down-fr {
		offset-path: path('M390 166 L660 250');
		fill: var(--accent-2);
		animation: pulse-down 16s linear infinite;
		animation-delay: 0.4s;
	}

	.pulse-wide.pulse-up-main {
		offset-path: path('M120 250 L390 166');
		animation: pulse-up 16s linear infinite;
	}

	.pulse-wide.pulse-across-en {
		offset-path: path('M210 340 L300 340');
		fill: var(--accent-2);
		animation: pulse-across 16s linear infinite;
	}

	.pulse-wide.pulse-across-fr {
		offset-path: path('M210 358 L570 358');
		fill: var(--accent-2);
		animation: pulse-across 16s linear infinite;
		animation-delay: 0.15s;
	}

	/* Narrow stacked geometry offset-paths */
	.pulse-narrow.pulse-down-es {
		offset-path: path('M170 154 L170 200');
		animation: pulse-down 16s linear infinite;
	}

	.pulse-narrow.pulse-down-en {
		offset-path: path('M300 154 L300 400');
		fill: var(--accent-2);
		animation: pulse-down 16s linear infinite;
		animation-delay: 0.2s;
	}

	.pulse-narrow.pulse-down-fr {
		offset-path: path('M60 154 L60 600');
		fill: var(--accent-2);
		animation: pulse-down 16s linear infinite;
		animation-delay: 0.4s;
	}

	.pulse-narrow.pulse-up-main {
		offset-path: path('M170 200 L170 154');
		animation: pulse-up 16s linear infinite;
	}

	.pulse-narrow.pulse-across-en {
		offset-path: path('M180 368 L180 400');
		fill: var(--accent-2);
		animation: pulse-across 16s linear infinite;
	}

	.pulse-narrow.pulse-across-fr {
		offset-path: path('M60 368 L60 600 L180 600');
		fill: var(--accent-2);
		animation: pulse-across 16s linear infinite;
		animation-delay: 0.15s;
	}

	@media (prefers-reduced-motion: reduce) {
		.phase,
		.tag,
		.wire,
		.pulse,
		.card {
			animation: none !important;
		}

		.phase1-main,
		.phase1-es,
		.phase1-en,
		.phase1-fr,
		.tag-es-p1,
		.tag-both {
			opacity: 1;
		}

		.phase2-main,
		.phase2-es,
		.phase2-en,
		.phase2-fr,
		.tag-es-p2,
		.tag-main-p2 {
			opacity: 0;
		}

		.pulse {
			opacity: 0;
		}
	}
</style>
