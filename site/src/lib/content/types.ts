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

export type PlanOffer = {
	id: string;
	name: string;
	/** Short badge under the name (e.g. "Up to 3 groups"). */
	badge: string;
	blurb: string;
	priceLabel: string;
	period: string;
	ctaHref: string;
	ctaLabel: string;
	/** Visual emphasis on the larger pack. */
	featured?: boolean;
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
			paid: {
				eyebrow: string;
				title: string;
				lead: string;
				note: string;
				offers: PlanOffer[];
			};
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
			codeLabel: string;
			codePlaceholder: string;
			submitLabel: string;
			errorEmpty: string;
			errorTooLong: string;
			linkHeading: string;
			linkBody: string;
			copyLabel: string;
			copyDoneLabel: string;
			messageCta: string;
			signalLinkMissing: string;
			enableNext: string;
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
