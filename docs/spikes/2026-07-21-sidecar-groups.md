# Spike: Signal sidecar groups + bot self-receive (Phase 0)

Date: 2026-07-21  
Context: Mutual-aid language sidecar plan (`translate-me` bridge).

## Local observation (pre-implementation)

Against a local `signal-cli-rest-api` with bot `+573107677679` sending into a group:

1. After the bot sent a group message, `/v1/receive` for the bot account returned a **delivery receipt**, not a `dataMessage` echoing the bot’s own text.
2. Human envelopes often use `source` = Signal UUID; `sourceNumber` may be null depending on contact sync.
3. Implication: relay loops are **unlikely** if we only act on inbound `dataMessage`s, but we still implement `BotIdentity` skip (phone + learned UUID) as defense in depth.

## REST contract (implemented + wiremocked)

| Call | Path |
|------|------|
| Create | `POST /v1/groups/{botPhone}` body `{ name, members, description? }` → `{ id: "group.…" }` |
| List | `GET /v1/groups/{botPhone}` → `{ id, internal_id, name }[]` |
| Add | `POST /v1/groups/{bot}/{groupSendId}/members` `{ members }` |
| Remove | `DELETE` same path + members body |
| Send | `POST /v2/send` recipients `["group.…"]` |

Inbound envelopes use `groupInfo.groupId` = list-groups `internal_id`.

## Manual checklist (still for alpha)

- [ ] Create group with member A → 201 + `group.` id; A accepts invite
- [ ] Add member B; send to group; A and B receive
- [ ] Capture any self-receive JSON (`source` / `sourceNumber` / `sourceUuid`) on CVM
- [ ] Remove member; bad member error copy

## Exit

Client methods + identity helper + bridge store are in tree. Full create/add/send on the pinned Phala image remains an alpha soak item.
