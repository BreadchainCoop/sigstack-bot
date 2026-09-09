Parent: #13

## Summary

Align all user-facing copy with pay → link → invite → enable. Fix confusion between "auto-accepts invites" and "self-subscribe".

## Website updates ([`en.ts`](site/src/lib/content/en.ts))

- [ ] Get-started steps: **Subscribe (or redeem alpha)** → **Link in Signal (`!link`)** → **Invite CipherSlate** → **Enable product**
- [ ] Plans page: clarify Individual vs Group scope
- [ ] Privacy/terms stubs: update before live payments ([`pages.terms`](site/src/lib/content/en.ts))

## Bot replies (entitlement gating companion)

- [ ] Unlinked: "Complete linking: `!link <code>` from your receipt"
- [ ] Unpaid: "Subscribe at <site>/plans"
- [ ] Expired alpha: "Alpha ended — subscribe to continue"
- [ ] `!billing` → Stripe Customer Portal URL (optional sub-issue if too large)

## Acceptance

- [ ] E2E smoke tests updated in [`smoke.e2e.ts`](site/src/routes/smoke.e2e.ts)
