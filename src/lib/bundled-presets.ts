import casScalePreset from '../../presets/cas-scale.json';
import { isCh340Adapter } from './cas-scale.js';
import { getAllProfiles, type DeviceProfile } from './profiles.js';
import { saveConfig, getConfig, defaultConfig, type WebhookConfig } from './webhook.js';
import { deviceFingerprint, type DiscoveredDevice } from './types.js';
import type { CommandPreset } from './ai-analysis.js';

interface BundledPreset {
  id: string;
  name: string;
  description: string;
  match: {
    manufacturer_contains?: string[];
    product_contains?: string[];
    protocol_contains?: string[];
  };
  profile: {
    protocol: string;
    confidence: number;
    device_hint: string;
    suggested_baud: number;
    needs_baud_warmup?: boolean;
    notes: string;
    expected_fields: DeviceProfile['expected_fields'];
    command_presets?: CommandPreset[];
  };
  webhook?: Partial<WebhookConfig>;
}

const BUNDLED_PRESETS: BundledPreset[] = [casScalePreset as BundledPreset];

function presetMatchesDevice(device: DiscoveredDevice, preset: BundledPreset): boolean {
  const manuf = (device.manufacturer ?? '').toLowerCase();
  const product = (device.product ?? '').toLowerCase();

  const matchManuf = preset.match.manufacturer_contains?.some((k) =>
    manuf.includes(k.toLowerCase())
  );
  const matchProduct = preset.match.product_contains?.some((k) =>
    product.includes(k.toLowerCase())
  );

  return !!(matchManuf || matchProduct || isCh340Adapter(device));
}

/** Apply a bundled preset to local store if this device has no profile yet. */
export async function applyBundledPreset(
  device: DiscoveredDevice
): Promise<{
  profile: DeviceProfile | null;
  webhook: WebhookConfig | null;
  profileCreated: boolean;
  webhookCreated: boolean;
}> {
  const preset = BUNDLED_PRESETS.find((p) => presetMatchesDevice(device, p));
  if (!preset) {
    return { profile: null, webhook: null, profileCreated: false, webhookCreated: false };
  }

  const fp = deviceFingerprint(device);
  const existingProfile = (await getAllProfiles())[fp];
  const existingWebhook = await getConfig(fp);

  let profile: DeviceProfile | null = existingProfile ?? null;
  let webhook: WebhookConfig | null = existingWebhook ?? null;
  let profileCreated = false;
  let webhookCreated = false;

  if (!existingProfile) {
    profileCreated = true;
    const now = Date.now();
    const data = preset.profile;
    profile = {
      fingerprint: fp,
      vid: device.vid,
      pid: device.pid,
      serial_number: device.serial_number,
      manufacturer: device.manufacturer,
      product: device.product,
      device_class: device.device_class,
      suggested_baud: data.suggested_baud,
      needs_baud_warmup: data.needs_baud_warmup ?? true,
      protocol: data.protocol,
      confidence: data.confidence,
      device_hint: data.device_hint,
      expected_fields: data.expected_fields,
      command_presets: data.command_presets ?? [],
      notes: data.notes,
      first_learned_at: now,
      last_seen_at: now,
      sample_count: 1,
      high_water_confidence: data.confidence,
      source: 'bundled',
      sumber: 'bundled',
    };
    const all = await getAllProfiles();
    const { Store } = await import('@tauri-apps/plugin-store');
    const store = await Store.load('serialbridge.profiles.json');
    all[fp] = profile;
    await store.set('profiles', all);
    await store.save();
  }

  if (!existingWebhook && preset.webhook) {
    webhookCreated = true;
    webhook = { ...defaultConfig(), ...preset.webhook };
    await saveConfig(fp, webhook);
  }

  return { profile, webhook, profileCreated, webhookCreated };
}

/** Apply every matching bundled preset for the discovered device list. */
export async function applyAllBundledPresets(
  devices: DiscoveredDevice[]
): Promise<{ profilesCreated: number; webhooksCreated: number }> {
  let profilesCreated = 0;
  let webhooksCreated = 0;
  for (const d of devices) {
    const result = await applyBundledPreset(d);
    if (result.profileCreated) profilesCreated++;
    if (result.webhookCreated) webhooksCreated++;
  }
  return { profilesCreated, webhooksCreated };
}

/** Bundled presets shipped in the repo (for UI copy). */
export function listBundledPresets(): Array<{ id: string; name: string; description: string }> {
  return BUNDLED_PRESETS.map((p) => ({
    id: p.id,
    name: p.name,
    description: p.description,
  }));
}
