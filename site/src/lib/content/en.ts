import type { SiteContent } from './types';

export const en: SiteContent = {
	meta: {
		siteName: 'Sigstack',
		tagline: 'Translation and transcription inside Signal',
		description:
			'TEE-hosted Signal bot for Language Threads, in-chat translation, and voice transcription — not a general chat assistant.'
	},
	pages: {
		home: {
			title: 'Sigstack',
			lead: 'One Signal bot for multilingual groups: Language Threads, same-group translation, and voice-to-text — hosted in a hardware TEE.',
			notChat:
				'Sigstack is not a general AI chat assistant. It translates and transcribes inside Signal so organizers can run multilingual rooms without leaving the app.',
			pathsHeading: 'Which mode do you want?',
			paths: [
				{
					id: 'threads',
					title: 'Language Threads',
					blurb:
						'Multilingual main chat plus one sidecar group per language. Best default for multilingual organizing.',
					href: '/products#language-threads',
					teaserHeading: 'See Language Threads',
					teaserLead:
						'Diagrams first — so organizers can picture fan-out before learning commands.',
					primary: true
				},
				{
					id: 'in-chat',
					title: 'In-chat translation',
					blurb:
						'Stay in one bilingual group. The bot quote-replies with the other language in the same thread.',
					href: '/products#in-chat',
					teaserHeading: 'See In-chat Translation',
					teaserLead:
						'One bilingual chat — the bot quote-replies so everyone keeps up in-thread.'
				},
				{
					id: 'transcription',
					title: 'Voice transcription',
					blurb: 'Turn voice notes into text in-group. Pairs with either translation mode.',
					href: '/products#transcription',
					teaserHeading: 'See Voice Transcription',
					teaserLead:
						'Voice notes become text in-group — follow along without needing to listen.'
				}
			]
		},
		products: {
			title: 'Products',
			lead: 'Pick the shape that fits your Signal group. Language Threads keep a multilingual hub with per-language lanes; In-chat Translation stays in one bilingual chat with quote-replies; Transcription turns voice notes into text. Threads and In-chat auto cannot run at the same time—choose one translation mode—while voice can compose with either.',
			sections: [
				{
					id: 'language-threads',
					title: 'Language Threads',
					lead: 'Keep one multilingual Signal group as the hub for your campaign or mutual-aid circle, and give each language its own lane. Organizers and bilingual members post in main; people who prefer Spanish, English, Basque, Swahili, or another language join a Language Thread and read and reply in that language—without anyone dual-posting by hand. Language Threads supports 32 languages (!list-langs)—more than in-chat auto-translate.'
				},
				{
					id: 'in-chat',
					title: 'In-chat Translation',
					lead: 'For a bilingual campaign or mutual-aid group that wants to stay in one Signal chat, the bot quote-replies each message in the other language. Monolingual members keep up in-thread—no sidecar groups, no dual-posting by organizers. Auto-translate supports 30 detectable languages (!list-langs-in-chat); quote !translate can target any Language Threads language (!list-langs).'
				},
				{
					id: 'transcription',
					title: 'Transcription',
					lead: 'When someone sends a voice note, the bot can post a text transcript so people who cannot listen still follow the action. Opt in for auto, or quote a note to transcribe once; transcripts work in the same groups as your translation products.'
				}
			]
		},
		privacy: {
			title: 'Privacy and trust',
			lead: 'Sigstack runs in a Phala Trusted Execution Environment (TEE)—sealed hardware the host cannot read. Privacy is enforced by the chip, not by a policy alone.',
			sections: [
				{
					id: 'what-is-a-tee',
					title: 'What a TEE is',
					lead: 'A locked region inside the server chip. Code and messages stay encrypted in memory; the cloud host cannot peek. Phala provides that sealed hardware for Sigstack.'
				},
				{
					id: 'how-sigstack',
					title: 'How Sigstack uses it',
					lead: 'One Signal number in one Phala TEE. Text and voice decrypt only inside that box. Translation and transcription go to NEAR AI private inference; voice leaves only as metadata-stripped audio.'
				},
				{
					id: 'promise-vs-proof',
					title: 'Promise vs proof',
					lead: 'Many services ask you to trust a privacy policy. We use hardware isolation you can challenge in Signal with !verify.'
				},
				{
					id: 'limits',
					title: 'Limits',
					lead: 'Group members still see posts. Operators see metadata (timing, sizes, numbers). NEAR AI processes translation text and stripped audio. !verify attests this CVM’s compose, not Whisper weights. Legal detail: Privacy Policy.'
				}
			]
		},
		getStarted: {
			title: 'Getting started',
			lead: 'Organizer checklist — the commands people actually type in Signal.',
			eyebrow: 'Organizers',
			steps: [
				{
					title: 'Add Sigstack',
					body: 'Organizers: invite Sigstack to your Signal group (it auto-accepts). Alpha or paid users: open a DM first via the Message Sigstack button on the alpha or checkout pages, then link with `!link <code>`.'
				},
				{
					title: 'Open the hub',
					body: 'Send !help for product menus: Language Threads, in-chat, and transcription.'
				},
				{
					title: 'Pick a path',
					body: 'Start Language Threads with !translate-me-thread <lang>, or in-chat with !translate-all-on <lang1> <lang2>, and opt into voice with !transcribe-on.'
				}
			],
			hubCommandsHeading: 'Hub commands',
			hubCommands: [
				{ command: '!help / !info', where: 'Any', effect: 'Sigstack hub menus' },
				{ command: '!privacy', where: 'Any', effect: 'Privacy, TEE, and !verify in one reply' },
				{ command: '!translation-threads', where: 'Any', effect: 'Language Threads menu' },
				{ command: '!translation-in-chat', where: 'Any', effect: 'In-chat translation menu' },
				{ command: '!transcription', where: 'Any', effect: 'Voice product menu' }
			]
		},
		plans: {
			title: 'Plans',
			lead: 'Pick a Bundle for all three products, or pay à la carte. Individual plans cover you; Group plans cover a whole Signal chat where the product works that way.',
			alphaBand: {
				eyebrow: 'Alpha',
				title: 'Try Sigstack free with an alpha code',
				lead: 'Full Bundle access for 90 days—no checkout. Redeem your code, open a Signal DM, and send `!link`.',
				ctaLabel: 'Redeem alpha code'
			},
			bundle: {
				eyebrow: 'Package',
				title: 'Sigstack Bundle',
				lead: 'Access to Language Threads, In-chat Translation, and Transcription in one subscription.',
				note: 'Language Threads and In-chat auto cannot run at the same time—choose one translation mode. Voice pairs with either.',
				offers: [
					{
						id: 'bundle-individual',
						name: 'Bundle · Individual',
						scope: 'individual',
						blurb:
							'Language Threads, In-chat me, and Transcription for one person. Self-subscribe in any group where the bot is present.',
						priceLabel: 'TBD',
						period: '',
						ctaHref: '/get-started',
						ctaLabel: 'Get started'
					},
					{
						id: 'bundle-group',
						name: 'Bundle · Group',
						scope: 'group',
						blurb:
							'Everything in Individual, plus In-chat all for one Signal group—bilingual quote-replies for every member.',
						priceLabel: 'TBD',
						period: '',
						ctaHref: '/get-started',
						ctaLabel: 'Get started'
					}
				]
			},
			aLaCarteHeading: 'À la carte',
			aLaCarteLead: 'Subscribe to one product. Scope follows how the feature works in Signal.',
			products: [
				{
					id: 'language-threads',
					title: 'Language Threads',
					lead: 'Self-subscribe to a monolingual sidecar lane, or cover the whole multilingual main with a Group plan.',
					offers: [
						{
							id: 'threads-individual',
							name: 'Language Threads · me',
							scope: 'individual',
							blurb: 'You join or create a Language Thread for yourself (`!translate-me-thread`).',
							priceLabel: 'TBD',
							period: '',
							ctaHref: '/products#language-threads',
							ctaLabel: 'Learn more'
						},
						{
							id: 'threads-group',
							name: 'Language Threads · group',
							scope: 'group',
							blurb:
								'Language Threads for one multilingual main—sidecars for every language lane the group needs.',
							priceLabel: 'TBD',
							period: '',
							ctaHref: '/products#language-threads',
							ctaLabel: 'Learn more'
						}
					]
				},
				{
					id: 'in-chat',
					title: 'In-chat Translation',
					lead: 'Stay in one bilingual chat. Me is per person; All covers the whole group.',
					offers: [
						{
							id: 'in-chat-me',
							name: 'In-chat · me',
							scope: 'individual',
							blurb: 'Auto-translate your messages only (`!translate-me-on`).',
							priceLabel: 'TBD',
							period: '',
							ctaHref: '/products#in-chat',
							ctaLabel: 'Learn more'
						},
						{
							id: 'in-chat-all',
							name: 'In-chat · all',
							scope: 'group',
							blurb: 'Group-wide bilingual quote-replies (`!translate-all-on`).',
							priceLabel: 'TBD',
							period: '',
							ctaHref: '/products#in-chat',
							ctaLabel: 'Learn more'
						}
					]
				},
				{
					id: 'transcription',
					title: 'Transcription',
					lead: 'Voice notes become text so members who cannot listen still follow along.',
					offers: [
						{
							id: 'transcription-individual',
							name: 'Transcription',
							scope: 'individual',
							blurb: 'Per-person auto transcription (`!transcribe-on`).',
							priceLabel: 'TBD',
							period: '',
							ctaHref: '/products#transcription',
							ctaLabel: 'Learn more'
						}
					]
				}
			],
			scopeLabels: {
				individual: 'Individual',
				group: 'Group'
			},
			footnote:
				'Paid pricing is not finalized yet. Alpha access is free with a code; checkout is not live.'
		},
		checkoutSuccess: {
			title: 'You are subscribed',
			lead: 'Next, link Sigstack to your Signal account so your plan unlocks in chat.',
			eyebrow: 'Checkout',
			planPurchased: 'Plan purchased: {plan}.',
			planPurchasedGeneric: 'Your plan is ready.',
			linkHeading: 'Link in Signal',
			linkBody: 'Copy the command, open Sigstack, paste and send in a DM.',
			copyLabel: 'Copy command',
			copyDoneLabel: 'Copied',
			messageCta: 'Message Sigstack',
			signalLinkMissing: 'Signal link not configured yet—ask your organizer how to DM Sigstack.',
			missingCode:
				'We could not find a link code in this page URL. Check your Stripe receipt email for `!link <code>`, then send that command in a DM with Sigstack.',
			portalCta: 'Manage billing',
			portalComingSoon: 'Manage billing (coming soon)',
			getStartedCta: 'Organizer checklist'
		},
		checkoutCancel: {
			title: 'Checkout canceled',
			lead: 'No charge was made and no entitlement was created. You can pick a plan whenever you are ready.',
			eyebrow: 'Checkout',
			plansCta: 'Back to Plans',
			getStartedCta: 'Organizer checklist'
		},
		alpha: {
			title: 'Alpha',
			lead: 'Enter your code, then message Sigstack in Signal.',
			eyebrow: 'Alpha',
			codeLabel: 'Alpha code',
			codePlaceholder: 'Enter your code',
			submitLabel: 'Continue',
			errorEmpty: 'Enter an alpha code to continue.',
			errorTooLong: 'That code is too long. Check the code you were given and try again.',
			linkHeading: 'Link in Signal',
			linkBody: 'Copy the command, open Sigstack, paste and send in a DM.',
			copyLabel: 'Copy command',
			copyDoneLabel: 'Copied',
			messageCta: 'Message Sigstack',
			signalLinkMissing: 'Signal link not configured yet—ask your organizer how to DM Sigstack.',
			enableNext:
				'After you link, create or open a Signal group, invite Sigstack, then run !enable-sigstack so everyone in that chat can use Sigstack.'
		}
	},
	legalPrivacy: {
		title: 'Privacy Policy',
		updated: 'August 2026',
		sections: [
			{
				title: 'Overview',
				body: 'Sigstack processes Signal group messages to provide translation and transcription. This stub summarizes processing for the public site; operators should align it with live deployment before collecting payment.'
			},
			{
				title: 'Data we process',
				body: 'Message text and voice-note audio the bot is configured to handle; Signal identifiers needed to post replies; encrypted preference state on the CVM volume.'
			},
			{
				title: 'Processors',
				body: 'Phala (TEE host), NEAR AI (chat translation and Whisper STT). Audio sent to Whisper is stripped of Signal metadata first.'
			},
			{
				title: 'Contact',
				body: 'For privacy questions about a Bread Cooperative deployment, contact the operators via the project GitHub repository.'
			}
		]
	},
	legalTerms: {
		title: 'Terms of Service',
		updated: 'August 2026',
		sections: [
			{
				title: 'Service',
				body: 'Sigstack is provided as a Signal automation for translation and transcription. Availability depends on the operator’s deployment.'
			},
			{
				title: 'Acceptable use',
				body: 'Do not use the bot to harass, break Signal’s terms, or process unlawful content. Organizers are responsible for group membership and consent.'
			},
			{
				title: 'No warranty',
				body: 'Translations and transcripts may contain errors. The service is provided as-is without guarantees of accuracy or uptime.'
			},
			{
				title: 'Changes',
				body: 'These terms are a public stub pending commercial launch. Operators may update them when subscriptions ship.'
			}
		]
	},
	legalLicense: {
		title: 'Apache License 2.0',
		updated: 'January 2004',
		copyright: 'Copyright 2024 Zaki Manian',
		overview:
			'Sigstack source code is licensed under the Apache License, Version 2.0. You may use, reproduce, and distribute the software subject to the terms below. The same text appears in the project LICENSE file.',
		fullText: `Apache License
                           Version 2.0, January 2004
                        http://www.apache.org/licenses/

   TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

   1. Definitions.

      "License" shall mean the terms and conditions for use, reproduction,
      and distribution as defined by Sections 1 through 9 of this document.

      "Licensor" shall mean the copyright owner or entity authorized by
      the copyright owner that is granting the License.

      "Legal Entity" shall mean the union of the acting entity and all
      other entities that control, are controlled by, or are under common
      control with that entity. For the purposes of this definition,
      "control" means (i) the power, direct or indirect, to cause the
      direction or management of such entity, whether by contract or
      otherwise, or (ii) ownership of fifty percent (50%) or more of the
      outstanding shares, or (iii) beneficial ownership of such entity.

      "You" (or "Your") shall mean an individual or Legal Entity
      exercising permissions granted by this License.

      "Source" form shall mean the preferred form for making modifications,
      including but not limited to software source code, documentation
      source, and configuration files.

      "Object" form shall mean any form resulting from mechanical
      transformation or translation of a Source form, including but
      not limited to compiled object code, generated documentation,
      and conversions to other media types.

      "Work" shall mean the work of authorship, whether in Source or
      Object form, made available under the License, as indicated by a
      copyright notice that is included in or attached to the work
      (an example is provided in the Appendix below).

      "Derivative Works" shall mean any work, whether in Source or Object
      form, that is based on (or derived from) the Work and for which the
      editorial revisions, annotations, elaborations, or other modifications
      represent, as a whole, an original work of authorship. For the purposes
      of this License, Derivative Works shall not include works that remain
      separable from, or merely link (or bind by name) to the interfaces of,
      the Work and Derivative Works thereof.

      "Contribution" shall mean any work of authorship, including
      the original version of the Work and any modifications or additions
      to that Work or Derivative Works thereof, that is intentionally
      submitted to the Licensor for inclusion in the Work by the copyright owner
      or by an individual or Legal Entity authorized to submit on behalf of
      the copyright owner. For the purposes of this definition, "submitted"
      means any form of electronic, verbal, or written communication sent
      to the Licensor or its representatives, including but not limited to
      communication on electronic mailing lists, source code control systems,
      and issue tracking systems that are managed by, or on behalf of, the
      Licensor for the purpose of discussing and improving the Work, but
      excluding communication that is conspicuously marked or otherwise
      designated in writing by the copyright owner as "Not a Contribution."

      "Contributor" shall mean Licensor and any individual or Legal Entity
      on behalf of whom a Contribution has been received by Licensor and
      subsequently incorporated within the Work.

   2. Grant of Copyright License. Subject to the terms and conditions of
      this License, each Contributor hereby grants to You a perpetual,
      worldwide, non-exclusive, no-charge, royalty-free, irrevocable
      copyright license to reproduce, prepare Derivative Works of,
      publicly display, publicly perform, sublicense, and distribute the
      Work and such Derivative Works in Source or Object form.

   3. Grant of Patent License. Subject to the terms and conditions of
      this License, each Contributor hereby grants to You a perpetual,
      worldwide, non-exclusive, no-charge, royalty-free, irrevocable
      (except as stated in this section) patent license to make, have made,
      use, offer to sell, sell, import, and otherwise transfer the Work,
      where such license applies only to those patent claims licensable
      by such Contributor that are necessarily infringed by their
      Contribution(s) alone or by combination of their Contribution(s)
      with the Work to which such Contribution(s) was submitted. If You
      institute patent litigation against any entity (including a
      cross-claim or counterclaim in a lawsuit) alleging that the Work
      or a Contribution incorporated within the Work constitutes direct
      or contributory patent infringement, then any patent licenses
      granted to You under this License for that Work shall terminate
      as of the date such litigation is filed.

   4. Redistribution. You may reproduce and distribute copies of the
      Work or Derivative Works thereof in any medium, with or without
      modifications, and in Source or Object form, provided that You
      meet the following conditions:

      (a) You must give any other recipients of the Work or
          Derivative Works a copy of this License; and

      (b) You must cause any modified files to carry prominent notices
          stating that You changed the files; and

      (c) You must retain, in the Source form of any Derivative Works
          that You distribute, all copyright, patent, trademark, and
          attribution notices from the Source form of the Work,
          excluding those notices that do not pertain to any part of
          the Derivative Works; and

      (d) If the Work includes a "NOTICE" text file as part of its
          distribution, then any Derivative Works that You distribute must
          include a readable copy of the attribution notices contained
          within such NOTICE file, excluding those notices that do not
          pertain to any part of the Derivative Works, in at least one
          of the following places: within a NOTICE text file distributed
          as part of the Derivative Works; within the Source form or
          documentation, if provided along with the Derivative Works; or,
          within a display generated by the Derivative Works, if and
          wherever such third-party notices normally appear. The contents
          of the NOTICE file are for informational purposes only and
          do not modify the License. You may add Your own attribution
          notices within Derivative Works that You distribute, alongside
          or as an addendum to the NOTICE text from the Work, provided
          that such additional attribution notices cannot be construed
          as modifying the License.

      You may add Your own copyright statement to Your modifications and
      may provide additional or different license terms and conditions
      for use, reproduction, or distribution of Your modifications, or
      for any such Derivative Works as a whole, provided Your use,
      reproduction, and distribution of the Work otherwise complies with
      the conditions stated in this License.

   5. Submission of Contributions. Unless You explicitly state otherwise,
      any Contribution intentionally submitted for inclusion in the Work
      by You to the Licensor shall be under the terms and conditions of
      this License, without any additional terms or conditions.
      Notwithstanding the above, nothing herein shall supersede or modify
      the terms of any separate license agreement you may have executed
      with Licensor regarding such Contributions.

   6. Trademarks. This License does not grant permission to use the trade
      names, trademarks, service marks, or product names of the Licensor,
      except as required for reasonable and customary use in describing the
      origin of the Work and reproducing the content of the NOTICE file.

   7. Disclaimer of Warranty. Unless required by applicable law or
      agreed to in writing, Licensor provides the Work (and each
      Contributor provides its Contributions) on an "AS IS" BASIS,
      WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
      implied, including, without limitation, any warranties or conditions
      of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
      PARTICULAR PURPOSE. You are solely responsible for determining the
      appropriateness of using or redistributing the Work and assume any
      risks associated with Your exercise of permissions under this License.

   8. Limitation of Liability. In no event and under no legal theory,
      whether in tort (including negligence), contract, or otherwise,
      unless required by applicable law (such as deliberate and grossly
      negligent acts) or agreed to in writing, shall any Contributor be
      liable to You for damages, including any direct, indirect, special,
      incidental, or consequential damages of any character arising as a
      result of this License or out of the use or inability to use the
      Work (including but not limited to damages for loss of goodwill,
      work stoppage, computer failure or malfunction, or any and all
      other commercial damages or losses), even if such Contributor
      has been advised of the possibility of such damages.

   9. Accepting Warranty or Additional Liability. While redistributing
      the Work or Derivative Works thereof, You may choose to offer,
      and charge a fee for, acceptance of support, warranty, indemnity,
      or other liability obligations and/or rights consistent with this
      License. However, in accepting such obligations, You may act only
      on Your own behalf and on Your sole responsibility, not on behalf
      of any other Contributor, and only if You agree to indemnify,
      defend, and hold each Contributor harmless for any liability
      incurred by, or claims asserted against, such Contributor by reason
      of your accepting any such warranty or additional liability.

   END OF TERMS AND CONDITIONS

   Copyright 2024 Zaki Manian

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.`
	}
};
