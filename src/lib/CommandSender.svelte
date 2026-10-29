<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  export interface CommandPreset {
    name: string;
    description: string;
    hex?: string;
    ascii?: string;
  }

  let {
    port,
    presets = [],
    onClose,
  }: {
    port: string;
    presets?: CommandPreset[];
    onClose: () => void;
  } = $props();

  let mode = $state<'ascii' | 'hex'>('ascii');
  let input = $state('');
  let appendCR = $state(true);
  let appendLF = $state(true);
  let sending = $state(false);
  let status = $state<{ ok: boolean; text: string } | null>(null);
  let history = $state<string[]>([]);
  let sendConfirmPending = $state(false);
  let sendConfirmTimer: ReturnType<typeof setTimeout> | null = null;

  function parseAsciiToBytes(text: string): number[] {
    // Handle escape sequences: \r \n \t \\ \xNN
    const bytes: number[] = [];
    for (let i = 0; i < text.length; i++) {
      const ch = text[i];
      if (ch === '\\' && i + 1 < text.length) {
        const next = text[i + 1];
        if (next === 'r') { bytes.push(0x0d); i++; continue; }
        if (next === 'n') { bytes.push(0x0a); i++; continue; }
        if (next === 't') { bytes.push(0x09); i++; continue; }
        if (next === '\\') { bytes.push(0x5c); i++; continue; }
        if (next === 'x' && i + 3 < text.length) {
          const hex = text.slice(i + 2, i + 4);
          const code = parseInt(hex, 16);
          if (!isNaN(code)) { bytes.push(code); i += 3; continue; }
        }
      }
      bytes.push(ch.charCodeAt(0) & 0xff);
    }
    if (appendCR) bytes.push(0x0d);
    if (appendLF) bytes.push(0x0a);
    return bytes;
  }

  function parseHexToBytes(text: string): number[] | null {
    const cleaned = text.replace(/[\s,_-]/g, '').replace(/0x/gi, '');
    if (cleaned.length === 0) return null;
    if (cleaned.length % 2 !== 0) return null;
    if (!/^[0-9a-fA-F]+$/.test(cleaned)) return null;
    const bytes: number[] = [];
    for (let i = 0; i < cleaned.length; i += 2) {
      bytes.push(parseInt(cleaned.slice(i, i + 2), 16));
    }
    return bytes;
  }

  function clearSendConfirm() {
    sendConfirmPending = false;
    if (sendConfirmTimer) {
      clearTimeout(sendConfirmTimer);
      sendConfirmTimer = null;
    }
  }

  function armSendConfirm(): boolean {
    if (!sendConfirmPending) {
      sendConfirmPending = true;
      status = { ok: true, text: 'Click Send again within 4s to transmit to the device' };
      if (sendConfirmTimer) clearTimeout(sendConfirmTimer);
      sendConfirmTimer = setTimeout(clearSendConfirm, 4000);
      return false;
    }
    clearSendConfirm();
    return true;
  }

  async function send() {
    if (!input.trim()) return;
    if (!armSendConfirm()) return;
    sending = true;
    status = null;

    let bytes: number[] | null;
    if (mode === 'ascii') {
      bytes = parseAsciiToBytes(input);
    } else {
      bytes = parseHexToBytes(input);
      if (bytes === null) {
        status = { ok: false, text: 'Invalid hex — use pairs like "1B 54 0D" or "1B540D"' };
        sending = false;
        return;
      }
    }

    try {
      await invoke('send_bytes_to_port', { port, data: bytes });
      status = { ok: true, text: `Sent ${bytes.length} byte${bytes.length !== 1 ? 's' : ''}` };
      // Add to history (dedupe, max 10)
      history = [input, ...history.filter((h) => h !== input)].slice(0, 10);
    } catch (e) {
      status = { ok: false, text: String(e) };
    } finally {
      sending = false;
    }
  }

  function sendPreset(preset: CommandPreset) {
    if (preset.hex) {
      mode = 'hex';
      input = preset.hex;
    } else if (preset.ascii) {
      mode = 'ascii';
      input = preset.ascii;
    }
    send();
  }

  function useHistory(h: string) {
    input = h;
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  }
</script>

<div class="sender">
  <div class="sender-header">
    <span class="tx-icon">→</span>
    <span class="sender-title">Send to device</span>
    <div class="mode-toggle">
      <button
        class="mode-btn"
        class:active={mode === 'ascii'}
        onclick={() => (mode = 'ascii')}
      >
        ASCII
      </button>
      <button
        class="mode-btn"
        class:active={mode === 'hex'}
        onclick={() => (mode = 'hex')}
      >
        HEX
      </button>
    </div>
    <button class="close-x" onclick={onClose} title="Hide sender">✕</button>
  </div>

  <div class="input-row">
    <input
      type="text"
      class="cmd-input"
      placeholder={mode === 'ascii'
        ? 'Type command (\\r \\n \\xNN supported)'
        : 'Hex bytes: 1B 54 0D or 1B540D'}
      bind:value={input}
      onkeydown={onKey}
      disabled={sending}
      autocomplete="off"
      spellcheck="false"
    />
    <button
      class="send-btn"
      class:pending-confirm={sendConfirmPending}
      onclick={send}
      disabled={sending || !input.trim()}
    >
      {#if sending}◌{:else if sendConfirmPending}Confirm ↵{:else}Send ↵{/if}
    </button>
  </div>

  {#if mode === 'ascii'}
    <div class="options-row">
      <label class="opt-toggle">
        <input type="checkbox" bind:checked={appendCR} />
        <span>\r</span>
      </label>
      <label class="opt-toggle">
        <input type="checkbox" bind:checked={appendLF} />
        <span>\n</span>
      </label>
      <span class="opt-hint">appended</span>
    </div>
  {/if}

  {#if status}
    <div class="status" class:fail={!status.ok}>{status.text}</div>
  {/if}

  {#if presets.length > 0}
    <div class="preset-row">
      <span class="preset-label">Presets:</span>
      {#each presets as preset}
        <button
          class="preset-chip"
          onclick={() => sendPreset(preset)}
          title={preset.description}
        >
          {preset.name}
        </button>
      {/each}
    </div>
  {/if}

  {#if history.length > 0}
    <div class="history-row">
      <span class="preset-label">Recent:</span>
      {#each history.slice(0, 5) as h}
        <button class="history-chip" onclick={() => useHistory(h)} title={h}>
          {h.length > 22 ? h.slice(0, 22) + '…' : h}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .sender {
    background: var(--color-stream-bg);
    border-top: 1px solid #1e293b;
    padding: 0.5rem 0.7rem;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    flex-shrink: 0;
  }

  .sender-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .tx-icon {
    color: #f59e0b;
    font-weight: 700;
    font-size: 1rem;
  }

  .sender-title {
    font-size: 0.72rem;
    color: #8090a8;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    flex: 1;
  }

  .mode-toggle {
    display: flex;
    gap: 2px;
    background: #141824;
    border-radius: 5px;
    padding: 2px;
  }

  .mode-btn {
    padding: 0.15rem 0.5rem;
    background: transparent;
    border: none;
    color: #5a6a8a;
    font-size: 0.65rem;
    font-weight: 600;
    cursor: pointer;
    border-radius: 3px;
    letter-spacing: 0.04em;
    transition: all 0.12s;
  }

  .mode-btn.active {
    background: #1e2a42;
    color: #c0d0ff;
  }

  .close-x {
    background: none;
    border: none;
    color: #3a3a5a;
    cursor: pointer;
    font-size: 0.7rem;
    padding: 0.1rem 0.3rem;
  }

  .close-x:hover { color: #f87171; }

  .input-row {
    display: flex;
    gap: 0.4rem;
  }

  .cmd-input {
    flex: 1;
    background: #0a0e1a;
    border: 1px solid #2a3450;
    border-radius: 5px;
    color: #e0e8ff;
    font-size: 0.8rem;
    padding: 0.4rem 0.55rem;
    font-family: "JetBrains Mono", monospace;
    outline: none;
    transition: border-color 0.15s;
  }

  .cmd-input:focus {
    border-color: #f59e0b;
  }

  .send-btn.pending-confirm {
    border-color: #f59e0b;
    color: #fbbf24;
  }

  .send-btn {
    padding: 0.4rem 0.8rem;
    background: #f59e0b;
    border: none;
    border-radius: 5px;
    color: #1a0e00;
    font-size: 0.75rem;
    font-weight: 700;
    cursor: pointer;
    transition: background 0.15s;
    white-space: nowrap;
  }

  .send-btn:hover:not(:disabled) {
    background: #d88906;
  }

  .send-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .options-row {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    font-size: 0.7rem;
    color: #7a8aa8;
  }

  .opt-toggle {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    cursor: pointer;
  }

  .opt-toggle input[type='checkbox'] {
    accent-color: #f59e0b;
    width: 12px;
    height: 12px;
  }

  .opt-toggle span {
    font-family: "JetBrains Mono", monospace;
    color: #b0c0d8;
  }

  .opt-hint {
    font-size: 0.64rem;
    color: #4a5a7a;
    margin-left: auto;
  }

  .status {
    padding: 0.25rem 0.5rem;
    background: #0e2018;
    border: 1px solid #1e4028;
    color: #4ade80;
    border-radius: 4px;
    font-size: 0.7rem;
    font-family: "JetBrains Mono", monospace;
  }

  .status.fail {
    background: #1e0e0e;
    border-color: #4a1e1e;
    color: #f87171;
  }

  .preset-row,
  .history-row {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    flex-wrap: wrap;
  }

  .preset-label {
    font-size: 0.62rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: #4a5a7a;
    font-weight: 600;
  }

  .preset-chip,
  .history-chip {
    padding: 0.18rem 0.55rem;
    font-size: 0.7rem;
    border-radius: 4px;
    cursor: pointer;
    border: 1px solid;
    font-family: "JetBrains Mono", monospace;
    transition: all 0.15s;
  }

  .preset-chip {
    background: #201608;
    border-color: #402e14;
    color: #fbbf24;
  }

  .preset-chip:hover {
    background: #2a1e0a;
    border-color: #6a4e20;
    color: #fcd34d;
  }

  .history-chip {
    background: #141828;
    border-color: #2a3040;
    color: #8090a8;
  }

  .history-chip:hover {
    background: #1a2030;
    color: #a0b0d0;
  }
</style>
