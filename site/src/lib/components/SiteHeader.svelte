<script lang="ts">
	import type { Pathname } from '$app/types';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { locales, getLocale, setLocale } from '$lib/paraglide/runtime';
	import * as m from '$lib/paraglide/messages';
	import { List, X } from 'phosphor-svelte';
	import { appPathname } from '$lib/appPathname';

	let open = $state(false);

	const links = $derived([
		{ href: '/suite', label: m.nav_suite() },
		{ href: '/how-it-works', label: m.nav_how() },
		{ href: '/language-threads', label: m.nav_threads() },
		{ href: '/in-chat', label: m.nav_in_chat() },
		{ href: '/transcription', label: m.nav_transcription() },
		{ href: '/privacy', label: m.nav_privacy() },
		{ href: '/faq', label: m.nav_faq() },
		{ href: '/get-started', label: m.nav_start() },
		{ href: '/plans', label: m.nav_plans() }
	]);

	const pathWithoutLocale = $derived(appPathname(page.url));

	function close() {
		open = false;
	}

	function switchLocale(locale: (typeof locales)[number]) {
		setLocale(locale);
		close();
	}
</script>

<a class="skip-link" href="#main">Skip to content</a>

<header class="header">
	<div class="shell bar">
		<a class="brand" href={resolve('/' as Pathname)} onclick={close}>Bread Bot</a>

		<button
			type="button"
			class="menu-toggle"
			aria-expanded={open}
			aria-controls="site-nav"
			onclick={() => (open = !open)}
		>
			{#if open}
				<X size={24} aria-hidden="true" />
				<span class="sr-only">Close menu</span>
			{:else}
				<List size={24} aria-hidden="true" />
				<span class="sr-only">Open menu</span>
			{/if}
		</button>

		<nav id="site-nav" class="nav" class:open aria-label="Primary">
			<ul>
				{#each links as link (link.href)}
					<li>
						<a
							href={resolve(link.href as Pathname)}
							aria-current={pathWithoutLocale.replace(/\/$/, '') === link.href
								? 'page'
								: undefined}
							onclick={close}>{link.label}</a
						>
					</li>
				{/each}
			</ul>

			<div class="lang" aria-label={m.lang_label()}>
				{#each locales as locale (locale)}
					<button
						type="button"
						class="lang-btn"
						aria-current={getLocale() === locale ? 'true' : undefined}
						onclick={() => switchLocale(locale)}>{locale.toUpperCase()}</button
					>
				{/each}
			</div>
		</nav>
	</div>
</header>

<style>
	.header {
		position: sticky;
		top: 0;
		z-index: 40;
		background: color-mix(in srgb, var(--color-paper-0) 92%, transparent);
		backdrop-filter: blur(8px);
		border-bottom: 1px solid var(--color-paper-1);
		min-height: var(--header-height);
	}

	.bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-4);
		min-height: var(--header-height);
	}

	.brand {
		font-family: var(--font-display);
		font-weight: 800;
		font-size: 1.25rem;
		color: var(--color-ink);
		text-decoration: none;
		letter-spacing: -0.03em;
	}

	.brand:hover {
		color: var(--color-primary-orange);
	}

	.menu-toggle {
		display: inline-flex;
		background: transparent;
		border: 1px solid var(--color-paper-2);
		padding: 0.35rem;
		color: var(--color-ink);
		cursor: pointer;
	}

	.nav {
		display: none;
		position: absolute;
		inset-inline: 0;
		top: var(--header-height);
		background: var(--color-paper-0);
		border-bottom: 1px solid var(--color-paper-1);
		padding: var(--space-4) 1rem var(--space-5);
	}

	.nav.open {
		display: block;
	}

	.nav ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		gap: var(--space-2);
	}

	.nav a {
		color: var(--color-ink);
		text-decoration: none;
		font-weight: 600;
		font-size: 0.95rem;
	}

	.nav a:hover,
	.nav a[aria-current='page'] {
		color: var(--color-primary-orange);
	}

	.lang {
		display: flex;
		gap: var(--space-3);
		margin-top: var(--space-4);
		padding-top: var(--space-3);
		border-top: 1px solid var(--color-paper-1);
	}

	.lang-btn {
		font-size: 0.85rem;
		font-weight: 700;
		color: var(--color-grey-2);
		background: transparent;
		border: 0;
		padding: 0;
		cursor: pointer;
		font-family: inherit;
	}

	.lang-btn[aria-current='true'] {
		color: var(--color-primary-jade);
	}

	.sr-only {
		position: absolute;
		width: 1px;
		height: 1px;
		padding: 0;
		margin: -1px;
		overflow: hidden;
		clip: rect(0, 0, 0, 0);
		border: 0;
	}

	@media (min-width: 980px) {
		.menu-toggle {
			display: none;
		}

		.nav {
			display: flex;
			position: static;
			align-items: center;
			gap: var(--space-5);
			background: transparent;
			border: 0;
			padding: 0;
		}

		.nav ul {
			display: flex;
			flex-wrap: wrap;
			gap: var(--space-3);
			justify-content: flex-end;
		}

		.lang {
			margin: 0;
			padding: 0;
			border: 0;
		}
	}
</style>
