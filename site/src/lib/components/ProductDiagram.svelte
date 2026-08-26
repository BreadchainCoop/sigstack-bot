<script lang="ts">
	import languageThreadsUrl from '$lib/assets/diagrams/language-threads.svg?url';
	import inChatUrl from '$lib/assets/diagrams/in-chat.svg?url';
	import transcriptionUrl from '$lib/assets/diagrams/transcription.svg?url';

	const urls = {
		'language-threads': languageThreadsUrl,
		'in-chat': inChatUrl,
		transcription: transcriptionUrl
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
	<img src={urls[kind]} alt="" width="720" height="420" />
	<figcaption>{caption}</figcaption>
</figure>
