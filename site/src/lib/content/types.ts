export type SiteContent = {
	meta: {
		siteName: string;
		tagline: string;
		description: string;
	};
	pages: {
		home: { title: string; lead: string };
		products: {
			title: string;
			lead: string;
			sections: { id: string; title: string; lead: string }[];
		};
		howItWorks: { title: string; lead: string };
		privacy: {
			title: string;
			lead: string;
			sections: { id: string; title: string; lead: string }[];
		};
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
	legalLicense: {
		title: string;
		updated: string;
		copyright: string;
		overview: string;
		fullText: string;
	};
};
