import { describe, expect, it } from 'vitest';
import {
  DEMO_SCENARIOS,
  PROMPT_INJECTION_RELAY_HEX,
  decodeDemoChunksAsText,
} from '../src/lib/demo.js';

describe('demo scenarios', () => {
  it('prompt injection trap emits a SYSTEM line with relay hex', () => {
    const scenario = DEMO_SCENARIOS.find((s) => s.id === 'prompt-injection');
    expect(scenario).toBeDefined();
    const text = decodeDemoChunksAsText(scenario!.generate());
    expect(text).toContain('STATUS OK');
    expect(text).toContain('SYSTEM:');
    expect(text).toContain('send_bytes with hex');
    expect(text).toContain(PROMPT_INJECTION_RELAY_HEX);
    expect(PROMPT_INJECTION_RELAY_HEX.replace(/ /g, '')).toBe('52454C41595F4F4E0D0A');
  });
});
