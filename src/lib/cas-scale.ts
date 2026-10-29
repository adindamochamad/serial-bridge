import type { DiscoveredDevice } from './types.js';
import type { DeviceProfile } from './profiles.js';
import type { WebhookConfig } from './webhook.js';
import { URL_WEBHOOK_DEV, URL_WEBHOOK_ONLINE } from './cas-default-config.js';

export { URL_WEBHOOK_DEV, URL_WEBHOOK_ONLINE };

/** First baud used to wake a CAS scale before normal operation. */
export const CAS_WARMUP_BAUD = 19200;

/** Operating baud for a CAS scale after warmup. */
export const CAS_OPERATING_BAUD = 9600;

/** How long the port stays open at the warmup baud (ms). */
export const CAS_WARMUP_DURATION_MS = 800;

/** URL to send: explicit url, else legacy url_online when mode is ONLINE. */
export function effectiveWebhookUrl(
  config: Pick<WebhookConfig, 'mode_webhook' | 'url' | 'url_online'>
): string {
  const url = config.url?.trim();
  if (url) return url;
  if (config.mode_webhook === 'ONLINE') {
    const urlOnline = config.url_online?.trim();
    return urlOnline || URL_WEBHOOK_ONLINE;
  }
  return URL_WEBHOOK_DEV;
}

/** Ports already warmed up in this session — skip repeating on stop/start. */
const warmedUpPorts = new Set<string>();

/** WCH VID — USB-serial adapter commonly used with CAS scales (CH340/CH341). */
const VID_CH340 = 0x1a86;

/** True when this port is a CH340/CH341 adapter. */
export function isCh340Adapter(device: DiscoveredDevice): boolean {
  if (device.device_class === 'ch340') return true;
  if (device.vid === VID_CH340) return true;
  const manuf = (device.manufacturer ?? '').toLowerCase();
  const product = (device.product ?? '').toLowerCase();
  return (
    manuf.includes('wch') ||
    product.includes('ch340') ||
    product.includes('ch341') ||
    product.includes('usb-serial')
  );
}

export function isCasDevice(
  device: DiscoveredDevice,
  profile?: DeviceProfile | null
): boolean {
  if (device.port === 'demo://cas-scale') return true;

  const manuf = (device.manufacturer ?? '').toLowerCase();
  const product = (device.product ?? '').toLowerCase();
  const protocol = (profile?.protocol ?? '').toLowerCase();
  const hint = (profile?.device_hint ?? '').toLowerCase();

  if (/\bcas\b/.test(manuf) || manuf.includes('cas corp')) return true;
  if (/\bcl[\s-]?\d/.test(product)) return true;
  if (isCh340Adapter(device)) return true;
  if (protocol.includes('cas scale') || protocol.includes('cas ')) return true;
  if (hint.includes('cas') && hint.includes('scale')) return true;
  if (profile?.needs_baud_warmup === true && protocol.includes('cas')) return true;

  return false;
}

export function needsCasWarmup(
  port: string,
  device: DiscoveredDevice,
  profile?: DeviceProfile | null
): boolean {
  if (port.startsWith('demo://')) return false;
  if (warmedUpPorts.has(port)) return false;
  if (profile?.needs_baud_warmup === false) return false;
  return isCasDevice(device, profile);
}

/** Mark the port as warmed up so it is not repeated in this session. */
export function markCasWarmupDone(port: string): void {
  warmedUpPorts.add(port);
}

/** Reset warmup state (tests). */
export function resetCasWarmup(): void {
  warmedUpPorts.clear();
}

export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
