import { render } from 'svelte/server';
import { describe, expect, it } from 'vitest';
import PathChooser from './PathChooser.svelte';

describe('PathChooser', () => {
	it('renders recommended path and heading', () => {
		const { body } = render(PathChooser, {
			props: {
				heading: 'Which mode?',
				paths: [
					{
						id: 'threads',
						title: 'Language Threads',
						blurb: 'Primary path',
						href: '/language-threads',
						primary: true
					},
					{
						id: 'in-chat',
						title: 'In-chat',
						blurb: 'Secondary path',
						href: '/in-chat'
					}
				]
			}
		});
		expect(body).toContain('Which mode?');
		expect(body).toContain('Recommended');
		expect(body).toContain('Language Threads');
	});
});
