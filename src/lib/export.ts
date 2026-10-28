import type { AnalysisField, ProtocolAnalysis } from './ai-analysis.js';
import type { DataLine } from './types.js';
import { compileFields } from './parser.js';

export type ExportFormat = 'csv' | 'json';
export type ExportScope = 'structured' | 'raw';

export interface ExportContext {
  port: string;
  device_class?: string;
  analysis: ProtocolAnalysis | null;
  lines: DataLine[];
}

// ── CSV helpers ─────────────────────────────────────────────────────────────

function csvCell(v: unknown): string {
  if (v === null || v === undefined) return '';
  const s = String(v);
  if (/[",\n\r]/.test(s)) return `"${s.replace(/"/g, '""')}"`;
  return s;
}

function csvRow(cells: unknown[]): string {
  return cells.map(csvCell).join(',');
}

function isoTimestamp(ms: number): string {
  return new Date(ms).toISOString();
}

// ── Raw exports ─────────────────────────────────────────────────────────────

export function exportRawCSV(ctx: ExportContext): string {
  const header = csvRow(['timestamp_iso', 'timestamp_ms', 'bytes', 'ascii', 'hex']);
  const rows = ctx.lines.map((l) =>
    csvRow([isoTimestamp(l.timestamp), l.timestamp, l.bytes, l.ascii, l.hex])
  );
  return [header, ...rows].join('\n');
}

export function exportRawJSON(ctx: ExportContext): string {
  return JSON.stringify(
    {
      port: ctx.port,
      device_class: ctx.device_class,
      exported_at: new Date().toISOString(),
      line_count: ctx.lines.length,
      lines: ctx.lines.map((l) => ({
        timestamp: l.timestamp,
        timestamp_iso: isoTimestamp(l.timestamp),
        ascii: l.ascii,
        hex: l.hex,
        bytes: l.bytes,
      })),
    },
    null,
    2
  );
}

// ── Structured exports ──────────────────────────────────────────────────────

function matchAllFields(
  line: DataLine,
  fields: AnalysisField[],
  compiled: Record<string, RegExp | null>
): Record<string, { value: string | number | boolean; raw: string }> {
  const matches: Record<string, { value: string | number | boolean; raw: string }> = {};
  for (const f of fields) {
    const re = compiled[f.name];
    if (!re) continue;
    const m = line.ascii.match(re);
    if (!m) continue;
    const raw = m[f.capture_group ?? 1] ?? m[0];
    let value: string | number | boolean = raw;
    if (f.data_type === 'number') {
      const n = parseFloat(raw);
      if (!isNaN(n)) value = n;
    } else if (f.data_type === 'boolean') {
      value = ['true', '1', 'yes', 'on'].includes(raw.toLowerCase().trim());
    }
    matches[f.name] = { value, raw };
  }
  return matches;
}

export function exportStructuredCSV(ctx: ExportContext): string {
  if (!ctx.analysis) return exportRawCSV(ctx);
  const fields = ctx.analysis.fields;
  const compiled = compileFields(fields);
  const fieldNames = fields.map((f) => f.name);

  const header = csvRow(['timestamp_iso', 'timestamp_ms', 'ascii', ...fieldNames]);
  const rows = ctx.lines.map((l) => {
    const matches = matchAllFields(l, fields, compiled);
    const cells: unknown[] = [isoTimestamp(l.timestamp), l.timestamp, l.ascii];
    for (const name of fieldNames) {
      cells.push(matches[name]?.value ?? '');
    }
    return csvRow(cells);
  });
  return [header, ...rows].join('\n');
}

export function exportStructuredJSON(ctx: ExportContext): string {
  if (!ctx.analysis) return exportRawJSON(ctx);
  const fields = ctx.analysis.fields;
  const compiled = compileFields(fields);

  const records = ctx.lines
    .map((l) => {
      const matches = matchAllFields(l, fields, compiled);
      if (Object.keys(matches).length === 0) return null;
      const parsed: Record<string, unknown> = {};
      for (const [name, m] of Object.entries(matches)) {
        const f = fields.find((ff) => ff.name === name);
        parsed[name] = {
          value: m.value,
          ...(f?.unit ? { unit: f.unit } : {}),
          ...(f?.data_type ? { type: f.data_type } : {}),
        };
      }
      return {
        timestamp: l.timestamp,
        timestamp_iso: isoTimestamp(l.timestamp),
        raw_ascii: l.ascii,
        fields: parsed,
      };
    })
    .filter((r) => r !== null);

  return JSON.stringify(
    {
      port: ctx.port,
      device_class: ctx.device_class,
      protocol: ctx.analysis.protocol,
      confidence: ctx.analysis.confidence,
      device_hint: ctx.analysis.device_hint,
      fields_schema: fields,
      notes: ctx.analysis.notes,
      exported_at: new Date().toISOString(),
      record_count: records.length,
      records,
    },
    null,
    2
  );
}

// ── Dispatcher + download ───────────────────────────────────────────────────

export function buildExport(
  ctx: ExportContext,
  format: ExportFormat,
  scope: ExportScope
): { content: string; filename: string; mime: string } {
  const safePort = ctx.port.replace(/[^a-zA-Z0-9._-]/g, '_');
  const ts = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
  const ext = format === 'csv' ? 'csv' : 'json';
  const filename = `serialbridge_${scope}_${safePort}_${ts}.${ext}`;
  const mime = format === 'csv' ? 'text/csv' : 'application/json';

  let content: string;
  if (scope === 'structured') {
    content = format === 'csv' ? exportStructuredCSV(ctx) : exportStructuredJSON(ctx);
  } else {
    content = format === 'csv' ? exportRawCSV(ctx) : exportRawJSON(ctx);
  }
  return { content, filename, mime };
}

export function downloadFile(content: string, filename: string, mime: string): void {
  const blob = new Blob([content], { type: `${mime};charset=utf-8` });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  a.style.display = 'none';
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
