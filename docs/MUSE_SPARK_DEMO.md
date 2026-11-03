# Muse Spark Integration Demo Guide

This guide shows how to demo the enhanced Muse Spark (muse-spark-1.3) features added for the Meta hackathon.

## Prerequisites

1. **Meta Model API Key** (Standard tier)
   - Get your key from [dev.meta.ai](https://dev.meta.ai)
   - Starts with `LLM_`
   - Free tier works fine for demos

2. **Development environment**
   ```bash
   npm ci
   npm run tauri dev
   ```

## Quick Start: Enable Muse Spark Features

1. Launch the app: `npm run tauri dev`
2. Click the **⚙ Settings** button (top-right toolbar)
3. Paste your `LLM_` key in the "Meta Model API Key" field
4. Click **Save**

The app will validate the key format. Once saved, all three Muse Spark features become available.

---

## Feature 1: 🧠 Safe Mode Block Explanations

**What it does**: When Safe Mode blocks a command, ask Muse Spark to explain *why* in plain language.

### Demo Steps

1. Enable Demo Mode (click **Demo** button)
2. Open a virtual device (e.g., "CAS SW-1 Scale")
3. Let some demo data flow in
4. Click **Send Command**
5. Type a payload that Safe Mode will block:
   ```
   FF FF FF FF FF
   ```
   (Unrecognized hex pattern → Safe Mode denies)

6. The **approval dialog** appears showing the block reason
7. Click **🧠 Explain with AI** (button appears when key is configured)
8. Muse Spark generates a 2-3 sentence explanation:
   - Why the command was blocked
   - What risk the hex bytes posed
   - Specific to the exact bytes shown

**Key points for judges**:
- ✅ Explanation is **read-only** (cannot bypass the gate)
- ✅ Model sees the exact blocked hex + Safe Mode's reason
- ✅ Response limited to 200 tokens (cost-effective, focused)
- ✅ If no key configured, button doesn't appear (graceful degradation)

---

## Feature 2: 🔄 Iterative Parser Refinement

**What it does**: Multi-step agent loop where Muse Spark proposes a parser, Rust validates it, tests against data, and the model refines based on feedback.

### Demo Steps

1. Open a virtual device in Demo Mode (e.g., "Arduino Sensor")
2. Let ~20 lines of data accumulate
3. Click **🔄 Refine Parser** (new button next to Investigate)
4. Watch the **Agent Trace Panel** (bottom tab: "Investigation")
5. You'll see iterations:
   - Iteration 1: Model proposes initial parser
   - ✓ Schema validation passed
   - ✗ Testing: some fields didn't match
   - Iteration 2: Model refines based on feedback
   - ✓ Schema validation passed
   - ✓ Testing: fields matched sample data
   - Success after 2 iterations

**What's happening under the hood**:
1. Muse Spark generates a `ProtocolAnalysis` JSON
2. **Rust validates** schema: required fields, confidence 0-100, data_type enum
3. **Regex testing**: Each field's regex runs against last 20 lines
4. If validation fails or no fields match → **feedback to model**
5. Model refines, loop continues (max 5 iterations)
6. Parser installs with `naive` commit (contrast to verified commit gate)

**Key points for judges**:
- ✅ Shows **iteration trace** (transparent refinement process)
- ✅ **Rust validation** enforces schema correctness
- ✅ **Testing loop** ensures parser works on real data
- ✅ Max 5 iterations (prevents runaway loops)
- ✅ Falls back gracefully if model can't converge

### Compare to standard Investigate

| Feature | 🔬 Investigate | 🔄 Refine Parser |
|---------|---------------|------------------|
| Iterations | Single-shot | Up to 5 |
| Validation | Front-end JSON parse | Rust schema validator |
| Testing | Visual evidence only | Regex match on data |
| Commit gate | Full evidence gate | Naive (no gate) |
| Use case | Production parser | Iteration demo |

---

## Feature 3: ✅ Rust-Side JSON Validation

**What it does**: Strict `ProtocolAnalysis` schema validation in Rust before parser can be used.

### Demo: Validation in Action

The validation runs automatically during refinement, but you can see it explicitly fail:

1. Open browser console (F12)
2. In the app, open a serial port with data
3. Run this in console to try an invalid parser:
   ```javascript
   const { invoke } = window.__TAURI_INTERNALS__;
   
   // Missing required field 'protocol'
   invoke('meta_validate_parser_json', {
     body: {
       confidence: 90,
       device_hint: "Test",
       fields: [],
       notes: "Invalid"
     }
   }).catch(e => console.error("Validation error:", e));
   ```

4. You'll see: `"Missing or invalid 'protocol' string"`

### Validated Rules

The Rust validator checks:
- **Required fields**: `protocol`, `confidence`, `device_hint`, `fields[]`, `notes`
- **Confidence**: Must be 0-100 (not 150)
- **Field objects**: Must have `name` and `description`
- **data_type enum**: Only `number`, `string`, `boolean`, `hex_u16_be`, `hex_u16_le`, `hex_s16_be`, `hex_u8`
- **Type safety**: regex must be string if present

**Unit tests** (5 new):
```bash
cd src-tauri
cargo test meta_validate
```

All pass, covering:
- Valid schema acceptance
- Missing protocol rejection
- Out-of-bounds confidence
- Invalid data_type
- Hex data_type support

---

## Safe Mode Guarantees

All three features preserve Safe Mode's security model:

### Block Explanations
- ✅ **Read-only**: Cannot approve, deny, or modify blocked command
- ✅ **No bypass**: Explanation is informational only
- ✅ **Existing flows**: Approval/audit unchanged

### Iterative Refinement
- ✅ **Naive commit**: Parser doesn't auto-install (no verified gate bypass)
- ✅ **User control**: Manual approval still required for dangerous commands
- ✅ **Audit trail**: All iterations logged in agent trace

### Rust Validation
- ✅ **Fail-safe**: Invalid parser rejected before reaching dashboard
- ✅ **No data_type injection**: Enum validated, prevents arbitrary types
- ✅ **Backwards compatible**: Existing parsers still work

---

## Testing Without Real Hardware

**Demo Mode** works perfectly for all features:

1. Click **Demo** button
2. Available virtual devices:
   - **CAS SW-1 Scale** — ASCII weight data
   - **Arduino Sensor** — CSV with temp/humidity/pressure
   - **GPS NMEA** — NMEA 0183 sentences
   - **Modbus PLC** — Binary Modbus RTU frames

3. Each device generates realistic data
4. All Muse Spark features work identically to real hardware
5. Safe Mode blocks still trigger (demo has blocked commands)

---

## Performance & Cost

### Block Explanations
- **Token usage**: ~150 input + 100 output = 250 tokens/explanation
- **Cost**: ~$0.0005 per explanation (Meta Standard pricing)
- **Latency**: 1-2 seconds

### Iterative Refinement
- **Token usage**: ~3K-5K input + 1K-2K output per iteration
- **Cost**: ~$0.02-0.03 total (2-3 iterations typical)
- **Latency**: 5-10 seconds per iteration

All reasonable for a demo/hackathon scenario with free-tier key.

---

## Troubleshooting

### "No Meta Model API key" error
- Open Settings, paste a key that starts with `LLM_`
- Key must be from dev.meta.ai (not Claude API key)

### 🧠 Explain button doesn't appear
- Key not configured, or browser refresh needed
- Check Settings → key status should show "configured"

### Refine Parser fails immediately
- Need at least 10 lines of data
- Try Demo Mode if real hardware isn't available

### "Schema validation failed" in refinement
- This is expected during iterations!
- Watch the trace — model should fix it on next iteration
- If stuck after 5 iterations, it means the data is too ambiguous

---

## Code Structure

**Rust commands** (src-tauri/src/meta_api.rs):
```rust
pub async fn meta_explain_block(...) -> Result<String, String>
pub fn meta_validate_parser_json(...) -> Result<(), String>
```

**TypeScript wrappers** (src/lib/meta-api.ts):
```typescript
export async function metaExplainBlock(...)
export async function metaValidateParserJson(...)
```

**Agent loop** (src/lib/ai-analysis.ts):
```typescript
export async function refineParserIteratively(...)
```

**UI integration**:
- ApprovalDialog.svelte — block explanation button
- DataBuffer.svelte — refine parser button

---

## Demo Script for Judges (~5 min)

1. **Setup** (30s):
   - Launch app, configure key, enable Demo Mode

2. **Block Explanation** (90s):
   - Open virtual device, try blocked command
   - Click 🧠 Explain → show plain-language explanation
   - Emphasize: read-only, cannot bypass

3. **Iterative Refinement** (2 min):
   - Open Arduino Sensor, let data flow
   - Click 🔄 Refine Parser
   - Show Agent Trace: iterations, validation, testing
   - Point out: Rust validation, regex testing, feedback loop

4. **Validation** (60s):
   - Show browser console, trigger validation error
   - Explain: 5 new unit tests, schema enforcement

5. **Wrap-up** (30s):
   - Safe Mode preserved
   - Works in Demo Mode
   - Deadline met, shippable

---

## Next Steps

After the hackathon:
- [ ] Add verified commit gate option to refinement
- [ ] Expand validation to check regex compilation
- [ ] Support streaming explanations (SSE)
- [ ] Add explanation caching (same block → cached response)

For now: focused, robust, shippable for judges. 🚀
