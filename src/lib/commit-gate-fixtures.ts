import type { CommandPreset } from './ai-analysis.js';
import { DEMO_SCENARIOS } from './demo.js';
import type { DataLine } from './types.js';

function parseAsciiPresetBytes(text: string): number[] {
  const bytes: number[] = [];
  for (let i = 0; i < text.length; i++) {
    const ch = text[i];
    if (ch === '\\' && i + 1 < text.length) {
      const next = text[i + 1];
      if (next === 'r') {
        bytes.push(0x0d);
        i++;
        continue;
      }
      if (next === 'n') {
        bytes.push(0x0a);
        i++;
        continue;
      }
      if (next === 't') {
        bytes.push(0x09);
        i++;
        continue;
      }
      if (next === '\\') {
        bytes.push(0x5c);
        i++;
        continue;
      }
      if (next === 'x' && i + 3 < text.length) {
        const hex = text.slice(i + 2, i + 4);
        const code = Number.parseInt(hex, 16);
        if (!Number.isNaN(code)) {
          bytes.push(code);
          i += 3;
          continue;
        }
      }
    }
    bytes.push(ch.charCodeAt(0) & 0xff);
  }
  return bytes;
}

function parseHexPresetBytes(text: string): number[] | null {
  const cleaned = text.replace(/[\s,_-]/g, '').replace(/0x/gi, '');
  if (cleaned.length === 0 || cleaned.length % 2 !== 0 || !/^[0-9a-fA-F]+$/.test(cleaned)) {
    return null;
  }
  const bytes: number[] = [];
  for (let i = 0; i < cleaned.length; i += 2) {
    bytes.push(Number.parseInt(cleaned.slice(i, i + 2), 16));
  }
  return bytes;
}

export function bytesToSpacedHex(bytes: number[]): string {
  return bytes.map((b) => b.toString(16).padStart(2, '0').toUpperCase()).join(' ');
}

export type EncodePresetResult =
  | { ok: true; bytes: number[] }
  | { ok: false; reason: string };

/** Decode the first sendable payload from a command preset command preset. */
export function encodePresetPayload(preset: CommandPreset): EncodePresetResult {
  if (preset.hex?.trim()) {
    const bytes = parseHexPresetBytes(preset.hex);
    if (!bytes || bytes.length === 0) {
      return { ok: false, reason: 'Invalid hex in preset.' };
    }
    return { ok: true, bytes };
  }
  if (preset.ascii?.trim()) {
    const bytes = parseAsciiPresetBytes(preset.ascii);
    if (bytes.length === 0) {
      return { ok: false, reason: 'Empty ascii preset.' };
    }
    return { ok: true, bytes };
  }
  return { ok: false, reason: 'Preset has no hex or ascii payload.' };
}

export function pickFirstEncodablePreset(presets: CommandPreset[] | undefined): CommandPreset | null {
  if (!presets?.length) return null;
  for (const preset of presets) {
    const encoded = encodePresetPayload(preset);
    if (encoded.ok) return preset;
  }
  return null;
}

export interface VirtualTransmitOutcome {
  accepted: boolean;
  bytes: number[];
  reason: string;
  hex: string | null;
}

/** Virtual PLC sink: install presets as real bytes only when the lane committed. */
export function attemptVirtualTransmit(input: {
  committed: boolean | undefined;
  preset: CommandPreset | null;
  commitReason?: string;
}): VirtualTransmitOutcome {
  if (input.committed !== true) {
    return {
      accepted: false,
      bytes: [],
      hex: null,
      reason: input.commitReason?.trim() || 'Commit gate withheld installation.',
    };
  }
  if (!input.preset) {
    return {
      accepted: false,
      bytes: [],
      hex: null,
      reason: 'No command preset to transmit.',
    };
  }
  const encoded = encodePresetPayload(input.preset);
  if (!encoded.ok) {
    return { accepted: false, bytes: [], hex: null, reason: encoded.reason };
  }
  return {
    accepted: true,
    bytes: encoded.bytes,
    hex: bytesToSpacedHex(encoded.bytes),
    reason: `SENT ${encoded.bytes.length} byte(s).`,
  };
}

function modbusCrc16(data: number[]): number {
  let crc = 0xffff;
  for (const byte of data) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit++) {
      if (crc & 1) crc = (crc >> 1) ^ 0xa001;
      else crc >>= 1;
    }
  }
  return crc & 0xffff;
}

/** Per-line Modbus CRC16. Each demo line is one frame. */
export function bufferModbusCrcRatio(lines: Array<{ hex: string }>): {
  frames: number;
  valid: number;
  ratio: number;
} {
  let frames = 0;
  let valid = 0;
  for (const line of lines) {
    const bytes = line.hex
      .split(/\s+/)
      .filter(Boolean)
      .map((part) => Number.parseInt(part, 16));
    if (bytes.length < 4 || bytes.some((byte) => Number.isNaN(byte) || byte > 0xff)) continue;
    frames++;
    const crcLo = bytes[bytes.length - 2];
    const crcHi = bytes[bytes.length - 1];
    const expected = (crcHi << 8) | crcLo;
    if (modbusCrc16(bytes.slice(0, -2)) === expected) valid++;
  }
  const ratio = frames > 0 ? Math.round((valid / frames) * 100) / 100 : 0;
  return { frames, valid, ratio };
}

/** Flip CRC bytes so analyze_binary_structure reports a Modbus miss. */
export function breakTrailingCrc(hex: string): string {
  const parts = hex.split(/\s+/).filter(Boolean);
  if (parts.length < 4) return hex;
  const last = parts.length - 1;
  parts[last] = parts[last] === '00' ? 'FF' : '00';
  parts[last - 1] = parts[last - 1] === '00' ? 'FF' : '00';
  return parts.join(' ');
}

export interface ModbusMachineReading {
  temperatureC: number;
  pressureHpa: number;
  flowLMin: number;
  motorRpm: number;
  valvePercent: number;
  alarm: number;
}

/** Engineering units from one Modbus read-holding-registers frame. */
export function readModbusMachine(hex: string): ModbusMachineReading | null {
  const bytes = hex
    .split(/\s+/)
    .filter(Boolean)
    .map((part) => Number.parseInt(part, 16));
  if (bytes.length < 15 || bytes[0] !== 0x01 || bytes[1] !== 0x03) return null;
  const count = bytes[2];
  if (!Number.isInteger(count) || count < 12 || bytes.length < 3 + 12) return null;
  const register = (index: number) => (bytes[3 + index * 2] << 8) | bytes[3 + index * 2 + 1];
  return {
    temperatureC: register(0) / 10,
    pressureHpa: register(1),
    flowLMin: register(2),
    motorRpm: register(3),
    valvePercent: register(4),
    alarm: register(5),
  };
}

/** PLC numbers appear only after the verified lane commits on a strong CRC. */
export function faceplateValuesVisible(input: {
  verifiedCommitted: boolean | null | undefined;
  crcRatio: number | null;
}): boolean {
  return input.verifiedCommitted === true && input.crcRatio !== null && input.crcRatio >= 0.9;
}

/** After a hold, clearing Break CRC re-runs both lanes on the intact buffer. */
export function shouldProveGateOpens(input: {
  previousBreakCrc: boolean;
  nextBreakCrc: boolean;
  scenario: 'cas-scale' | 'modbus-plc';
  verifiedCommitted: boolean | null | undefined;
  running: boolean;
}): boolean {
  return (
    input.previousBreakCrc &&
    !input.nextBreakCrc &&
    input.scenario === 'modbus-plc' &&
    input.verifiedCommitted === false &&
    !input.running
  );
}

/** Build buffer lines from a built-in device scenario. */
export function buildDemoBufferLines(
  scenarioId: 'cas-scale' | 'modbus-plc',
  lineCount = 24,
  options?: { breakCrc?: boolean }
): DataLine[] {
  const scenario = DEMO_SCENARIOS.find((s) => s.id === scenarioId);
  if (!scenario) return [];

  const lines: DataLine[] = [];
  let seq = 0;
  for (let i = 0; i < lineCount; i++) {
    const bytes = scenario.generate();
    let hex = Array.from(bytes)
      .map((b) => b.toString(16).padStart(2, '0').toUpperCase())
      .join(' ');
    if (options?.breakCrc && scenarioId === 'modbus-plc') {
      hex = breakTrailingCrc(hex);
    }
    const shown = hex.split(/\s+/).filter(Boolean).map((part) => Number.parseInt(part, 16));
    const ascii = shown
      .map((b) => (b >= 0x20 && b <= 0x7e ? String.fromCharCode(b) : '·'))
      .join('');
    lines.push({
      id: ++seq,
      timestamp: Date.now() + i,
      hex,
      ascii,
      bytes: shown.length,
    });
  }
  return lines;
}
