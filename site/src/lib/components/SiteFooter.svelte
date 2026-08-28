<script lang="ts">
	import type { Pathname } from '$app/types';
	import { resolve } from '$app/paths';
	import * as m from '$lib/paraglide/messages';
</script>

<footer class="footer">
	<div class="shell grid">
		<div>
			<p class="brand">Bread Bot</p>
			<p class="muted">{m.footer_tagline()}</p>
		</div>
		<div>
			<p class="label">{m.footer_legal()}</p>
			<ul>
				<li>
					<a href={resolve('/privacy-policy' as Pathname)}>{m.footer_privacy_policy()}</a>
				</li>
				<li>
					<a href={resolve('/terms' as Pathname)}>{m.footer_terms()}</a>
				</li>
				<li>
					<a href="https://github.com/BreadchainCoop/sigstack-bot" rel="noopener noreferrer"
						>{m.cta_github()}</a
					>
				</li>
			</ul>
		</div>
	</div>
</footer>

<style>
	.footer {
		border-top: 1px solid var(--border);
		background: var(--surface);
		padding: var(--space-7) 0;
	}

	.grid {
		display: grid;
		gap: var(--space-5);
	}

	@media (min-width: 720px) {
		.grid {
			grid-template-columns: 1.4fr 1fr;
		}
	}

	.brand {
		font-family: var(--font-sans);
		font-weight: 700;
		font-size: 1.1rem;
		margin: 0 0 var(--space-2);
		color: var(--fg);
	}

	.label {
		font-weight: 700;
		margin: 0 0 var(--space-2);
		color: var(--fg);
	}

	ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		gap: var(--space-2);
	}

	a {
		color: var(--muted);
		text-decoration: none;
		font-weight: 600;
	}

	a:hover {
		color: var(--accent-text);
	}
</style>
