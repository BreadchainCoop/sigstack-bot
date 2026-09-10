import { defineConfig } from '@playwright/test';
import { E2E_SIGNAL_USERNAME_TOKEN } from './src/lib/signalUsernameLink';

const basePath = process.env.BASE_PATH ?? '/sigstack-bot/sigstack';
const origin = 'http://127.0.0.1:4173';

export default defineConfig({
	webServer: {
		command: 'npm run build && npm run preview -- --host 127.0.0.1 --port 4173',
		url: `${origin}${basePath}/`,
		reuseExistingServer: !process.env.CI,
		timeout: 180_000,
		env: {
			...process.env,
			PUBLIC_SIGNAL_USERNAME_TOKEN:
				process.env.PUBLIC_SIGNAL_USERNAME_TOKEN || E2E_SIGNAL_USERNAME_TOKEN
		}
	},
	use: {
		baseURL: `${origin}${basePath}/`
	},
	testMatch: '**/*.e2e.{ts,js}'
});
