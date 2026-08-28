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
		suite: { title: 'Product suite', lead: stubLead },
		howItWorks: { title: 'How it works', lead: stubLead },
		languageThreads: { title: 'Language Threads', lead: stubLead },
		inChat: { title: 'In-chat translation', lead: stubLead },
		transcription: { title: 'Voice transcription', lead: stubLead },
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
