# Alpha code ops (mint / list / revoke)

Single-use alpha codes are pending `bundle-all-alpha` rows in encrypted `entitlements.enc` on the live CVM. The reusable friend code `bread` is separate and is **not** managed by this CLI.

CLI binary: `mint-alpha` (`crates/signal-bot`).

## Safety

- Run only against the **live** prefs/entitlements volume (or a throwaway local memory/file for dry runs).
- **Never commit** minted codes to git. Copy stdout to a secure channel, then clear the terminal scrollback if needed.
- In-place Phala upgrades only — do not wipe `group-prefs-translation` / `signal-config-translation`. See [`docs/one-cvm-architecture.md`](../one-cvm-architecture.md).

## Env

| Variable | Default |
|----------|---------|
| `ENTITLEMENTS__STORAGE_PATH` | `/data/entitlements.enc` |
| `DSTACK__SOCKET_PATH` | `/var/run/dstack.sock` |
| `ENTITLEMENTS__LEGACY_COMPOSE_HASH` | (optional) |

On Phala, use the same sealed env / volume mount as `signal-bot`.

## Commands

```bash
# Mint N single-use codes (90-day expiry default). Prints one token per line.
cargo run -p signal-bot --release --bin mint-alpha -- mint --count 10 --days 90

# Back-compat (same as mint):
cargo run -p signal-bot --release --bin mint-alpha -- --count 10

# List pending + bound alpha rows (TSV: state, status, expires, owner, record_id, code)
cargo run -p signal-bot --release --bin mint-alpha -- list

# Revoke unused pending code, bound record id, or all alpha for an owner UUID
cargo run -p signal-bot --release --bin mint-alpha -- revoke <code|record-id|owner-uuid>
```

### On the CVM

Prefer a one-shot container or `phala ssh` with the bot image and `/data` volume attached (same as production `ENTITLEMENTS__*` paths). After mint/revoke, the bot reloads from disk via flock / `reload_if_stale` — no volume wipe.

Example distribution URL shape (site):

```text
https://breadchaincoop.github.io/sigstack-bot/sigstack/alpha/?code=<token>
```

User then DMs Sigstack: `!link <token>`, then `!enable-sigstack` in each group.

## Revoke behavior

| Target | Effect |
|--------|--------|
| Pending `link_token` | Removes unused code (cannot `!link`) |
| Bound `record_id` | Sets that alpha row to `canceled` |
| `owner_uuid` | Cancels all alpha rows for that owner |

Stripe-paid rows are never touched. Already-canceled rows are reported unchanged.

## Related

- Catalog / packs: [`docs/commerce/stripe-catalog.md`](stripe-catalog.md)
- E2E before enforce: [`docs/commerce/e2e-checklist.md`](e2e-checklist.md)
- Tracker: [`.agents/docs/github-work-index.md`](../../.agents/docs/github-work-index.md)
