import { describe, it, expect } from 'vitest';
import {
  compileFields,
  parseStream,
  parseStreamWithCompiled,
  latest,
  numericHistory,
} from '../src/lib/parser.js';
import type { AnalysisField } from '../src/lib/ai-analysis.js';
import type { DataLine } from '../src/lib/types.js';

function makeLine(ascii: string, id = 1, ts = Date.now()): DataLine {
  return {
    id,
    timestamp: ts,
    hex: Array.from(ascii)
      .map((c) => c.charCodeAt(0).toString(16).padStart(2, '0'))
      .join(' '),
    ascii,
    bytes: ascii.length,
  };
}

describe('compileFields', () => {
  it('compiles valid regex', () => {
    const fields: AnalysisField[] = [
      { name: 'w', value: '', description: '', regex: 'GS,\\s*([\\d.]+)\\s*kg' },
    ];
    const compiled = compileFields(fields);
    expect(compiled.w).toBeInstanceOf(RegExp);
  });

  it('returns null for undefined regex', () => {
    const fields: AnalysisField[] = [{ name: 'w', value: '', description: '' }];
    expect(compileFields(fields).w).toBeNull();
  });

  it('returns null for malformed regex', () => {
    const fields: AnalysisField[] = [
      { name: 'bad', value: '', description: '', regex: '([unclosed' },
    ];
    expect(compileFields(fields).bad).toBeNull();
  });

  it('rejects nested-quantifier ReDoS pattern (a+)+', () => {
    const fields: AnalysisField[] = [
      { name: 'redos', value: '', description: '', regex: '(a+)+b' },
    ];
    expect(compileFields(fields).redos).toBeNull();
  });

  it('rejects overly long pattern', () => {
    const fields: AnalysisField[] = [
      { name: 'long', value: '', description: '', regex: 'a'.repeat(400) },
    ];
    expect(compileFields(fields).long).toBeNull();
  });
});

describe('parseStream — CAS scale protocol', () => {
  const casFields: AnalysisField[] = [
    {
      name: 'weight',
      value: '1.234',
      description: 'gross weight kg',
      regex: 'GS,\\s*([\\d.]+)\\s*kg',
      capture_group: 1,
      data_type: 'number',
      unit: 'kg',
    },
    {
      name: 'stable',
      value: 'ST',
      description: 'stability flag',
      regex: '^(ST|US),',
      capture_group: 1,
      data_type: 'string',
    },
  ];

  it('extracts weight and stability from real-looking frames', () => {
    const lines = [
      makeLine('ST,GS,  1.234 kg', 1),
      makeLine('US,GS,  1.450 kg', 2),
      makeLine('ST,GS,  1.450 kg', 3),
    ];
    const result = parseStream(lines, casFields);
    expect(result.parsableFieldCount).toBe(2);
    expect(numericHistory(result.history.weight)).toEqual([1.234, 1.45, 1.45]);
    expect(result.history.stable.map((v) => v.value)).toEqual(['ST', 'US', 'ST']);
  });

  it('coerces number type correctly', () => {
    const lines = [makeLine('ST,GS,  0.025 kg')];
    const result = parseStream(lines, casFields);
    expect(latest(result.history.weight)?.value).toBe(0.025);
    expect(typeof latest(result.history.weight)?.value).toBe('number');
  });

  it('parseStreamWithCompiled matches parseStream output', () => {
    const lines = [makeLine('ST,GS,  1.234 kg', 1)];
    const compiled = compileFields(casFields);
    const a = parseStream(lines, casFields);
    const b = parseStreamWithCompiled(lines, casFields, compiled);
    expect(b.history.weight).toEqual(a.history.weight);
    expect(b.parsableFieldCount).toBe(a.parsableFieldCount);
  });
});

describe('parseStream — NMEA 0183', () => {
  const nmeaFields: AnalysisField[] = [
    {
      name: 'lat_raw',
      value: '',
      description: 'latitude DDMM.MMMM',
      regex: '\\$GPGGA,[^,]*,([\\d.]+),',
      capture_group: 1,
      data_type: 'number',
    },
  ];

  it('captures latitude from GPGGA', () => {
    const lines = [
      makeLine('$GPGGA,123456.00,0612.4800,S,10650.7300,E,1,08,0.9,25.3,M,-10.2,M,,*47'),
    ];
    const result = parseStream(lines, nmeaFields);
    expect(latest(result.history.lat_raw)?.value).toBe(612.48);
  });
});

describe('parseStream — Modbus RTU binary (match_hex + hex_u16_be)', () => {
  const modbusFields: AnalysisField[] = [
    {
      name: 'temp',
      value: '',
      description: 'Temperature ×10',
      unit: '°C·10',
      regex: '^01 03 14 ([0-9A-F]{2} [0-9A-F]{2})',
      match_hex: true,
      capture_group: 1,
      data_type: 'hex_u16_be',
    },
    {
      name: 'pressure',
      value: '',
      description: 'Pressure hPa',
      unit: 'hPa',
      regex: '^01 03 14 [0-9A-F]{2} [0-9A-F]{2} ([0-9A-F]{2} [0-9A-F]{2})',
      match_hex: true,
      capture_group: 1,
      data_type: 'hex_u16_be',
    },
  ];

  function modbusLine(registers: number[], id = 1): DataLine {
    const bytes: number[] = [0x01, 0x03, 0x14];
    for (const r of registers) {
      bytes.push((r >> 8) & 0xff);
      bytes.push(r & 0xff);
    }
    // Pad with zeros + fake CRC for the 25-byte frame
    while (bytes.length < 23) bytes.push(0);
    bytes.push(0xaa, 0xbb);
    return {
      id,
      timestamp: 1_000_000 + id,
      hex: bytes.map((b) => b.toString(16).padStart(2, '0').toUpperCase()).join(' '),
      ascii: '',
      bytes: bytes.length,
    };
  }

  it('extracts register 0 as big-endian u16', () => {
    const lines = [
      modbusLine([254, 1013, 45]),
      modbusLine([256, 1014, 46], 2),
      modbusLine([300, 1015, 47], 3),
    ];
    const result = parseStream(lines, modbusFields);
    expect(result.parsableFieldCount).toBe(2);
    expect(numericHistory(result.history.temp)).toEqual([254, 256, 300]);
  });

  it('extracts register 1 at correct offset', () => {
    const lines = [modbusLine([254, 1013, 45])];
    const result = parseStream(lines, modbusFields);
    expect(latest(result.history.pressure)?.value).toBe(1013);
  });
});

describe('coerce hex variants', () => {
  const lines = (hexCap: string): DataLine[] => [{
    id: 1,
    timestamp: 0,
    hex: `PREFIX ${hexCap}`,
    ascii: '',
    bytes: 0,
  }];

  it('hex_u16_le reverses byte order', () => {
    const f: AnalysisField[] = [{
      name: 'x', value: '', description: '',
      regex: 'PREFIX ([0-9A-F]{2} [0-9A-F]{2})',
      match_hex: true,
      capture_group: 1,
      data_type: 'hex_u16_le',
    }];
    // "01 02" little-endian = 0x0201 = 513
    expect(latest(parseStream(lines('01 02'), f).history.x)?.value).toBe(513);
  });

  it('hex_s16_be handles negative two\'s complement', () => {
    const f: AnalysisField[] = [{
      name: 'x', value: '', description: '',
      regex: 'PREFIX ([0-9A-F]{2} [0-9A-F]{2})',
      match_hex: true,
      capture_group: 1,
      data_type: 'hex_s16_be',
    }];
    // "FF FE" = -2
    expect(latest(parseStream(lines('FF FE'), f).history.x)?.value).toBe(-2);
  });
});

describe('parseStream — key=value CSV (Arduino)', () => {
  it('extracts multiple fields per line', () => {
    const fields: AnalysisField[] = [
      {
        name: 'temp',
        value: '',
        description: '',
        regex: 'temp=([\\d.]+)',
        capture_group: 1,
        data_type: 'number',
      },
      {
        name: 'humid',
        value: '',
        description: '',
        regex: 'humid=([\\d.]+)',
        capture_group: 1,
        data_type: 'number',
      },
    ];
    const lines = [
      makeLine('temp=24.5,humid=62.3,light=512'),
      makeLine('temp=24.7,humid=62.1,light=520'),
    ];
    const result = parseStream(lines, fields);
    expect(numericHistory(result.history.temp)).toEqual([24.5, 24.7]);
    expect(numericHistory(result.history.humid)).toEqual([62.3, 62.1]);
  });
});
