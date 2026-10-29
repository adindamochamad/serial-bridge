import { invoke } from '@tauri-apps/api/core';
import { Store } from '@tauri-apps/plugin-store';
import { metaKeyClass, usableMetaKey } from './meta-api.js';

export { metaKeyClass, usableMetaKey };

const STORE_FILE = 'serialbridge.settings.json';
const KEY_API_KEY = 'meta_model_api_key';
const KEY_API_KEY_LEGACY = 'anthropic_api_key';

let _migrateDone = false;

async function migrateLegacyApiKeyFromStore(): Promise<void> {
  if (_migrateDone) return;
  _migrateDone = true;
  try {
    const store = await Store.load(STORE_FILE);
    const stored = await store.get<string>(KEY_API_KEY);
    const legacy = await store.get<string>(KEY_API_KEY_LEGACY);
    const key = usableMetaKey(stored) ?? usableMetaKey(legacy);
    if (key) {
      await invoke('meta_set_api_key', { key });
      await store.delete(KEY_API_KEY);
      await store.delete(KEY_API_KEY_LEGACY);
      await store.save();
    }
  } catch {
    // store or invoke unavailable (browser preview)
  }
}

export async function hasApiKey(): Promise<boolean> {
  await migrateLegacyApiKeyFromStore();
  try {
    const status = await invoke<{ configured: boolean }>('meta_key_status');
    return status.configured;
  } catch {
    return false;
  }
}

export async function setApiKey(key: string): Promise<void> {
  const usable = usableMetaKey(key);
  if (!usable) {
    throw new Error(
      'That key is not a Meta Model API key. Paste a Standard key from dev.meta.ai. It starts with LLM_.',
    );
  }
  await invoke('meta_set_api_key', { key: usable });
}

export async function clearApiKey(): Promise<void> {
  await invoke('meta_clear_api_key');
}
