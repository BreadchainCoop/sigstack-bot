export type FaqItem = { q: string; a: string };

export type CommandRow = { command: string; where: string; effect: string };

export type Step = { title: string; body: string };

export type PathCard = {
	id: string;
	title: string;
	blurb: string;
	href: string;
	primary?: boolean;
};

export type SiteContent = {
	meta: {
		siteName: string;
		tagline: string;
		description: string;
	};
	landing: {
		eyebrow: string;
		title: string;
		lead: string;
		notChat: string;
		pathsHeading: string;
		paths: PathCard[];
	};
	suite: {
		title: string;
		lead: string;
		products: { title: string; body: string; href: string }[];
	};
	howItWorks: {
		title: string;
		lead: string;
		oneBot: string;
		sections: { title: string; steps: Step[] }[];
	};
	languageThreads: {
		title: string;
		lead: string;
		when: string;
		flow: string[];
		commands: CommandRow[];
		diagramCaption: string;
	};
	inChat: {
		title: string;
		lead: string;
		when: string;
		flow: string[];
		commands: CommandRow[];
		diagramCaption: string;
	};
	transcription: {
		title: string;
		lead: string;
		when: string;
		flow: string[];
		commands: CommandRow[];
		diagramCaption: string;
	};
	privacy: {
		title: string;
		lead: string;
		points: { title: string; body: string }[];
		honest: string;
	};
	faq: {
		title: string;
		items: FaqItem[];
	};
	getStarted: {
		title: string;
		lead: string;
		steps: Step[];
		hubCommands: CommandRow[];
	};
	plans: {
		title: string;
		lead: string;
		body: string;
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
