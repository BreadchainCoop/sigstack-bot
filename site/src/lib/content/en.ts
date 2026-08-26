import type { SiteContent } from './types';

export const en: SiteContent = {
	meta: {
		siteName: 'Bread Bot',
		tagline: 'Translation and transcription inside Signal',
		description:
			'TEE-hosted Signal bot for Language Threads, in-chat translation, and voice transcription — not a general chat assistant.'
	},
	landing: {
		eyebrow: 'Bread Cooperative',
		title: 'Bread Bot',
		lead:
			'One Signal bot for multilingual groups: Language Threads, same-group translation, and voice-to-text — hosted in a hardware TEE.',
		notChat:
			'Bread Bot is not a general AI chat assistant. It translates and transcribes inside Signal so organizers can run multilingual rooms without leaving the app.',
		pathsHeading: 'Which mode do you want?',
		paths: [
			{
				id: 'threads',
				title: 'Language Threads',
				blurb: 'Multilingual main chat plus one sidecar group per language. Best default for multilingual organizing.',
				href: '/language-threads',
				primary: true
			},
			{
				id: 'in-chat',
				title: 'In-chat translation',
				blurb: 'Stay in one bilingual group. The bot quote-replies with the other language in the same thread.',
				href: '/in-chat'
			},
			{
				id: 'transcription',
				title: 'Voice transcription',
				blurb: 'Turn voice notes into text in-group. Pairs with either translation mode.',
				href: '/transcription'
			}
		]
	},
	suite: {
		title: 'Product suite',
		lead: 'Three capabilities on one bot and one phone number. Pick one translation mode per group; voice works alongside either.',
		products: [
			{
				title: 'Language Threads',
				body: 'Primary bridging model: multilingual hub + N monolingual sidecars with clear fan-out rules.',
				href: '/language-threads'
			},
			{
				title: 'In-chat translation',
				body: 'Secondary path when everyone shares one bilingual room and wants quote-reply translations.',
				href: '/in-chat'
			},
			{
				title: 'Voice transcription',
				body: 'Speech → text via NEAR AI Whisper. Opt-in; composes with translation when both are on.',
				href: '/transcription'
			}
		]
	},
	howItWorks: {
		title: 'How it works',
		lead: 'Add one Bread Bot to your Signal group. No second app. No dual phone numbers for organizers.',
		oneBot:
			'Production runs as one Phala CVM with one Signal identity. Signal end-to-end encryption terminates inside that TEE. The bot posts translations and transcripts back into your groups.',
		sections: [
			{
				title: 'Language Threads',
				steps: [
					{ title: 'Add the bot', body: 'Invite Bread Bot to your multilingual main group.' },
					{
						title: 'Open a language sidecar',
						body: 'Type !translate-me-thread es (or another language code) in the main group.'
					},
					{
						title: 'Talk in either room',
						body: 'Main fans out to sidecars; sidecars relay raw into main and translate across other sidecars.'
					}
				]
			},
			{
				title: 'In-chat translation',
				steps: [
					{ title: 'Stay in one group', body: 'Use a single bilingual Signal group.' },
					{
						title: 'Enable auto or quote',
						body: 'Turn on !translate-all-on es en, or quote a message with !translate <lang>.'
					},
					{
						title: 'Read the quote-reply',
						body: 'Original stays; the bot replies with the other language in the same thread.'
					}
				]
			},
			{
				title: 'Voice transcription',
				steps: [
					{ title: 'Opt in', body: 'Use !transcribe-on for auto, or quote a voice note with !transcribe.' },
					{
						title: 'Send a voice note',
						body: 'Audio is decrypted in the TEE, metadata stripped, then sent to NEAR Whisper.'
					},
					{
						title: 'Read the transcript',
						body: 'The bot quote-replies with text. If translation is active, that text can fan out too.'
					}
				]
			}
		]
	},
	languageThreads: {
		title: 'Language Threads',
		lead:
			'A multilingual main hub plus one sidecar Signal group per language for monolingual members (for example Spanish · Stacked).',
		when:
			'Prefer Language Threads when you have many languages, or when monolingual members need their own lane. Add languages anytime without reconfiguring the whole room.',
		flow: [
			'Main → sidecar: same language relays; otherwise translate.',
			'Sidecar → main: relay only so the main stays multilingual.',
			'Sidecar → other sidecar: translate (skip the source language).',
			'Attribution looks like: Name:\\nmessage'
		],
		commands: [
			{
				command: '!translate-me-thread es',
				where: 'Main',
				effect: 'Create or join the Spanish Language Thread and invite you'
			},
			{ command: '!leave', where: 'Sidecar', effect: 'Leave this Language Thread' },
			{ command: '!rename <name>', where: 'Sidecar', effect: 'Rename the sidecar group' },
			{
				command: '!enable-in-chat',
				where: 'Main',
				effect: 'Tear down threads and switch to in-chat translation'
			},
			{ command: '!help-threads', where: 'Any', effect: 'How Language Threads works' },
			{ command: '!list-langs', where: 'Any', effect: 'List language codes' }
		],
		diagramCaption:
			'Main multilingual hub with language sidecars. Arrows show relay vs translate between rooms.'
	},
	inChat: {
		title: 'In-chat translation',
		lead:
			'One bilingual Signal group. Human messages stay put; Bread Bot quote-replies with the other language in the same thread.',
		when:
			'Use in-chat when you want a single shared room with two languages and do not need sidecar groups. Mutually exclusive with Language Threads — switch with !enable-threads or !enable-in-chat.',
		flow: [
			'Member posts text in the group.',
			'Bot detects language against the configured pair.',
			'If it matches, NEAR AI translates and the bot quote-replies with a flag + translation.',
			'Original human message is unchanged.'
		],
		commands: [
			{
				command: '!translate-all-on es en',
				where: 'Group',
				effect: 'Auto-translate everyone’s messages'
			},
			{ command: '!translate-all-off', where: 'Group', effect: 'Turn off group-wide auto' },
			{
				command: '!translate-me-on es en',
				where: 'You',
				effect: 'Auto-translate only your messages (when group-wide is off)'
			},
			{ command: '!translate-me-off', where: 'You', effect: 'Stop personal auto' },
			{
				command: '!translate <lang>',
				where: 'Quote',
				effect: 'Translate one quoted message (always available)'
			},
			{
				command: '!enable-threads',
				where: 'Group',
				effect: 'Clear in-chat auto and switch to Language Threads'
			}
		],
		diagramCaption: 'Single group: human message stays; bot quote-replies with the translation.'
	},
	transcription: {
		title: 'Voice transcription',
		lead: 'Speech becomes text in the same chat via NEAR AI Whisper Large V3 (GPU TEE). Default off — opt in per user or group.',
		when:
			'Pair transcription with Language Threads or in-chat translation. After STT, spoken text can fan out as if the speaker typed it.',
		flow: [
			'Member sends a voice note.',
			'Bot decrypts audio in the TEE and strips Signal metadata.',
			'NEAR Whisper returns text.',
			'Bot quote-replies with a transcript; translation modes can fan it out.'
		],
		commands: [
			{ command: '!transcription', where: 'Any', effect: 'Open the voice menu' },
			{ command: '!transcribe-on / !transcribe-off', where: 'You', effect: 'Auto-transcribe (default off)' },
			{ command: '!transcribe', where: 'Quote', effect: 'Transcribe one quoted voice note' },
			{ command: '!help-transcription', where: 'Any', effect: 'How transcription works' }
		],
		diagramCaption: 'Voice note → TEE strip → NEAR Whisper → transcript quote-reply in group.'
	},
	privacy: {
		title: 'Privacy and trust',
		lead: 'Bread Bot runs in a hardware trusted execution environment (TEE). Here is what that means in plain language.',
		points: [
			{
				title: 'Signal E2E terminates in the TEE',
				body: 'Plaintext of messages the bot processes exists inside the attested CVM, not on a general-purpose host the operator can casually read.'
			},
			{
				title: 'Attestation with !verify',
				body: 'In Signal, !verify returns a remote attestation quote for this CVM’s compose so you can check what code is running.'
			},
			{
				title: 'What leaves the box for voice',
				body: 'Voice audio is decrypted in the TEE, stripped of Signal metadata, and uploaded to NEAR AI Whisper as a generic file for speech-to-text. That is intentional remote STT — not a claim that audio never leaves the CVM.'
			},
			{
				title: 'Preferences stay on disk volumes',
				body: 'Feature prefs live in encrypted volume storage on the CVM so in-place upgrades do not wipe who opted in.'
			}
		],
		honest:
			'We do not claim full end-to-end encryption of bot processing. The bot must read message content to translate or transcribe. Prefer honest TEE + attestation over marketing that pretends otherwise.'
	},
	faq: {
		title: 'FAQ',
		items: [
			{
				q: 'Is Bread Bot a ChatGPT-style assistant?',
				a: 'No. It is a Signal bot for translation and transcription only — not general chat, tools, or agent workflows.'
			},
			{
				q: 'Language Threads or in-chat — which should I pick?',
				a: 'Prefer Language Threads for multilingual groups and monolingual sidecars. Use in-chat when you want one bilingual room with quote-reply translations.'
			},
			{
				q: 'Can I run both translation modes in one group?',
				a: 'No. Language Threads, Bilingual Threads, and in-chat auto are mutually exclusive. Switch with !enable-in-chat or !enable-threads. Voice and manual !translate still work.'
			},
			{
				q: 'Do I need two phone numbers?',
				a: 'No. One bot, one Signal number, one CVM. Add that bot to your group(s).'
			},
			{
				q: 'Where is my data processed?',
				a: 'Bot logic runs in a Phala TEE. Text translation uses NEAR AI chat. Voice uses NEAR AI Whisper after metadata stripping. See the Privacy page for details.'
			},
			{
				q: 'Is processing fully end-to-end encrypted?',
				a: 'Signal E2E protects the path to the bot. Inside the TEE the bot must decrypt to act. Remote Whisper receives stripped audio. Use !verify to attest the CVM.'
			},
			{
				q: 'How do I start?',
				a: 'Add Bread Bot to a Signal group, open !help, then follow Getting started for the commands for your chosen product.'
			}
		]
	},
	getStarted: {
		title: 'Getting started',
		lead: 'Organizer checklist — the commands people actually type in Signal.',
		steps: [
			{
				title: 'Add Bread Bot',
				body: 'Invite the bot to your Signal group. It auto-accepts group invites.'
			},
			{
				title: 'Open the hub',
				body: 'Send !help for product menus: Language Threads, in-chat, and transcription.'
			},
			{
				title: 'Pick a path',
				body: 'Start Language Threads with !translate-me-thread <lang>, or in-chat with !translate-all-on <lang1> <lang2>, and opt into voice with !transcribe-on.'
			}
		],
		hubCommands: [
			{ command: '!help / !info', where: 'Any', effect: 'Bread Bot hub menus' },
			{ command: '!privacy', where: 'Any', effect: 'Privacy, TEE, and !verify in one reply' },
			{ command: '!translation-threads', where: 'Any', effect: 'Language Threads menu' },
			{ command: '!translation-in-chat', where: 'Any', effect: 'In-chat translation menu' },
			{ command: '!transcription', where: 'Any', effect: 'Voice product menu' }
		]
	},
	plans: {
		title: 'Plans',
		lead: 'Pricing and checkout are coming soon.',
		body: 'This educational site ships first (issue #23). Subscriptions and Stripe checkout land in a later release. Until then, use Getting started to learn the products, or open a GitHub issue if you are organizing a pilot.'
	},
	legalPrivacy: {
		title: 'Privacy Policy',
		updated: 'August 2026',
		sections: [
			{
				title: 'Overview',
				body: 'Bread Bot processes Signal group messages to provide translation and transcription. This stub summarizes processing for the public site; operators should align it with live deployment before collecting payment.'
			},
			{
				title: 'Data we process',
				body: 'Message text and voice-note audio the bot is configured to handle; Signal identifiers needed to post replies; encrypted preference state on the CVM volume.'
			},
			{
				title: 'Processors',
				body: 'Phala (TEE host), NEAR AI (chat translation and Whisper STT). Audio sent to Whisper is stripped of Signal metadata first.'
			},
			{
				title: 'Contact',
				body: 'For privacy questions about a Bread Cooperative deployment, contact the operators via the project GitHub repository.'
			}
		]
	},
	legalTerms: {
		title: 'Terms of Service',
		updated: 'August 2026',
		sections: [
			{
				title: 'Service',
				body: 'Bread Bot is provided as a Signal automation for translation and transcription. Availability depends on the operator’s deployment.'
			},
			{
				title: 'Acceptable use',
				body: 'Do not use the bot to harass, break Signal’s terms, or process unlawful content. Organizers are responsible for group membership and consent.'
			},
			{
				title: 'No warranty',
				body: 'Translations and transcripts may contain errors. The service is provided as-is without guarantees of accuracy or uptime.'
			},
			{
				title: 'Changes',
				body: 'These terms are a public stub pending commercial launch. Operators may update them when subscriptions ship.'
			}
		]
	}
};
