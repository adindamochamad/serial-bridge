import { invoke } from '@tauri-apps/api/core';
import type Anthropic from '@anthropic-ai/sdk';

/** Messages API origin (host proxies; WebView does not call this directly). */
export const META_MODEL_API_BASE = 'https://api.meta.ai';

/** Class only. Never log the key itself. */
export function metaKeyClass(key: string | null | undefined): 'empty' | 'meta' | 'unsupported' | 'other' {
  const trimmed = key?.trim() ?? '';
  if (!trimmed) return 'empty';
  if (trimmed.startsWith('LLM_')) return 'meta';
  if (trimmed.startsWith('sk-ant')) return 'unsupported';
  return 'other';
}

/** Meta Model API accepts Standard keys that start with LLM_. */
export function usableMetaKey(key: string | null | undefined): string | null {
  const trimmed = key?.trim() ?? '';
  return trimmed.startsWith('LLM_') ? trimmed : null;
}

export const META_MODEL_SPARK = 'muse-spark-1.3';

/** Meta Standard list pricing for muse-spark-1.3 (USD per 1M tokens) — UI estimates only. */
export const MUSE_SPARK_PRICING_USD_PER_MTOK = {
  input: 1.25,
  output: 4.25,
  cache_read: 0.15,
  cache_write: 1.25,
};

export const DESKTOP_REQUIRED_MSG =
  'Meta Model API calls require the SerialBridge desktop app (npm run tauri dev). Browser preview cannot reach the Rust host.';

export interface MetaKeyStatus {
  configured: boolean;
  keyClass: string;
}

export async function fetchMetaKeyStatus(): Promise<MetaKeyStatus | null> {
  try {
    const status = await invoke<{ configured: boolean; key_class: string }>('meta_key_status');
    return { configured: status.configured, keyClass: status.key_class };
  } catch {
    return null;
  }
}

/** Create a Messages API response via the Rust host (model forced to muse-spark-1.3). */
export async function metaMessagesCreate(
  body: Record<string, unknown>
): Promise<Anthropic.Messages.Message> {
  return invoke<Anthropic.Messages.Message>('meta_messages_create', { body });
}

export async function metaListModels(): Promise<string[]> {
  return invoke<string[]>('meta_list_models');
}

/** Lightweight auth check — counts input tokens without generating a reply. */
export async function metaCountTokens(
  body: Record<string, unknown>
): Promise<{ input_tokens: number }> {
  return invoke<{ input_tokens: number }>('meta_count_tokens', { body });
}
