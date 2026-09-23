#!/usr/bin/env node
/**
 * Seed Stripe **test** catalog: one Product, two monthly Prices for all-access packs.
 *
 * Usage:
 *   STRIPE_SECRET_KEY=sk_test_... node scripts/stripe-seed-catalog.mjs
 *   STRIPE_SECRET_KEY=sk_test_... node scripts/stripe-seed-catalog.mjs --write-docs
 *
 * Idempotent via Price `lookup_key` and Product metadata `sigstack_catalog=all-access`.
 * Never commit a secret key. Price IDs are not secrets.
 */

import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, '..');
const CATALOG_DOC = join(ROOT, 'docs/commerce/stripe-catalog.md');
const API = 'https://api.stripe.com/v1';

const PRODUCT_META_KEY = 'sigstack_catalog';
const PRODUCT_META_VALUE = 'all-access';

/** @type {const} */
const PACKS = [
	{
		planSku: 'all-access-3',
		lookupKey: 'all-access-3-monthly',
		nickname: 'All-access · 3 groups',
		unitAmount: 1000,
		maxGroups: '3'
	},
	{
		planSku: 'all-access-10',
		lookupKey: 'all-access-10-monthly',
		nickname: 'All-access · 10 groups',
		unitAmount: 2500,
		maxGroups: '10'
	}
];

const key = process.env.STRIPE_SECRET_KEY?.trim();
if (!key) {
	console.error('STRIPE_SECRET_KEY is required (use a sk_test_… key).');
	process.exit(1);
}
if (!key.startsWith('sk_test_')) {
	console.error('Refusing to run: STRIPE_SECRET_KEY must start with sk_test_ (test mode only).');
	process.exit(1);
}

const writeDocs = process.argv.includes('--write-docs');

/**
 * @param {string} method
 * @param {string} path
 * @param {Record<string, string> | null} form
 */
async function stripe(method, path, form = null) {
	const url = path.startsWith('http') ? path : `${API}${path}`;
	/** @type {RequestInit} */
	const init = {
		method,
		headers: {
			Authorization: `Bearer ${key}`
		}
	};
	if (form) {
		init.headers = {
			...init.headers,
			'Content-Type': 'application/x-www-form-urlencoded'
		};
		init.body = new URLSearchParams(form).toString();
	}
	const res = await fetch(url, init);
	const body = await res.json();
	if (!res.ok) {
		const msg = body?.error?.message || JSON.stringify(body);
		throw new Error(`Stripe ${method} ${path}: ${msg}`);
	}
	return body;
}

/** Flatten nested objects for Stripe form encoding (metadata[k]=v). */
function encodeForm(obj, prefix = '') {
	/** @type {Record<string, string>} */
	const out = {};
	for (const [k, v] of Object.entries(obj)) {
		const key = prefix ? `${prefix}[${k}]` : k;
		if (v !== null && typeof v === 'object' && !Array.isArray(v)) {
			Object.assign(out, encodeForm(v, key));
		} else if (v !== undefined && v !== null) {
			out[key] = String(v);
		}
	}
	return out;
}

async function findOrCreateProduct() {
	try {
		const search = await stripe(
			'GET',
			`/products/search?query=${encodeURIComponent(`metadata['${PRODUCT_META_KEY}']:'${PRODUCT_META_VALUE}'`)}&limit=1`
		);
		if (search.data?.length) {
			console.log(`Reusing product ${search.data[0].id}`);
			return search.data[0];
		}
	} catch (err) {
		console.warn(`Product search unavailable (${err.message}); falling back to list.`);
		let startingAfter = null;
		for (let page = 0; page < 10; page++) {
			const qs = new URLSearchParams({ limit: '100', active: 'true' });
			if (startingAfter) qs.set('starting_after', startingAfter);
			const listed = await stripe('GET', `/products?${qs}`);
			const hit = listed.data?.find(
				(p) => p.metadata?.[PRODUCT_META_KEY] === PRODUCT_META_VALUE
			);
			if (hit) {
				console.log(`Reusing product ${hit.id}`);
				return hit;
			}
			if (!listed.has_more || !listed.data?.length) break;
			startingAfter = listed.data[listed.data.length - 1].id;
		}
	}

	const product = await stripe(
		'POST',
		'/products',
		encodeForm({
			name: 'Sigstack All-Access',
			description:
				'All-access subscription: Language Threads, In-chat Translation, and Transcription for every member in enabled Signal groups.',
			metadata: {
				[PRODUCT_META_KEY]: PRODUCT_META_VALUE,
				scope: 'group'
			}
		})
	);
	console.log(`Created product ${product.id}`);
	return product;
}

/**
 * @param {string} productId
 * @param {(typeof PACKS)[number]} pack
 */
async function findOrCreatePrice(productId, pack) {
	const listed = await stripe(
		'GET',
		`/prices?lookup_keys[]=${encodeURIComponent(pack.lookupKey)}&limit=1&active=true`
	);
	if (listed.data?.length) {
		const existing = listed.data[0];
		console.log(`Reusing price ${existing.id} (${pack.lookupKey})`);
		return existing;
	}

	const price = await stripe(
		'POST',
		'/prices',
		encodeForm({
			product: productId,
			currency: 'usd',
			unit_amount: String(pack.unitAmount),
			nickname: pack.nickname,
			lookup_key: pack.lookupKey,
			recurring: { interval: 'month' },
			metadata: {
				plan_sku: pack.planSku,
				max_groups: pack.maxGroups,
				scope: 'group'
			}
		})
	);
	console.log(`Created price ${price.id} (${pack.lookupKey})`);
	return price;
}

function renderCatalogBody(productId, rows) {
	const table = [
		'| plan_sku | lookup_key | amount | max_groups | Price ID (test) |',
		'|----------|------------|--------|------------|-----------------|',
		...rows.map(
			(r) =>
				`| \`${r.planSku}\` | \`${r.lookupKey}\` | $${(r.unitAmount / 100).toFixed(0)}/mo | ${r.maxGroups} | \`${r.priceId}\` |`
		)
	].join('\n');

	return `## Test catalog (seeded)

Product: **Sigstack All-Access** (\`${productId}\`)

${table}

Env placeholders for checkout (IDs are public; secret key stays out of git):

\`\`\`bash
STRIPE_PRICE_ALL_ACCESS_3=${rows[0].priceId}
STRIPE_PRICE_ALL_ACCESS_10=${rows[1].priceId}
\`\`\`
`;
}

function writeCatalogDoc(productId, rows) {
	const start = '<!-- stripe-catalog:generated:start -->';
	const end = '<!-- stripe-catalog:generated:end -->';
	const generated = `${start}\n${renderCatalogBody(productId, rows)}\n${end}`;
	let doc = readFileSync(CATALOG_DOC, 'utf8');
	if (!doc.includes(start) || !doc.includes(end)) {
		throw new Error(`${CATALOG_DOC} missing ${start} / ${end} markers`);
	}
	doc = doc.replace(new RegExp(`${start}[\\s\\S]*?${end}`), generated);
	writeFileSync(CATALOG_DOC, doc);
	console.log(`Updated ${CATALOG_DOC}`);
}

const product = await findOrCreateProduct();
/** @type {{ planSku: string, lookupKey: string, unitAmount: number, maxGroups: string, priceId: string }[]} */
const rows = [];
for (const pack of PACKS) {
	const price = await findOrCreatePrice(product.id, pack);
	rows.push({
		planSku: pack.planSku,
		lookupKey: pack.lookupKey,
		unitAmount: pack.unitAmount,
		maxGroups: pack.maxGroups,
		priceId: price.id
	});
}

console.log('\n' + renderCatalogBody(product.id, rows));

if (writeDocs) {
	writeCatalogDoc(product.id, rows);
}
