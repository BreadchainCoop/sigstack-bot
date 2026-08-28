export type SiteContent = {
	meta: {
		siteName: string;
		tagline: string;
		description: string;
	};
	pages: {
		home: { title: string; lead: string };
		suite: { title: string; lead: string };
		howItWorks: { title: string; lead: string };
		languageThreads: { title: string; lead: string };
		inChat: { title: string; lead: string };
		transcription: { title: string; lead: string };
		privacy: { title: string; lead: string };
		getStarted: { title: string; lead: string };
		plans: { title: string; lead: string };
	};
	legalPrivacy: {
		title: string;
		updated: string;
		sections: { title: string; body: string }[];
	};
	legalTerms: {
		title: string;
		updated: string;
		sections: { title: string; body: string }[];
	};
};
