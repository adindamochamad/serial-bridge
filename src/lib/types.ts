export interface DiscoveredDevice {
  port: string;
  device_class: string;
  label: string;
  manufacturer: string | null;
  product: string | null;
  vid: number | null;
  pid: number | null;
  serial_number: string | null;
  suggested_baud: number;
}

export type DataDirection = "rx" | "tx";

export interface PortDataEvent {
  port: string;
  data: number[];
  timestamp: number;
  text: string;
  direction?: DataDirection;
}

export interface DataLine {
  id: number;
  timestamp: number;
  hex: string;
  ascii: string;
  bytes: number;
  direction?: DataDirection;
}

export const BAUD_RATES = [1200, 2400, 4800, 9600, 19200, 38400, 57600, 115200] as const;

/** Subset probed by auto-detection (covers ~95% of devices) */
export const BAUDS_TO_PROBE = [4800, 9600, 19200, 38400, 57600, 115200] as const;

export interface BaudProbeResult {
  baud: number;
  score: number; // 0-1 printable ratio
  bytes_read: number;
  printable_count: number;
  sample_ascii: string;
  error: string | null;
}

export const CLASS_COLORS: Record<string, string> = {
  arduino: "#22c55e",
  ftdi_adapter: "#3b82f6",
  ch340: "#a855f7",
  cp210x: "#06b6d4",
  unknown_usb: "#f59e0b",
  pci_serial: "#64748b",
  bluetooth_serial: "#ec4899",
  demo: "#c084fc",
  unknown: "#4b5563",
};

export function toHex(bytes: number[]): string {
  return bytes.map((b) => b.toString(16).padStart(2, "0").toUpperCase()).join(" ");
}

export function toVisibleAscii(bytes: number[]): string {
  return bytes
    .map((b) => {
      if (b === 0x0d) return "↵";
      if (b === 0x0a) return "⏎";
      if (b === 0x09) return "→";
      if (b === 0x00) return "∅";
      if (b < 0x20 || b > 0x7e) return "·";
      return String.fromCharCode(b);
    })
    .join("");
}

export function deviceFingerprint(d: {
  vid: number | null;
  pid: number | null;
  serial_number: string | null;
}): string {
  const vid = d.vid !== null ? d.vid.toString(16).padStart(4, "0") : "0000";
  const pid = d.pid !== null ? d.pid.toString(16).padStart(4, "0") : "0000";
  const base = `${vid}:${pid}`;
  return d.serial_number ? `${base}:${d.serial_number}` : base;
}

export function formatTimestamp(ms: number): string {
  const d = new Date(ms);
  const h = String(d.getHours()).padStart(2, "0");
  const m = String(d.getMinutes()).padStart(2, "0");
  const s = String(d.getSeconds()).padStart(2, "0");
  const ms2 = String(d.getMilliseconds()).padStart(3, "0");
  return `${h}:${m}:${s}.${ms2}`;
}
