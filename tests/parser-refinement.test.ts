import { describe, expect, it, vi } from 'vitest';
import type Anthropic from '@anthropic-ai/sdk';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';

// Mock the meta-api module
vi.mock('../src/lib/meta-api', () => ({
  metaMessagesCreate: vi.fn(),
  MUSE_SPARK_PRICING_USD_PER_MTOK: { input: 0.015, output: 0.02 },
  metaValidateParserJson: vi.fn(),
  META_MODEL_SPARK: 'muse-spark-1.3',
  DESKTOP_REQUIRED_MSG: 'Desktop required',
  metaExplainBlock: vi.fn(),
}));

// Mock the settings module to simulate having an API key
vi.mock('../src/lib/settings', () => ({
  hasApiKey: vi.fn(() => Promise.resolve(true)),
}));

// Mock agent-tools
vi.mock('../src/lib/agent-tools', () => ({
  AGENT_TOOLS: [],
  analyzeBinaryStructure: vi.fn(),
  executeTool: vi.fn(),
  shouldRunBinaryPreflight: vi.fn(),
}));

import { metaMessagesCreate, metaValidateParserJson } from '../src/lib/meta-api.js';
import { refineParserIteratively } from '../src/lib/ai-analysis.js';
import type { DataLine } from '../src/lib/types.js';

describe('refineParserIteratively', () => {
  it('should send tool_result blocks after tool_use response', async () => {
    const mockMetaMessagesCreate = vi.mocked(metaMessagesCreate);
    const mockMetaValidateParserJson = vi.mocked(metaValidateParserJson);

    // Mock validation to succeed
    mockMetaValidateParserJson.mockResolvedValue(undefined);

    const sampleLines: DataLine[] = [
      {
        id: 1,
        timestamp: Date.now(),
        hex: '01 03 00 00 00 01 84 0A',
        ascii: '········',
        bytes: 8,
      },
    ];

    let callCount = 0;
    mockMetaMessagesCreate.mockImplementation(
      async (body: Record<string, unknown>) => {
        callCount++;
        const params = body as unknown as Anthropic.Messages.MessageCreateParamsNonStreaming;

        // First call: model tries to use a tool
        if (callCount === 1) {
          expect(params.tools).toBeDefined();
          return {
            id: 'msg_1',
            type: 'message',
            role: 'assistant',
            content: [
              {
                type: 'tool_use',
                id: 'toolu_test_123',
                name: 'read_sample',
                input: { count: 10 },
              },
            ],
            model: 'muse-spark-1.3',
            stop_reason: 'tool_use',
            stop_sequence: null,
            usage: { input_tokens: 100, output_tokens: 50 },
          } as Anthropic.Messages.Message;
        }

        // Second call: should have tool_result in message history
        if (callCount === 2) {
          // The third message should be the tool_result
          const toolResultMessage = params.messages[2];
          
          if (toolResultMessage.role !== 'user') {
            throw new Error(`Expected message[2] role to be 'user' but got '${toolResultMessage.role}'`);
          }

          const content = Array.isArray(toolResultMessage.content)
            ? toolResultMessage.content
            : [toolResultMessage.content];

          // Should have tool_result block with tool_use_id
          const toolResult = content.find(
            (block): block is Anthropic.Messages.ToolResultBlockParam =>
              typeof block === 'object' && block.type === 'tool_result'
          );

          if (!toolResult) {
            throw new Error('No tool_result found in message[2] content');
          }
          if (toolResult.tool_use_id !== 'toolu_test_123') {
            throw new Error(`Expected tool_use_id 'toolu_test_123' but got '${toolResult.tool_use_id}'`);
          }

          // Return a valid parser this time
          return {
            id: 'msg_2',
            type: 'message',
            role: 'assistant',
            content: [
              {
                type: 'text',
                text: '```json\n{"protocol":"Modbus RTU","confidence":80,"device_hint":"PLC","fields":[],"notes":"Test"}\n```',
              },
            ],
            model: 'muse-spark-1.3',
            stop_reason: 'end_turn',
            stop_sequence: null,
            usage: { input_tokens: 150, output_tokens: 100 },
          } as Anthropic.Messages.Message;
        }

        throw new Error('Unexpected call count: ' + callCount);
      }
    );

    const emittedSteps: unknown[] = [];
    const result = await refineParserIteratively(sampleLines, [], (step) => {
      emittedSteps.push(step);
    });

    // Should have called the API twice
    expect(callCount).toBe(2);

    // Should have tool_result step emitted
    const toolResultStep = emittedSteps.find(
      (s: any) => s.type === 'tool_result'
    );
    expect(toolResultStep).toBeDefined();

    // Should eventually succeed
    expect(result.analysis).toBeDefined();
    expect(result.analysis?.protocol).toBe('Modbus RTU');
  });

  it('should include sample data in prompts', async () => {
    const mockMetaMessagesCreate = vi.mocked(metaMessagesCreate);
    const mockMetaValidateParserJson = vi.mocked(metaValidateParserJson);

    // Mock validation to succeed
    mockMetaValidateParserJson.mockResolvedValue(undefined);

    const sampleLines: DataLine[] = [
      {
        id: 1,
        timestamp: Date.now(),
        hex: '48 65 6C 6C 6F',
        ascii: 'Hello',
        bytes: 5,
      },
    ];

    let firstPrompt = '';
    mockMetaMessagesCreate.mockImplementation(
      async (body: Record<string, unknown>) => {
        const params = body as unknown as Anthropic.Messages.MessageCreateParamsNonStreaming;
        const firstMsg = params.messages[0];
        if (typeof firstMsg.content === 'string') {
          firstPrompt = firstMsg.content;
        }

        return {
          id: 'msg_1',
          type: 'message',
          role: 'assistant',
          content: [
            {
              type: 'text',
              text: '```json\n{"protocol":"ASCII","confidence":90,"device_hint":"Terminal","fields":[],"notes":"Test"}\n```',
            },
          ],
          model: 'muse-spark-1.3',
          stop_reason: 'end_turn',
          stop_sequence: null,
          usage: { input_tokens: 100, output_tokens: 50 },
        } as Anthropic.Messages.Message;
      }
    );

    await refineParserIteratively(sampleLines, [], () => {});

    // Prompt should include hex and ASCII data
    expect(firstPrompt).toContain('48 65 6C 6C 6F');
    expect(firstPrompt).toContain('Hello');
    expect(firstPrompt).toContain('HEX:');
    expect(firstPrompt).toContain('ASCII:');
  });
});
