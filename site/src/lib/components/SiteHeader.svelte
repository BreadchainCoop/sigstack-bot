<script lang="ts">
	import type { Pathname } from '$app/types';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { locales, getLocale, setLocale } from '$lib/paraglide/runtime';
	import * as m from '$lib/paraglide/messages';
	import { List, X, Sun, Moon, CaretDown, Translate } from 'phosphor-svelte';
	import { onMount } from 'svelte';
	import { appPathname } from '$lib/appPathname';
	import { toggleTheme, type Theme } from '$lib/theme';

	let open = $state(false);
	let productsOpen = $state(false);
	let langOpen = $state(false);
	let theme = $state<Theme>('light');

	const productsBase = $derived(resolve('/products' as Pathname));

	const flatLinks = $derived([
		{ href: '/privacy', label: m.nav_privacy() },
		{ href: '/plans', label: m.nav_plans() }
	]);

	const productItems = $derived([
		{ hash: 'language-threads', label: m.nav_threads() },
		{ hash: 'in-chat', label: m.nav_in_chat() },
		{ hash: 'transcription', label: m.nav_transcription() }
	]);

	const pathWithoutLocale = $derived(appPathname(page.url));
	const onProducts = $derived(pathWithoutLocale.replace(/\/$/, '') === '/products');
	const themeLabel = $derived(theme === 'dark' ? m.theme_to_light() : m.theme_to_dark());

	onMount(() => {
		const current = document.documentElement.dataset.theme;
		theme = current === 'dark' ? 'dark' : 'light';
	});

	function close() {
		open = false;
		productsOpen = false;
		langOpen = false;
	}

	function switchLocale(locale: (typeof locales)[number]) {
		setLocale(locale);
		close();
	}

	function onToggleTheme() {
		theme = toggleTheme();
	}
</script>

<svelte:window
	onclick={(e) => {
		const el = e.target as Element | null;
		if (el && !el.closest?.('.products-menu')) productsOpen = false;
		if (el && !el.closest?.('.lang-menu')) langOpen = false;
	}}
	onkeydown={(e) => {
		if (e.key === 'Escape') {
			productsOpen = false;
			langOpen = false;
		}
	}}
/>

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
				<li class="products-menu">
					<div class="products-wrap">
						<div class="products-trigger">
							<a
								class="products-link"
								href={productsBase}
								aria-current={onProducts ? 'page' : undefined}
								onclick={close}>{m.nav_products()}</a
							>
							<button
								type="button"
								class="products-caret"
								aria-expanded={productsOpen}
								aria-controls="products-dropdown"
								aria-label={m.nav_products_menu()}
								onclick={(e) => {
									e.stopPropagation();
									productsOpen = !productsOpen;
								}}
							>
								<CaretDown size={14} aria-hidden="true" />
							</button>
						</div>

						<ul
							id="products-dropdown"
							class="nav-dropdown products-panel"
							class:show={productsOpen}
							role="list"
						>
							{#each productItems as item (item.hash)}
								<li>
									<a href="{productsBase}#{item.hash}" onclick={close}>{item.label}</a>
								</li>
							{/each}
						</ul>

						<ul class="products-mobile">
							{#each productItems as item (item.hash)}
								<li>
									<a href="{productsBase}#{item.hash}" onclick={close}>{item.label}</a>
								</li>
							{/each}
						</ul>
					</div>
				</li>

				{#each flatLinks as link (link.href)}
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
					class="icon-btn"
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

				<div class="lang-menu">
					<div class="lang-wrap">
						<button
							type="button"
							class="icon-btn"
							aria-label={m.lang_label()}
							aria-expanded={langOpen}
							aria-controls="lang-dropdown"
							onclick={(e) => {
								e.stopPropagation();
								langOpen = !langOpen;
								productsOpen = false;
							}}
						>
							<Translate size={20} aria-hidden="true" />
						</button>

						<ul
							id="lang-dropdown"
							class="nav-dropdown lang-panel"
							class:show={langOpen}
							role="list"
						>
							{#each locales as locale (locale)}
								<li>
									<button
										type="button"
										class="lang-option"
										aria-current={getLocale() === locale ? 'true' : undefined}
										onclick={() => switchLocale(locale)}>{locale.toUpperCase()}</button
									>
								</li>
							{/each}
						</ul>
					</div>
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
		background: color-mix(in srgb, var(--surface) 88%, transparent);
		backdrop-filter: blur(10px);
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
		color: var(--accent-text);
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

	.nav > ul {
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

	.nav a:hover {
		color: var(--accent-text);
	}

	.nav a[aria-current='page'] {
		color: var(--fg);
		box-shadow: inset 0 -2px 0 var(--accent);
	}

	.products-menu {
		position: relative;
	}

	.products-wrap {
		position: relative;
	}

	.products-trigger {
		display: flex;
		align-items: center;
		gap: var(--space-1);
	}

	.products-caret {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		background: transparent;
		border: 0;
		padding: 0.2rem;
		color: var(--muted);
		cursor: pointer;
	}

	.products-caret:hover,
	.products-caret[aria-expanded='true'] {
		color: var(--accent-text);
	}

	.products-panel {
		display: none;
		list-style: none;
		margin: var(--space-2) 0 0;
		gap: 0.15rem;
	}

	.products-panel.show {
		display: grid;
	}

	.products-panel a {
		display: block;
		padding: var(--space-2) var(--space-3);
		border-radius: calc(var(--radius) - 2px);
		font-weight: 500;
	}

	.products-panel a:hover {
		background: color-mix(in srgb, var(--accent) 12%, transparent);
	}

	/* Mobile drawer: always list product anchors; hide desktop panel + caret */
	.products-panel {
		display: none !important;
	}

	.products-caret {
		display: none;
	}

	.products-mobile {
		list-style: none;
		margin: var(--space-2) 0 0;
		padding: 0 0 0 var(--space-4);
		display: grid;
		gap: var(--space-2);
	}

	.products-mobile a {
		font-weight: 500;
		font-size: 0.9rem;
		color: var(--muted);
	}

	.controls {
		display: flex;
		align-items: center;
		gap: var(--space-4);
		margin-top: var(--space-4);
		padding-top: var(--space-3);
		border-top: 1px solid var(--border);
	}

	.icon-btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 2.25rem;
		height: 2.25rem;
		background: transparent;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 0;
		color: var(--fg);
		cursor: pointer;
	}

	.icon-btn:hover,
	.icon-btn[aria-expanded='true'] {
		color: var(--accent-text);
		border-color: var(--accent);
	}

	.lang-menu {
		position: relative;
	}

	.lang-wrap {
		position: relative;
	}

	.lang-panel {
		display: none;
		list-style: none;
		margin: var(--space-2) 0 0;
		gap: 0.15rem;
		min-width: 5.5rem;
	}

	.lang-panel.show {
		display: grid;
	}

	.lang-option {
		display: block;
		width: 100%;
		text-align: left;
		padding: var(--space-2) var(--space-3);
		border: 0;
		border-radius: calc(var(--radius) - 2px);
		background: transparent;
		color: var(--fg);
		font-family: inherit;
		font-weight: 600;
		font-size: 0.9rem;
		cursor: pointer;
	}

	.lang-option:hover {
		background: color-mix(in srgb, var(--accent) 12%, transparent);
		color: var(--accent-text);
	}

	.lang-option[aria-current='true'] {
		color: var(--fg);
		box-shadow: inset 3px 0 0 var(--accent);
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

		.nav > ul {
			display: flex;
			flex-wrap: wrap;
			align-items: center;
			gap: var(--space-4);
			justify-content: flex-end;
		}

		.products-caret {
			display: inline-flex;
		}

		/* Hoverable bridge between trigger and panel (avoids dead gap) */
		.products-wrap {
			padding-bottom: 0.5rem;
		}

		.products-panel {
			position: absolute;
			top: 100%;
			left: 0;
			z-index: 50;
			margin: 0;
			display: none !important;
		}

		.products-panel.show,
		.products-wrap:hover .products-panel,
		.products-wrap:focus-within .products-panel {
			display: grid !important;
		}

		.products-mobile {
			display: none;
		}

		.lang-wrap {
			padding-bottom: 0.5rem;
		}

		.lang-panel {
			position: absolute;
			top: 100%;
			right: 0;
			left: auto;
			z-index: 50;
			margin: 0;
			display: none;
		}

		.lang-panel.show,
		.lang-wrap:hover .lang-panel,
		.lang-wrap:focus-within .lang-panel {
			display: grid;
		}

		.controls {
			margin: 0;
			padding: 0;
			border: 0;
		}
	}
</style>
