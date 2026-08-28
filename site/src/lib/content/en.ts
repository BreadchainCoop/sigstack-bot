import type { SiteContent } from './types';

const stubLead = 'Placeholder — redesign pending.';

export const en: SiteContent = {
	meta: {
		siteName: 'Bread Bot',
		tagline: 'Translation and transcription inside Signal',
		description:
			'TEE-hosted Signal bot for Language Threads, in-chat translation, and voice transcription — not a general chat assistant.'
	},
	pages: {
		home: { title: 'Bread Bot', lead: stubLead },
		products: {
			title: 'Products',
			lead: 'Pick the shape that fits your Signal group. Language Threads keep a multilingual hub with per-language lanes; In-chat Translation stays in one bilingual chat with quote-replies; Transcription turns voice notes into text. Threads and In-chat auto cannot run at the same time—choose one translation mode—while voice can compose with either.',
			sections: [
				{
					id: 'language-threads',
					title: 'Language Threads',
					lead: 'Keep one multilingual Signal group as the hub for your campaign or mutual-aid circle, and give each language its own lane. Organizers and bilingual members post in main; people who prefer Spanish, English, or another language join a Language Thread and read and reply in that language—without anyone dual-posting by hand.'
				},
				{
					id: 'in-chat',
					title: 'In-chat Translation',
					lead: 'For a bilingual campaign or mutual-aid group that wants to stay in one Signal chat, the bot quote-replies each message in the other language. Monolingual members keep up in-thread—no sidecar groups, no dual-posting by organizers.'
				},
				{
					id: 'transcription',
					title: 'Transcription',
					lead: 'When someone sends a voice note, the bot can post a text transcript so people who cannot listen still follow the action. Opt in for auto, or quote a note to transcribe once; transcripts work in the same groups as your translation products.'
				}
			]
		},
		howItWorks: { title: 'How it works', lead: stubLead },
		privacy: { title: 'Privacy and trust', lead: stubLead },
		getStarted: { title: 'Getting started', lead: stubLead },
		plans: { title: 'Plans', lead: stubLead }
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
