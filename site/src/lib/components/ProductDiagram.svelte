<script lang="ts">
	import { asset } from '$app/paths';

	const urls = {
		'language-threads': asset('/diagrams/language-threads.svg'),
		'in-chat': asset('/diagrams/in-chat.svg'),
		transcription: asset('/diagrams/transcription.svg')
	} as const;

	const dimensions = {
		'language-threads': { width: 720, height: 420 },
		'in-chat': { width: 720, height: 320 },
		transcription: { width: 720, height: 280 }
	} as const;

	let {
		kind,
		caption
	}: {
		kind: keyof typeof urls;
		caption: string;
	} = $props();
</script>

<figure class="diagram-frame">
	<img
		src={urls[kind]}
		alt={caption}
		width={dimensions[kind].width}
		height={dimensions[kind].height}
		style="--diagram-aspect-w: {dimensions[kind].width}; --diagram-aspect-h: {dimensions[kind].height}"
	/>
	<figcaption>{caption}</figcaption>
</figure>
