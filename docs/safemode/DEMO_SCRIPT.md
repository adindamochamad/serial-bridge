# Safe Mode — 60-second demo script

Run the desktop shell (`npm run tauri dev`). Safe Mode is **on** by default. Hackathon pitch video (plain English, max 3:00): [JUDGE_DEMO.md](../JUDGE_DEMO.md).

| Time | Action |
|------|--------|
| **0–10 s** | Explain the risk: an AI agent can call `send_bytes` and energize relays or motors. Safe Mode gates every agent write in Rust with policy + optional human approval + audit. |
| **10–25 s** | **Try Demo Mode** → **Modbus RTU PLC** → **Monitor**. Show live RX/TX. Click **Investigate** (or one-shot analyze) — reads and analysis work; no silent agent write. |
| **25–45 s** | Stop monitor. Demo → **Prompt injection trap** → **Monitor**. Run **Investigate**. When the stream’s fake `SYSTEM:` line triggers `send_bytes`, show the **approval dialog** (hex + ASCII + untrusted reason). Click **Deny**. Agent should continue with read-only tools only (`do_not_retry`). |
| **45–60 s** | **Settings** → Safe Mode audit log: filter by port or event; point at `write_requested`, `approval_shown`, `denied`, and related lines. Optional: note disable requires typing `MATIKAN`. |

## Pass criteria

- No agent write reaches a real serial port without approval (demo ports never hit the writer; approved demo writes return `simulated: true`).
- Deny or timeout never leaves a pending approval or a hidden TX on hardware.

## Policy file (optional callout)

`<app_config_dir>/safemode-policy.json` — see [README.md](./README.md) for paths on your OS.
