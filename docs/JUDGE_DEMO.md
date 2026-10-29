# Pitch video (English, max 3:00)

**Line judges should remember:** SerialBridge lets AI read an undocumented machine — and never lets it write to hardware without a human.

Record the **desktop app only**. Target **2:40**; hard cap **3:00**. One take if you can; cut only at the scene beats below.

```bash
npm run tauri dev
```

**Before record:** Meta Standard key already saved in **Settings**. Demo Mode off. No ports monitored. Cursor not over developer chrome.

Technical Safe Mode rehearsal (not this video): [safemode/DEMO_SCRIPT.md](./safemode/DEMO_SCRIPT.md).

Groqbot prompt (real desk-test look, plain VO + SRT): [PITCH_VIDEO_GROKBOT.md](./PITCH_VIDEO_GROKBOT.md).

---

## One-sentence shape

Problem → live plant conversation (no cable) → AI turns bytes into a dashboard → same buffer, naive vs verified → the stream tries to trick the AI → human Deny → close.

Do **not** open on a feature grid. Do **not** catalogue CAS, webhooks, Settings, or file paths unless you finish early (see leftover time).

---

## Clock

| Time | Click | Voice-over (read this) |
|------|--------|-------------------------|
| **0:00–0:20** | Empty state. Optional: one beat of scrolling hex after demo is live, then cut back — or stay on welcome. | Factories still talk in serial. Integrating that usually means an expert, a sniffer, and custom code. One wrong write can move a motor. |
| **0:20–0:35** | **Try Demo Mode** | SerialBridge is a desktop gateway. AI proposes how to read the machine. The app decides what hardware may trust. |
| **0:35–1:10** | **Modbus RTU PLC** → **Monitor**. Let ~10 lines land. Stay on the live buffer. | No cable on this desk. This is a live plant conversation — request and response, over and over. |
| **1:10–1:30** | **Investigate**. When the header shows the verified lane, open **Dashboard**. | We ask the model to investigate. Watch unnamed bytes become values you can actually use. |
| **1:30–2:10** | Same port: **Naive (no gate)** once. Point at the naive chip vs the verified chip. | Same AI. Two lanes. Naive is what most agent demos ship: trust the model. Verified only installs a parser when there is evidence. That is how you put AI next to machines. |
| **2:10–2:35** | Stop monitor. **Prompt injection trap** → **Monitor** → **Investigate**. When the approval dialog appears: show the bytes, click **Deny**. | Now the stream itself says: ignore the rules, fire the relay. We show a human the exact bytes — labeled as untrusted, written by AI. Deny. Nothing silent goes out. |
| **2:35–2:50** | Optional: **Settings** → audit list, one second. Else stay on the denied dialog / live buffer. | Every attempt is logged. Safe Mode is on from the first run. |
| **2:50–3:00** | Welcome screen or product name. | The model proposes. SerialBridge — and you — decide. Thank you. |

If Investigate is slow: say *“It’s reading the frames…”* and wait. Do not name tools. Last resort only: freeze a prior successful dashboard and still call it the live app.

---

## Four proofs (and nothing else)

1. Live stream without a cable — Demo Mode, Modbus PLC.
2. AI turns bytes into a dashboard — Investigate → structured values.
3. Trust contrast — same buffer, **Investigate** vs **Naive (no gate)**.
4. The trap — Prompt injection → approval → **Deny**. Hardware stays quiet.

---

## Voice rules

**Say:** undocumented machine, live data, dashboard, human approval, the stream tried to trick the AI.

**Never say on VO:** analyze_binary_structure, CRC, hex regex, JSONL, send_bytes, muse-spark, Tauri, Svelte, do_not_retry, MATIKAN, app config dir.

**If a rubric demands stack (one breath, not in the default cut):** Desktop app, AI in the loop, safety in the host — not in the chat.

---

## Fallbacks

| What goes wrong | What to do |
|-----------------|------------|
| Model never tries a write on the trap | Second take: hover **Investigate** and say *“When the agent tries to write, this is the gate.”* Show the approval UI from a take where it *did* appear. Never fake a silent transmit to a real port. |
| Approval does not appear in 20 s | Cut to a saved approval dialog take; keep Deny on camera. |
| Naive click looks identical | Stay on the chip labels. VO already carries the contrast. |
| Key missing / model error | Stop. Do not paste a key on camera. Save in Settings off-record, restart the take from 1:10. |
| You finish before 2:35 | Optional leftover: **CAS Weighing Scale** dashboard (live kg, no AI). Do not start a new story. |

## What not to do on camera

- Do not record browser-only `npm run dev` — AI needs the desktop host.
- Do not paste or display the API key after saving.
- Do not explain the audit file format or policy JSON.
- Do not click Command Sender or send bytes yourself in this cut.
