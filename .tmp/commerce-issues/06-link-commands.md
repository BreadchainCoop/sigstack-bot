Parent: #13

## Summary

Bind Signal identity to entitlements after payment or alpha claim.

## `!link <code>`

- [ ] Accept code from DM (preferred) or group (warn about leakage)
- [ ] Match pending entitlement by `link_token` OR alpha code (alpha codes issue)
- [ ] Bind `message.source` (UUID) as `owner_uuid`
- [ ] Reply with plan summary + next steps (invite bot, run product commands)
- [ ] Reject: unknown code, already linked, expired

## `!claim-group`

- [ ] Only entitlement **owner** in the group
- [ ] Only for `scope: group` plans (bundle-group, in-chat-all, etc.)
- [ ] Record `claimed_group_id` = group `internal_id`
- [ ] One subscription → one group (**MVP**); multi-group is future upsell

## Decision: auto-claim

| Option | Behavior |
|--------|----------|
| A | Require explicit `!claim-group` |
| B | Auto-claim when owner runs first group-scoped command (`!translate-all-on`) |

Recommend **B with explicit `!claim-group` also available**.

## Depends on

Entitlement store issue
