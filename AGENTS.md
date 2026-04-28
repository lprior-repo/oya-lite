# oya-lite — Opencode CLI Orchestrator

`oya-lite` is a Rust CLI tool that wraps the `opencode` CLI to run AI-driven development tasks as structured, persisted lifecycles. It takes a **bead ID** as input and orchestrates a reproducible workflow against `opencode` (either as a subprocess or HTTP server), with full state persistence via Fjall and structured progress output.

## Quick Start

### Build

```bash
cargo build --release
```

### Run (subprocess mode — no server)

```bash
./target/release/oya-lite \
  --bead-id my-feature-1 \
  --model anthropic/claude-sonnet-4-20250514 \
  --prompt "Implement error handling in the lifecycle module" \
  --data-dir .oya-lite
```

### Run (server mode — HTTP API to opencode)

```bash
OPENCODE_SERVER_PASSWORD=secret \
  ./target/release/oya-lite \
  --bead-id my-feature-1 \
  --server http://localhost:4099 \
  --server-password secret \
  --prompt "Review and fix the opencode error predicate"
```

## Architecture

```
main.rs (CLI entry, clap args)
  └─ LifecycleOrchestrator (lifecycle/run/lifecycle.rs)
       ├─ StateDb (Fjall KV store — workflows + journal keyspaces)
       ├─ TokioCommandExecutor (subprocess runner via tokio::process)
       └─ OpencodeServer (HTTP client to opencode server API)

lifecycle/run/lifecycle.rs — orchestrator: spawns step pipeline, manages state transitions
lifecycle/run/lifecycle/steps.rs — step execution loop with progress channel
lifecycle/run/lifecycle/opencode_server.rs — HTTP client for opencode server API
lifecycle/effects/executor.rs — subprocess command runner with timeouts
lifecycle/effects/run.rs — effect dispatch, timeout config, error classification
lifecycle/state/state_db.rs — Fjall-backed persistence (batch + flush)
lifecycle/state/persist.rs — batch persist state + journal
lifecycle/types/ — DDD value types: BeadId, ModelId, Effect, WorkflowState, LifecycleProgress
lifecycle/types/state_machine.rs — FSM: Planned → WorkspaceReady → Executing → Completed/Failed
lifecycle/error.rs — Terminal vs Transient errors with FailureCategory
```

### State Machine

```
Planned → WorkspaceReady → Executing (step N) → Completed / Failed
```

- `Planned` — lifecycle created, no work done
- `WorkspaceReady` — workspace directory created
- `Executing` — running a specific effect step
- `Completed` — all steps succeeded (terminal)
- `Failed` — a step failed (terminal)

### Lifecycle Steps

1. **workspace-prepare** — creates the workspace directory (`mkdir -p <path>`)
2. **opencode-run** — runs the opencode CLI (subprocess or server mode) with the given prompt and model

## CLI Reference

```
oya-lite [OPTIONS] --bead-id <BEAD_ID>

Options:
  --data-dir <DATA_DIR>              Data directory for Fjall DB (default: .oya-lite)
  --bead-id <BEAD_ID>                Bead identifier [required]
  --model <MODEL>                    Model ID (e.g. anthropic/claude-sonnet-4) [default: anthropic/claude-sonnet-4-20250514]
  --repo <REPO_URL>                  Repository URL
  --prompt <PROMPT>                  Prompt to send to opencode
  --server <URL>                     Opencode server URL (enables HTTP mode)
  --server-user <USER>               Server username (default: opencode)
  --server-password <PASSWORD>       Server password (env: OPENCODE_SERVER_PASSWORD)
  -h, --help                         Print help
  -V, --version                      Print version
```

## Bead ID Validation

- Max 64 chars
- Only lowercase ASCII letters, digits, and hyphens
- No whitespace, no slashes, no uppercase

## Opencode Integration

### Subprocess Mode (default)

Runs `opencode run --format json --model <model> <prompt>` as a subprocess. Times out after 3600s. Detects errors via JSON parsing of stdout for `"type":"error"` markers and stderr for `ProviderModelNotFoundError` / `Model not found`.

### Server Mode (`--server`)

Uses the opencode HTTP API:
1. `POST /session` — creates a session with basic auth
2. `POST /session/<id>/message` — sends a message with provider/model/prompt
3. Parses response body for error markers (`"type":"error"`)

Error messages are sanitized before display — raw opencode errors are replaced with `"opencode model not found or unavailable"` to prevent secret leaks and stack trace exposure.

## Persistence

All state is persisted to Fjall (LSM-tree embedded KV store) in the `--data-dir` directory:
- `workflows` keyspace: bead state JSON (serialized `WorkflowState`)
- `journal` keyspace: per-bead effect journal entries (keyed by `bead_id_ts_seq`)

Batch writes state + journal together, then flushes to disk. Survives process restart.

## Quality Guarantees

- `#![forbid(unsafe_code)]` — no unsafe blocks
- `#![deny(clippy::unwrap_used)]` — no `unwrap()` anywhere except test code
- `#![deny(clippy::expect_used)]` — no `expect()` anywhere except test code  
- `#![deny(clippy::panic)]` — no `panic!()` anywhere except test code
- `#[warn(clippy::cognitive_complexity)]` — keeps functions readable
- 95+ tests including unit, integration, proptest, and mutation-killing tests
- All clippy warnings denied, all tests pass, release builds clean

## Running the Full Suite

```bash
# Lint
cargo clippy --tests -- -D warnings -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic

# Tests
cargo test

# Release build
cargo build --release
```

## Progress Output

All lifecycle progress is emitted as pretty-printed JSON to stdout:

```json
{
  "Initialized": {
    "bead_id": "my-feature-1",
    "steps": ["workspace-prepare", "opencode-run"]
  }
}
{
  "StepStarted": {
    "step": "workspace-prepare",
    "started_at": "2025-04-27T06:00:00.000000+00:00"
  }
}
...
{
  "Finished": {
    "result": "success",
    "message": "all steps completed"
  }
}
```

The exit code is 0 only if all steps succeed. Any `StepFailed` event causes exit code 1.
