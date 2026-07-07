# Poa integration (DAO task tools)

This bot can read and update a [Poa](https://github.com/poa-box) DAO organization's
work board. Poa is a DAO coordination protocol: each org is a set of upgradeable
BeaconProxy contracts (voting, participation token, and a **TaskManager** that
runs a project/task lifecycle). These tools let the LLM answer questions about an
org's projects and tasks and — for authorized operators — create and manage
tasks on-chain, all from inside the TEE.

The bot's wallet key is **derived inside the enclave** (dstack `derive_key`) by
default, so no private key ever leaves the TEE. Write actions are gated twice: by
a global `enable_writes` switch and by a per-sender allowlist.

## Crate layout

- `crates/poa-tools/` — the tool implementations.
  - `client.rs` — wallet + RPC + subgraph endpoints (`PoaClient`).
  - `subgraph.rs` — read queries against the Poa subgraph.
  - `contract.rs` — `alloy` `sol!` bindings + transaction senders for TaskManager.
  - `tools_read.rs` — read tools (no authorization).
  - `tools_write.rs` — write tools (`requires_authorization() == true`).
  - `units.rs` — 18-decimal participation-token parsing/formatting.
- `crates/tools/` — gained `Tool::requires_authorization()` and
  `Tool::timeout_override()`, plus `ToolRegistry::get_definitions_authorized()`
  and `ToolExecutor::execute_authorized()` to enforce the gate.
- `crates/signal-bot/` — `PoaConfig`, `register_poa_tools()` in `main.rs`, and a
  `ToolAuthorization` gate on `ChatHandler`.

## Tools

Read tools (offered to everyone when Poa is enabled):

| Tool | Purpose |
|------|---------|
| `poa_list_projects` | List the org's projects (id, title, task count). |
| `poa_list_tasks` | List tasks, optionally filtered by status/project. |
| `poa_get_task` | Full detail of one task (call before `poa_update_task`). |
| `poa_wallet_info` | Bot wallet address, gas balance, configured TaskManager. |

Write tools (only offered to / executed for allowlisted senders, and only when
`enable_writes` is true):

| Tool | On-chain call | Permission the wallet needs |
|------|---------------|-----------------------------|
| `poa_create_task` | `createTask` | project manager or `CREATE` hat |
| `poa_update_task` | `updateTask` | PM or `EDIT_FULL` (or `CREATE` while unclaimed) |
| `poa_assign_task` | `assignTask` | PM or `ASSIGN` hat |
| `poa_complete_task` | `completeTask` | PM or `REVIEW` hat (mints payout) |
| `poa_reject_task` | `rejectTask` | PM or `REVIEW` hat |
| `poa_cancel_task` | `cancelTask` | PM or `CREATE` hat |

Payouts are participation tokens and are given as decimals (`"5"`, `"2.5"`) — the
tool converts to 18-decimal wei. Metadata/rejection text is accepted either as a
`0x`-prefixed bytes32 (e.g. an IPFS CID digest) or as free text, which is
sha256-hashed on chain.

## Configuration

All keys live under `TOOLS__POA__` (see `.env.example`):

| Env var | Default | Notes |
|---------|---------|-------|
| `TOOLS__POA__ENABLED` | `false` | Master switch. |
| `TOOLS__POA__RPC_URL` | — | JSON-RPC for the org's chain. |
| `TOOLS__POA__SUBGRAPH_URL` | — | Poa subgraph GraphQL endpoint. |
| `TOOLS__POA__TASK_MANAGER` | — | The org's TaskManager proxy address. |
| `TOOLS__POA__NETWORK_NAME` | `gnosis` | Shown to users. |
| `TOOLS__POA__PRIVATE_KEY` | — | Dev only; unset ⇒ TEE-derived key. |
| `TOOLS__POA__DERIVE_KEY_PATH` | `poa-tools/task-manager-wallet` | dstack derivation path. |
| `TOOLS__POA__ENABLE_WRITES` | `false` | Enable write tools at all. |
| `TOOLS__POA__AUTHORIZED_SENDERS` | — | Comma/space-separated Signal ids allowed to write. |

Poa governance orgs live on **Arbitrum**; test orgs (KUBI, Test6, …) on
**Gnosis**. Point `RPC_URL` + `SUBGRAPH_URL` at the chain your org is on. Subgraph
endpoints:

- Gnosis: `https://api.studio.thegraph.com/query/73367/poa-gnosis-v-1/version/latest`
- Arbitrum: `https://api.studio.thegraph.com/query/73367/poa-arb-v-1/version/latest`

### Finding an org's TaskManager

Query the subgraph by orgId:

```bash
curl -s -X POST "$TOOLS__POA__SUBGRAPH_URL" \
  -H 'Content-Type: application/json' \
  -d '{"query":"{ organization(id:\"0x<orgId>\"){ name taskManager{ id } } }"}'
```

## Authorization model

1. **Read tools** are always available when `ENABLED=true`.
2. **Write tools** are registered only when `ENABLE_WRITES=true`.
3. Even then, a write tool is only *offered* to the model (via
   `get_definitions_authorized`) and only *executed* (via `execute_authorized`)
   when the Signal sender is in `AUTHORIZED_SENDERS`. The executor refuses a
   privileged tool for any other sender as a defense-in-depth backstop.
4. Authorization keys off the **individual sender's number**, even in group
   chats — never the group id.

On top of the bot-side gate, the chain is the final authority: a write only
succeeds if the bot's wallet holds the relevant TaskManager permission. Granting
that permission is a governance action on the Poa side — see the
`SIGSTACK_BOT_INTEGRATION.md` doc in the Poa contracts repo (`macau-v6`).

## Getting the bot's wallet address

Start the bot with Poa enabled and check the log line:

```
Poa wallet address: 0x… (grant this address project-manager rights on-chain)
```

or ask the bot (`poa_wallet_info`). Grant that address project-manager or the
appropriate role hat on the target project, then fund it with a little gas on the
org's chain.

## Testing

```bash
cargo test -p poa-tools          # unit tests (subgraph parsing, units, tool split)
cargo test -p tools -p signal-bot
```

Subgraph parsing is covered with `wiremock`. The on-chain senders are thin
wrappers over `alloy` and are exercised end-to-end against a testnet org rather
than in unit tests.
