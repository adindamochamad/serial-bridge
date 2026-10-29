# Safe Mode — repo notes

Last verified on the **current working tree** after **M5** (prompt-injection demo, docs, tests). Baseline PRD commit was `075b35e`; line numbers below drift with edits.

Locked decisions: [serialbridge-safe-mode-plan.md](./serialbridge-safe-mode-plan.md). PRD copy: [serialbridge-safe-mode-prd.md](./serialbridge-safe-mode-prd.md). (Historical name: OmniBridge.)

## §2.1 fact table (post-M5)

| Area | Fact |
|------|------|
| Stack | Tauri **2.10.3** (`src-tauri/Cargo.lock`). SvelteKit 2 + Svelte 5 + `@sveltejs/adapter-static`. TypeScript **~5.6.2**, Vite **^6.0.3**. Version **0.2.3** (`package.json`) vs **0.2.2** (`src-tauri/Cargo.toml`, `tauri.conf.json`) — leave mismatch per PRD. |
| Rust deps | `tauri`, `tauri-plugin-opener`, `tauri-plugin-serialplugin` 2.22.0, `tauri-plugin-store`, `serialport` 4.9.0, `serde`, `serde_json`, `reqwest` 0.12 (rustls), **`regex`** (policy patterns), **`sha2`** (audit hash chain). No direct `tokio`; approval wait uses `std::sync::mpsc` + `tauri::async_runtime::spawn_blocking`. No SQLite. |
| Rust layout | `lib.rs`, `discovery.rs`, `main.rs`, **`safemode/`** (`mod.rs`, `policy.rs`, `rate_limit.rs`, `approval.rs`, **`gate.rs`**, **`audit.rs`**). |
| Serial write path | Shared **`write_to_port`**. Human **`send_bytes_to_port`** writes then logs `human_write`. Agent writes go through **`safemode_agent_send_bytes`** and **`finish_after_approval`**. |
| Tauri commands | Existing serial/webhook commands plus **`safemode_begin_session`**, **`safemode_authorize_tool`**, **`safemode_resolve_approval`**, **`safemode_agent_send_bytes`** (async), **`safemode_read_audit`**, **`safemode_get_enabled`**, **`safemode_set_enabled`**. |
| Agent tools | `executeTool` authorizes first. **`send_bytes`** calls **`safemode_agent_send_bytes`** only. Write denials use FR-9 reasons: `rate_limited`, `session_blocked`, `safe_mode_policy_invalid`, otherwise `policy_denied:<rule>`. |
| Invalid policy | Known read tools stay allowed. `send_bytes` is denied. Unknown tools are denied. |
| Approval UI | `ApprovalDialog.svelte` on `+page.svelte`. |
| Safe Mode toggle | `SettingsModal.svelte`. Off requires confirm text `MATIKAN`. Missing enabled file means **on**. State file: `<app_config_dir>/safemode-enabled.json`. |
| Demo injection | `prompt-injection` scenario in `demo.ts`; gate `demo://` → `simulated: true` on approve; [DEMO_SCRIPT.md](./DEMO_SCRIPT.md), [README.md](./README.md). |
| Audit UI | `AuditLogPanel.svelte` inside Settings. Read-only, filter by event and port. |
| Capabilities | [`default.json`](../../src-tauri/capabilities/default.json): **`core:default`**, **`opener:default`**, **`store:default`**. **`tauri_plugin_serialplugin::init()`** still registered. `rg serialplugin src` is empty. |
| Paths | Policy: `<app_config_dir>/safemode-policy.json`. Audit: `<app_data_dir>/safemode-audit.jsonl`. |
| Tests / CI | Vitest: `safemode`, `demo` (injection line), `agent-tools` (no `send_bytes_to_port`). Rust: policy, gate (incl. demo simulated), approval, audit, FR-9, toggle. CI: `cargo fmt`, clippy `-D warnings`, `cargo test`, `npm run check`, `npm test`, `npm run build`. |
| CSP | `tauri.conf.json`: `"csp": null`. |

## Differences from original M0 snapshot

| Topic | M0 / PRD §2.1 | Post-M5 |
|-------|----------------|---------|
| Agent `send_bytes` | Direct `send_bytes_to_port` | Gated `safemode_agent_send_bytes` + approval dialog |
| `toolUseId` | Not wired | `tu.id` → gate |
| Capabilities | `serialplugin:default` + `allow-available-ports` | Removed; plugin `init()` remains |
| Safe Mode modules | Not present | `safemode/*` including `gate.rs` and `audit.rs` |
| Audit view | Not present | `AuditLogPanel.svelte` in Settings |
| Demo injection fixture | Not present | `prompt-injection` scenario, M5 tests, [DEMO_SCRIPT.md](./DEMO_SCRIPT.md) |

## M0.2 — `serialplugin` (reconfirmed after M2.4)

- **Frontend:** no `serialplugin` imports under `src/`.
- **Discovery / monitor / probe / Command Sender:** `serialport` + Tauri commands; writes via `write_to_port` / `send_bytes_to_port`.
- **`init()`:** still registered; dropping it not validated in this session.
- **Capabilities:** narrowed as above; no `serialplugin` permission grants.

## Module layout in use

Approved M0 list **plus** `src-tauri/src/safemode/gate.rs` (commands, state, approval settlement).

- `mod.rs` — `PolicyEngine`. Invalid policy denies writes only; known reads stay allowed.
- `policy.rs`, `rate_limit.rs`, `approval.rs` — pending map uses `take`; the command sends the bool outside the state mutex.
- `audit.rs` — append-only JSONL, SHA-256 `prev_hash` chain, `verify_file`.
- `src/lib/safemode.ts`, `ApprovalDialog.svelte`, `AuditLogPanel.svelte`.

## Manual regression checklist

**Not run on real hardware in the session that updated this file.** Fill pass/fail when verified locally.

| Feature | M2 | M4 | Notes |
|---------|----|----|-------|
| Device discovery | | | `discover_devices` / `serialport` |
| Start/stop port monitor | | | |
| RX `port:data` display | | | |
| Command Sender write | | | `send_bytes_to_port` logs `human_write` |
| Baud probe (port not monitored) | | | |
| Webhook forward | | | |
| Demo virtual port | | | |
| One-shot analysis | | | |
| Agent Investigate + Safe Mode gate | auto | auto | Vitest + Rust gate tests; manual: [DEMO_SCRIPT.md](./DEMO_SCRIPT.md) |
| Approval timeout / queue | auto | auto | Rust `approval.rs` tests |
| Audit panel + Safe Mode toggle | auto | auto | Rust toggle test; manual UI smoke |
| Prompt injection demo | auto | auto | `tests/demo.test.ts`, Rust demo simulated write |
