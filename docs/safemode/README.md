# Safe Mode — operator guide

Safe Mode gates **AI agent** serial writes in the Rust host. Human **Command Sender** uses a separate path; agent tools must go through `safemode_agent_send_bytes`.

---

## English

### Where files live

| File | Location (Tauri 2) | Purpose |
|------|-------------------|---------|
| Policy | `<app_config_dir>/safemode-policy.json` | Tool classes, payload limits, hex patterns, approval timeout |
| Audit log | `<app_data_dir>/safemode-audit.jsonl` | Append-only JSONL with hash chain |
| Enabled flag | `<app_config_dir>/safemode-enabled.json` | `true` / `false`; missing file = **on** |

On macOS, config dir is typically `~/Library/Application Support/com.serialbridge.app/` (exact identifier matches `tauri.conf.json` bundle id). Data dir is `~/Library/Application Support/com.serialbridge.app/` or the platform equivalent for app data — use **Settings → Safe Mode** in the app to confirm paths if unsure.

### Editing policy

1. Quit the app or avoid concurrent edits while it is running.
2. Open `safemode-policy.json` in a text editor.
3. Keep `"default": "deny"`. The loader rejects `"default": "allow"`.
4. **`deny_patterns`** win over everything (hex regex on uppercase spaced hex, e.g. `01 06`).
5. Empty **`allow_patterns`** means no allow-list filter; writes still need approval when `require_approval` is true.
6. Non-empty **`allow_patterns`**: payload must match at least one pattern or the write is denied before approval.
7. Invalid JSON or unknown keys → **fail closed** (agent writes denied; known read tools still allowed). The UI should show a policy error.

Shipped defaults are created on first run (see Rust `safemode` policy loader).

### Turning Safe Mode off

**Settings** → disable Safe Mode → type **`MATIKAN`** to confirm. The change is audited. When off, agent writes skip approval but still respect size limits, deny patterns, and audit logging.

### Agent `send_bytes` results (FR-9)

| Situation | Shape | Agent hint |
|-----------|--------|------------|
| Approved, real port | `ok: true`, `bytes_sent`, `response_preview`, … | Same as before Safe Mode |
| Approved, `demo://…` port | `ok: false`, `simulated: true`, `blocked: false`, `bytes_sent` | No serial write; demo only |
| User deny | `ok: false`, `blocked: true`, `reason: "denied_by_user"`, `retry: "do_not_retry"` | Do not call `send_bytes` again this investigation |
| Timeout | `reason: "timeout"`, `retry: "do_not_retry"` | Same |
| Policy deny | `reason: "policy_denied:…"` or `rate_limited`, `session_blocked`, `safe_mode_policy_invalid` | `retry: "do_not_retry"` |

### Demos and docs

- **Prompt injection trap** demo: [DEMO_SCRIPT.md](./DEMO_SCRIPT.md) (60 s).
- Implementation milestones: [serialbridge-safe-mode-plan.md](./serialbridge-safe-mode-plan.md).

---

## Bahasa Indonesia

### Lokasi file

| File | Lokasi | Fungsi |
|------|--------|--------|
| Policy | `<app_config_dir>/safemode-policy.json` | Aturan tool, batas payload, pola hex, timeout persetujuan |
| Audit | `<app_data_dir>/safemode-audit.jsonl` | Catatan append-only dengan rantai hash |
| Saklar | `<app_config_dir>/safemode-enabled.json` | Nyala/mati; file hilang = **nyala** |

### Mengedit policy

1. Tutup aplikasi atau hati-hati jika file diedit saat app jalan.
2. Edit `safemode-policy.json` dengan editor teks.
3. `"default"` harus `"deny"`.
4. **`deny_patterns`** selalu menang.
5. **`allow_patterns`** kosong = tidak ada filter allow-list; persetujuan tetap jalan jika `require_approval: true`.
6. **`allow_patterns`** tidak kosong = payload harus cocok minimal satu pola, kalau tidak ditolak sebelum dialog.
7. JSON rusak → semua tulis agent ditolak (baca tetap untuk tool baca yang dikenal).

### Mematikan Safe Mode

Di **Settings**, matikan Safe Mode dan ketik **`MATIKAN`**. Peristiwa tercatat di audit. Saat mati, persetujuan dilewati; batas ukuran, deny pattern, dan audit tetap jalan.

### Hasil `send_bytes` untuk agent

| Kejadian | Bentuk | Catatan |
|----------|--------|---------|
| Disetujui, port nyata | `ok: true`, … | Tulis serial sungguhan |
| Disetujui, port `demo://` | `simulated: true` | Tidak ada tulis ke kabel |
| Ditolak operator / timeout | `blocked: true`, `retry: "do_not_retry"` | Jangan panggil `send_bytes` lagi di sesi investigasi ini |
| Policy / rate / sesi | `reason` sesuai aturan | `retry: "do_not_retry"` |

Demo 60 detik: [DEMO_SCRIPT.md](./DEMO_SCRIPT.md).
