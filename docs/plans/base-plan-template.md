# {Feature Title}

**Date:** YYYY-MM-DD  
**Status:** Draft | In Review | Approved | Implemented | Superseded  
**Authors:** {name(s)}  
**Related:** {links to other plans, issues, PRs}

---

## Overview

One paragraph: what we are building and why it matters for Signal Bot TEE (privacy, TEE constraints, user value).

## Goals

1. Primary goal
2. Secondary goal
3. Non-goals (explicit scope boundaries)

## Decisions Made

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Example | Option A | Why not B |

> Add rows as design choices are locked. Update **Status** when approved.

## Current Architecture (Baseline)

Brief summary of relevant existing components (`signal-api`, `signal-bot`, compose stack, TEE model). Reference files/crates that exist today.

## Proposed Architecture

### High-Level Flow

```
ASCII or mermaid diagram showing data path through TEE boundary
```

### New / Modified Components

| Component | Type | Responsibility |
|-----------|------|----------------|
| `crates/example/` | New crate | … |
| `signal-bot` | Modify | … |
| `docker/phala-compose.yaml` | Modify | … |

### Crate / Service Structure (if applicable)

```
crates/example/
├── Cargo.toml
└── src/
    ├── lib.rs
    └── ...
```

## Security & TEE Considerations

- What must run inside the CVM with `signal-api` (plaintext boundary)
- What data is ephemeral vs persisted (and why)
- Attestation impact (compose hash, new services, pinned image digests)
- Metadata leakage the operator can still observe
- Threat model: what this feature does **not** protect against

## Integration Points

| Location | Change |
|----------|--------|
| `crates/signal-bot/src/main.rs` | … |
| `docker/docker-compose.yaml` | … |
| `docker/phala-compose.yaml` | … |

Include code sketches only when they clarify the design:

```rust
// Example integration snippet
```

## Configuration

Environment variables (both `docker/.env` compose names and `SIGNAL__`-style if applicable):

| Variable | Default | Description |
|----------|---------|-------------|
| `EXAMPLE__ENABLED` | `false` | Master switch |

## API / User-Facing Behavior

### Signal Commands (if any)

| Command | Behavior |
|---------|----------|
| `!example` | … |

### HTTP Endpoints (if any)

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/v1/example` | … |

## Dependencies

```toml
# New workspace / crate dependencies
```

External services (Whisper, LibreTranslate, NEAR AI, etc.) and whether they run in-TEE or off-device.

## Resource Requirements (Phala CVM)

| Resource | Local dev | Production CVM |
|----------|-----------|----------------|
| vCPU | | |
| Memory | | |
| Disk | | |

## Files to Modify

| File | Changes |
|------|---------|
| `path/to/file` | Brief description |

## Testing Plan

- [ ] Unit tests (`cargo test -p …`)
- [ ] Local compose stack (`docker compose up`)
- [ ] Manual Signal end-to-end test
- [ ] TEE attestation smoke test (`!verify`) after compose change
- [ ] Regression: existing text chat / tools / registration unaffected

## Implementation Phases

### Phase 1: {name}

- [ ] Task 1
- [ ] Task 2

### Phase 2: {name}

- [ ] Task 1

> For step-by-step implementation detail (file paths, commits, exact commands), create a companion doc:  
> `YYYY-MM-DD-{feature}-implementation.md`  
> See `2024-12-15-tool-use-implementation.md` for the detailed task format.

## Open Questions

1. Unresolved decision or spike needed

## References

- [Link to external doc or repo]()
- `CLAUDE.md` — TEE security model
- Related plan: `docs/plans/…`

---

## Appendix: Naming Conventions for This Folder

| Pattern | Example | Use when |
|---------|---------|----------|
| `YYYY-MM-DD-{feature}-design.md` | `2024-12-15-tool-use-system-design.md` | Architecture, decisions, security |
| `YYYY-MM-DD-{feature}-implementation.md` | `2024-12-15-tool-use-implementation.md` | Step-by-step build tasks with commits |
| `YYYY-MM-DD-{feature}-impl.md` | `2024-12-21-fund-sweeping-multichain-impl.md` | Shorter implementation companion |
| `YYYY-M-D-{feature}.md` | `2026-6-22-whisper-integration.md` | Combined tech spec (design-first) |
| `{feature}-integration.md` | `x402-payment-integration.md` | Cross-cutting feature, design + checklist |

**Status lifecycle:** `Draft` → `In Review` → `Approved` → `Implemented` (or `Superseded` with link to replacement).
