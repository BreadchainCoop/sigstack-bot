<script lang="ts">
	import type { Pathname } from '$app/types';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { locales, getLocale, setLocale } from '$lib/paraglide/runtime';
	import * as m from '$lib/paraglide/messages';
	import { List, X, Sun, Moon } from 'phosphor-svelte';
	import { onMount } from 'svelte';
	import { appPathname } from '$lib/appPathname';
	import { toggleTheme, type Theme } from '$lib/theme';

	let open = $state(false);
	let theme = $state<Theme>('light');

	const links = $derived([
		{ href: '/suite', label: m.nav_suite() },
		{ href: '/how-it-works', label: m.nav_how() },
		{ href: '/language-threads', label: m.nav_threads() },
		{ href: '/in-chat', label: m.nav_in_chat() },
		{ href: '/transcription', label: m.nav_transcription() },
		{ href: '/privacy', label: m.nav_privacy() },
		{ href: '/get-started', label: m.nav_start() },
		{ href: '/plans', label: m.nav_plans() }
	]);

	const pathWithoutLocale = $derived(appPathname(page.url));
	const themeLabel = $derived(theme === 'dark' ? m.theme_to_light() : m.theme_to_dark());

	onMount(() => {
		const current = document.documentElement.dataset.theme;
		theme = current === 'dark' ? 'dark' : 'light';
	});

	function close() {
		open = false;
	}

	function switchLocale(locale: (typeof locales)[number]) {
		setLocale(locale);
		close();
	}

	function onToggleTheme() {
		theme = toggleTheme();
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
							aria-current={pathWithoutLocale.replace(/\/$/, '') === link.href ? 'page' : undefined}
							onclick={close}>{link.label}</a
						>
					</li>
				{/each}
			</ul>

			<div class="controls">
				<button
					type="button"
					class="theme-btn"
					aria-label={themeLabel}
					aria-pressed={theme === 'dark'}
					onclick={onToggleTheme}
				>
					{#if theme === 'dark'}
						<Sun size={20} aria-hidden="true" />
					{:else}
						<Moon size={20} aria-hidden="true" />
					{/if}
				</button>

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
			</div>
		</nav>
	</div>
</header>

<style>
	.header {
		position: sticky;
		top: 0;
		z-index: 40;
		background: color-mix(in srgb, var(--surface) 92%, transparent);
		backdrop-filter: blur(8px);
		border-bottom: 1px solid var(--border);
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
		font-family: var(--font-sans);
		font-weight: 700;
		font-size: 1.2rem;
		color: var(--fg);
		text-decoration: none;
		letter-spacing: -0.03em;
	}

	.brand:hover {
		color: var(--accent);
	}

	.menu-toggle {
		display: inline-flex;
		background: transparent;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 0.35rem;
		color: var(--fg);
		cursor: pointer;
	}

	.nav {
		display: none;
		position: absolute;
		inset-inline: 0;
		top: var(--header-height);
		background: var(--surface);
		border-bottom: 1px solid var(--border);
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
		color: var(--fg);
		text-decoration: none;
		font-weight: 600;
		font-size: 0.95rem;
	}

	.nav a:hover,
	.nav a[aria-current='page'] {
		color: var(--accent);
	}

	.controls {
		display: flex;
		align-items: center;
		gap: var(--space-4);
		margin-top: var(--space-4);
		padding-top: var(--space-3);
		border-top: 1px solid var(--border);
	}

	.theme-btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		background: transparent;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 0.3rem;
		color: var(--fg);
		cursor: pointer;
	}

	.theme-btn:hover {
		color: var(--accent);
		border-color: var(--accent);
	}

	.lang {
		display: flex;
		gap: var(--space-3);
	}

	.lang-btn {
		font-size: 0.85rem;
		font-weight: 700;
		color: var(--muted);
		background: transparent;
		border: 0;
		padding: 0;
		cursor: pointer;
		font-family: inherit;
	}

	.lang-btn[aria-current='true'] {
		color: var(--accent);
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

		.controls {
			margin: 0;
			padding: 0;
			border: 0;
		}
	}
</style>
