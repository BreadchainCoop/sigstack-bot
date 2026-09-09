export type CommandRow = { command: string; where: string; effect: string };

export type Step = { title: string; body: string };

export type PathCard = {
	id: string;
	title: string;
	blurb: string;
	href: string;
	teaserHeading: string;
	teaserLead: string;
	primary?: boolean;
};

export type PlanScope = 'individual' | 'group';

export type PlanOffer = {
	id: string;
	name: string;
	scope: PlanScope;
	blurb: string;
	priceLabel: string;
	period: string;
	ctaHref: string;
	ctaLabel: string;
};

export type PlanProduct = {
	id: string;
	title: string;
	lead: string;
	offers: PlanOffer[];
};

export type SiteContent = {
	meta: {
		siteName: string;
		tagline: string;
		description: string;
	};
	pages: {
		home: {
			title: string;
			lead: string;
			notChat: string;
			pathsHeading: string;
			paths: PathCard[];
		};
		products: {
			title: string;
			lead: string;
			sections: { id: string; title: string; lead: string }[];
		};
		privacy: {
			title: string;
			lead: string;
			sections: { id: string; title: string; lead: string }[];
		};
		getStarted: {
			title: string;
			lead: string;
			eyebrow: string;
			steps: Step[];
			hubCommandsHeading: string;
			hubCommands: CommandRow[];
		};
		plans: {
			title: string;
			lead: string;
			alphaBand: {
				eyebrow: string;
				title: string;
				lead: string;
				ctaLabel: string;
			};
			bundle: {
				eyebrow: string;
				title: string;
				lead: string;
				note: string;
				offers: PlanOffer[];
			};
			aLaCarteHeading: string;
			aLaCarteLead: string;
			products: PlanProduct[];
			scopeLabels: Record<PlanScope, string>;
			footnote: string;
		};
		checkoutSuccess: {
			title: string;
			lead: string;
			eyebrow: string;
			planPurchased: string;
			planPurchasedGeneric: string;
			linkHeading: string;
			linkBody: string;
			linkSteps: string[];
			copyLabel: string;
			copyDoneLabel: string;
			messageCta: string;
			signalLinkMissing: string;
			missingCode: string;
			portalCta: string;
			portalComingSoon: string;
			getStartedCta: string;
		};
		checkoutCancel: {
			title: string;
			lead: string;
			eyebrow: string;
			plansCta: string;
			getStartedCta: string;
		};
		alpha: {
			title: string;
			lead: string;
			eyebrow: string;
			benefitsHeading: string;
			benefits: string[];
			codeLabel: string;
			codePlaceholder: string;
			submitLabel: string;
			errorEmpty: string;
			errorTooLong: string;
			linkHeading: string;
			linkBody: string;
			linkSteps: string[];
			copyLabel: string;
			copyDoneLabel: string;
			messageCta: string;
			signalLinkMissing: string;
			getStartedCta: string;
			plansCta: string;
		};
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
	legalLicense: {
		title: string;
		updated: string;
		copyright: string;
		overview: string;
		fullText: string;
	};
};
