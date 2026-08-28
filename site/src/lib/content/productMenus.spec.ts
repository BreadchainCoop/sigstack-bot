import { describe, expect, it } from 'vitest';
import { inChatMenu, threadsMenu, transcriptionMenu } from './productMenus';

describe('productMenus', () => {
	it('mirrors key Language Threads commands', () => {
		expect(threadsMenu).toContain('!translate-me-thread <lang>');
		expect(threadsMenu).toContain('!help-threads');
		expect(threadsMenu).toContain('!enable-in-chat');
	});

	it('mirrors key In-chat commands', () => {
		expect(inChatMenu).toContain('!translate-all-on <lang1> <lang2>');
		expect(inChatMenu).toContain('!translate <lang> (as reply)');
		expect(inChatMenu).toContain('!enable-threads');
	});

	it('mirrors key Transcription commands', () => {
		expect(transcriptionMenu).toContain('!transcribe-on');
		expect(transcriptionMenu).toContain('!transcribe');
		expect(transcriptionMenu).toContain('!help-transcription');
	});
});
