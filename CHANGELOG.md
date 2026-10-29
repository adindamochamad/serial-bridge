# Changelog

## Unreleased

- Verified Investigate runs host `analyze_binary_structure` on RX bytes before the model, so Modbus CRC evidence does not depend on a tool call.
- Demo Modbus PLC emits a master poll (TX) and a CRC-valid slave response (RX) each tick; demo writes get a slave reply.
- Rename remaining Indonesian identifiers in the front end; persisted store keys (`mode_webhook`, `url_online`, profile `sumber`) are unchanged.
- Fix Settings **Test connection** after host-only key save (button enabled + `count_tokens` probe).
- Meta Model API proxy in Rust: `LLM_` keys in host config only; strip `cache_control` on forwarded Messages bodies; `meta_count_tokens` for connection test.
- Rebrand crate and docs from OmniBridge to SerialBridge; release workflow drops `VITE_ANTHROPIC_API_KEY`.
- Pitch video script: English ~3 min (problem, live demo, naive vs verified, injection trap) in [JUDGE_DEMO.md](docs/JUDGE_DEMO.md); empty-state copy matches the pitch.
- Groqbot prompt: plain-English desk test with Linear / Screen Studio motion (cursor zoom, sliding panels), not a locked screenshot: [docs/PITCH_VIDEO_GROKBOT.md](docs/PITCH_VIDEO_GROKBOT.md).
- Safe Mode M5: `docs/safemode/README.md`, `docs/safemode/DEMO_SCRIPT.md`, prompt-injection Vitest + Rust simulated `demo://` write tests.
- Harden native webhook forwarding: POST/PUT/PATCH only, URL validation, 10s timeout, no redirects, filtered headers, body size cap.
- Set Content-Security-Policy on the main window (IPC + Meta API + dev HMR).
- Stop inlining Meta Model API keys from `VITE_*` at build; keys come from Settings store only.
- Command Sender requires click-again confirmation within 4s before transmitting; human writes respect Safe Mode deny-list and max payload size.
- ONLINE webhook mode uses persisted `url_online` instead of a fixed production constant.
- Version aligned to 0.2.2 across npm and Tauri config; removed unused `tauri-plugin-serialplugin`.
- Parser dashboard path reuses compiled regex per field set (`parseStreamWithCompiled`).
- Monitor write queue bounded; read errors emit `port:error`.
- Remove ENTIS-specific webhook branding, form field, and hostnames; auto-forward uses the generic webhook enable toggle.
- Remove scale-only Kode / Co.ID / DEV controls from port cards; webhook uses a single URL field.

### Manual verification checklist

Demo path (no serial cable; covered in unit tests except live Meta Investigate):

- [x] Demo Modbus TX poll + RX response buffer (`tests/agent-tools.test.ts`, `tests/commit-gate.test.ts`)
- [x] Host preflight CRC evidence can commit the verified lane without a model tool call
- [ ] Live **Investigate** with a Meta Standard key in the desktop shell (operator)

Hardware (not covered by CI — leave unchecked until run on a real port):

- [ ] Port monitor start/stop
- [ ] CAS warmup 19200→9600 when applicable
- [ ] Command Sender confirm-then-send
- [ ] Webhook POST (DEV and ONLINE URLs)
- [ ] Settings API key test; Analyze or Investigate
- [ ] `npm run tauri dev` — IPC and CSP (no broken shell)
