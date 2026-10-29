import { describe, expect, it } from 'vitest';
import {
  META_MODEL_API_BASE,
  META_MODEL_SPARK,
  metaKeyClass,
  usableMetaKey,
} from '../src/lib/meta-api.js';

describe('Meta Model API helpers', () => {
  it('pins model id and API origin for documentation', () => {
    expect(META_MODEL_API_BASE).toBe('https://api.meta.ai');
    expect(META_MODEL_SPARK).toBe('muse-spark-1.3');
    expect(META_MODEL_SPARK).not.toContain('contributor');
  });

  it('refuses non-Meta keys and accepts a Meta Standard key', () => {
    expect(metaKeyClass('sk-ant-example')).toBe('unsupported');
    expect(usableMetaKey('sk-ant-example')).toBeNull();
    expect(metaKeyClass('LLM_example_key')).toBe('meta');
    expect(usableMetaKey('  LLM_example_key  ')).toBe('LLM_example_key');
    expect(usableMetaKey('')).toBeNull();
  });
});
