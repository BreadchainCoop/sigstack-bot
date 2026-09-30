// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
declare global {
	namespace App {
		// interface Error {}
		// interface Locals {}
		// interface PageData {}
		// interface PageState {}
		// interface Platform {}
	}
}

interface ImportMetaEnv {
	readonly PUBLIC_STRIPE_PORTAL_URL?: string;
	readonly PUBLIC_SIGNAL_USERNAME_TOKEN?: string;
	/** Public base URL for signal-commerce (no trailing slash), e.g. http://localhost:8082 */
	readonly PUBLIC_COMMERCE_API_BASE_URL?: string;
}

export {};
