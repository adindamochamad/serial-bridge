mod discovery;
mod meta_api;
pub mod safemode;
mod webhook_http;

use serde::{Deserialize, Serialize};
use serialport::{DataBits, FlowControl, Parity, SerialPort, StopBits};
use std::collections::HashMap;
use std::io::Write;
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PortDataEvent {
    port: String,
    data: Vec<u8>,
    timestamp: u64,
    text: String,
    direction: String, // "rx" (from device) or "tx" (to device)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BaudProbeResult {
    baud: u32,
    score: f32,
    bytes_read: usize,
    printable_count: usize,
    sample_ascii: String,
    error: Option<String>,
}

const WRITE_QUEUE_CAPACITY: usize = 32;

pub(crate) struct PortRegistration {
    active: bool,
    write_tx: SyncSender<Vec<u8>>,
    /// Unique per start; a reader thread only honours/removes its own registration.
    generation: u64,
}

static NEXT_MONITOR_GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

pub(crate) type MonitorRegistry = Arc<Mutex<HashMap<String, PortRegistration>>>;

pub(crate) fn write_to_port(
    registry: &MonitorRegistry,
    port: &str,
    data: Vec<u8>,
) -> Result<(), String> {
    if data.is_empty() {
        return Err("Cannot send empty payload".into());
    }
    let map = registry.lock().map_err(|e| e.to_string())?;
    let state = map
        .get(port)
        .ok_or_else(|| format!("{} is not being monitored", port))?;
    if !state.active {
        return Err(format!("{} monitor is not active", port));
    }
    state.write_tx.try_send(data).map_err(|e| match e {
        std::sync::mpsc::TrySendError::Full(_) => {
            "Write queue full — wait for the port to drain".into()
        }
        std::sync::mpsc::TrySendError::Disconnected(_) => {
            format!("Write channel closed: {}", e)
        }
    })?;
    Ok(())
}

fn is_active(registry: &MonitorRegistry, port: &str, generation: u64) -> bool {
    registry
        .lock()
        .ok()
        .and_then(|map| {
            map.get(port)
                .map(|r| r.active && r.generation == generation)
        })
        .unwrap_or(false)
}

/// Remove the registration only if it still belongs to this reader thread.
fn remove_if_owned(registry: &MonitorRegistry, port: &str, generation: u64) {
    if let Ok(mut map) = registry.lock() {
        if map
            .get(port)
            .map(|r| r.generation == generation)
            .unwrap_or(false)
        {
            map.remove(port);
        }
    }
}

fn kandidat_port_macos(nama_port: &str) -> Vec<String> {
    #[cfg(target_os = "macos")]
    {
        let mut daftar_port = vec![nama_port.to_string()];
        if let Some(akhiran_port) = nama_port.strip_prefix("/dev/cu.") {
            daftar_port.push(format!("/dev/tty.{}", akhiran_port));
        } else if let Some(akhiran_port) = nama_port.strip_prefix("/dev/tty.") {
            daftar_port.push(format!("/dev/cu.{}", akhiran_port));
        }
        daftar_port
    }
    #[cfg(not(target_os = "macos"))]
    {
        vec![nama_port.to_string()]
    }
}

/// On Unix, serialport opens ttys with TIOCEXCL. The flag lives on the tty, not
/// the fd, so if anything else keeps the tty open (a socat/virtual pty pair, a
/// USB-serial driver that holds the line) the flag survives our close and every
/// reopen fails with EBUSY — e.g. the CAS warmup 19200→9600 stop/start.
/// This guard clears the flag just before our fd is closed.
pub(crate) struct ExclusiveRelease(#[allow(dead_code)] Option<i32>);

impl Drop for ExclusiveRelease {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some(fd) = self.0 {
            // SAFETY: fd is still owned by the serial port, which is dropped after this guard.
            unsafe {
                libc::ioctl(fd, libc::TIOCNXCL as _);
            }
        }
    }
}

/// Returns the port plus a guard; bind the guard AFTER the port so it drops first.
fn buka_port_dengan_konfigurasi(
    nama_port: &str,
    baud_rate: u32,
) -> Result<(Box<dyn SerialPort>, ExclusiveRelease), String> {
    let mut daftar_error: Vec<String> = Vec::new();
    for kandidat_port in kandidat_port_macos(nama_port) {
        let builder = serialport::new(&kandidat_port, baud_rate)
            .data_bits(DataBits::Eight)
            .parity(Parity::None)
            .stop_bits(StopBits::One)
            .flow_control(FlowControl::None)
            .timeout(std::time::Duration::from_millis(100));
        #[cfg(unix)]
        let hasil_buka = serialport::TTYPort::open(&builder).map(|p| {
            use std::os::unix::io::AsRawFd;
            let fd = p.as_raw_fd();
            (
                Box::new(p) as Box<dyn SerialPort>,
                ExclusiveRelease(Some(fd)),
            )
        });
        #[cfg(not(unix))]
        let hasil_buka = builder.open().map(|p| (p, ExclusiveRelease(None)));
        match hasil_buka {
            Ok(port_serial) => return Ok(port_serial),
            Err(error_buka) => {
                daftar_error.push(format!("{} => {}", kandidat_port, error_buka));
            }
        }
    }
    Err(daftar_error.join(" | "))
}

#[tauri::command]
fn discover_devices() -> Vec<discovery::DiscoveredDevice> {
    discovery::list_devices()
}

#[tauri::command]
async fn start_port_monitor(
    app: AppHandle,
    registry: tauri::State<'_, MonitorRegistry>,
    port: String,
    baud_rate: u32,
) -> Result<(), String> {
    let registry = registry.inner().clone();

    // Create write channel synchronously so send_bytes_to_port calls race-free
    let (write_tx, write_rx) = sync_channel::<Vec<u8>>(WRITE_QUEUE_CAPACITY);
    let generation = NEXT_MONITOR_GENERATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    {
        let mut map = registry.lock().map_err(|e| e.to_string())?;
        if map.get(&port).map(|r| r.active).unwrap_or(false) {
            return Err(format!("{} is already being monitored", port));
        }
        map.insert(
            port.clone(),
            PortRegistration {
                active: true,
                write_tx,
                generation,
            },
        );
    }

    let port_name = port.clone();
    let reg = registry.clone();

    std::thread::spawn(move || {
        let (mut serial, _excl_guard) = match buka_port_dengan_konfigurasi(&port_name, baud_rate) {
            Ok(p) => p,
            Err(e) => {
                let _ = app.emit(
                    "port:error",
                    serde_json::json!({ "port": &port_name, "error": e.to_string() }),
                );
                remove_if_owned(&reg, &port_name, generation);
                return;
            }
        };

        let mut buf = vec![0u8; 512];

        loop {
            if !is_active(&reg, &port_name, generation) {
                break;
            }

            // Drain any pending writes first (non-blocking)
            while let Ok(data) = write_rx.try_recv() {
                match serial.write_all(&data) {
                    Ok(_) => {
                        let _ = serial.flush();
                        let timestamp = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64;
                        let text = String::from_utf8_lossy(&data).into_owned();
                        let _ = app.emit(
                            "port:data",
                            PortDataEvent {
                                port: port_name.clone(),
                                data: data.clone(),
                                timestamp,
                                text,
                                direction: "tx".into(),
                            },
                        );
                    }
                    Err(e) => {
                        let _ = app.emit(
                            "port:error",
                            serde_json::json!({
                                "port": &port_name,
                                "error": format!("Write failed: {}", e),
                            }),
                        );
                    }
                }
            }

            // Read incoming data
            match serial.read(&mut buf) {
                Ok(0) => {}
                Ok(n) => {
                    let data = buf[..n].to_vec();
                    let text = String::from_utf8_lossy(&data).into_owned();
                    let timestamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;
                    let _ = app.emit(
                        "port:data",
                        PortDataEvent {
                            port: port_name.clone(),
                            data,
                            timestamp,
                            text,
                            direction: "rx".into(),
                        },
                    );
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(e) => {
                    let _ = app.emit(
                        "port:error",
                        serde_json::json!({
                            "port": &port_name,
                            "error": format!("Read failed: {}", e),
                        }),
                    );
                    break;
                }
            }
        }

        // A newer monitor may have replaced us (e.g. CAS warmup stop→start);
        // don't tear down its registration or tell the UI the port closed.
        let owned = reg
            .lock()
            .map(|m| {
                m.get(&port_name)
                    .map(|r| r.generation == generation)
                    .unwrap_or(true)
            })
            .unwrap_or(true);
        remove_if_owned(&reg, &port_name, generation);
        if owned {
            let _ = app.emit("port:closed", &port_name);
        }
    });

    Ok(())
}

#[tauri::command]
fn send_bytes_to_port(
    registry: tauri::State<'_, MonitorRegistry>,
    safe_mode: tauri::State<'_, safemode::gate::SafeModeState>,
    port: String,
    data: Vec<u8>,
) -> Result<(), String> {
    safe_mode.validate_human_send_payload(&data)?;
    write_to_port(registry.inner(), &port, data.clone())?;
    safe_mode.record_human_write(&port, &data)?;
    Ok(())
}

fn is_printable_byte(b: u8) -> bool {
    (0x20..=0x7E).contains(&b) || b == b'\n' || b == b'\r' || b == b'\t'
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::mpsc::sync_channel;

    #[test]
    fn is_printable_byte_treats_ascii_space_as_printable() {
        assert!(is_printable_byte(b' '));
        assert!(!is_printable_byte(0x1F));
    }

    #[test]
    fn write_to_port_delivers_bytes_to_channel() {
        let registry: MonitorRegistry = Arc::new(Mutex::new(HashMap::new()));
        let (write_tx, write_rx) = sync_channel(WRITE_QUEUE_CAPACITY);
        registry.lock().unwrap().insert(
            "ttyTEST".into(),
            PortRegistration {
                active: true,
                write_tx,
                generation: 0,
            },
        );
        write_to_port(&registry, "ttyTEST", vec![1, 2, 3]).unwrap();
        assert_eq!(write_rx.recv().unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn write_to_port_errors_unchanged() {
        let registry: MonitorRegistry = Arc::new(Mutex::new(HashMap::new()));
        assert_eq!(
            write_to_port(&registry, "ttyX", vec![]).unwrap_err(),
            "Cannot send empty payload"
        );
        assert_eq!(
            write_to_port(&registry, "ttyX", vec![1]).unwrap_err(),
            "ttyX is not being monitored"
        );
        let (write_tx, _) = sync_channel(WRITE_QUEUE_CAPACITY);
        registry.lock().unwrap().insert(
            "ttyY".into(),
            PortRegistration {
                active: false,
                write_tx,
                generation: 0,
            },
        );
        assert_eq!(
            write_to_port(&registry, "ttyY", vec![1]).unwrap_err(),
            "ttyY monitor is not active"
        );
    }
}

fn probe_single_sync(port: &str, baud: u32) -> BaudProbeResult {
    let (mut serial, _excl_guard) = match buka_port_dengan_konfigurasi(port, baud) {
        Ok(p) => p,
        Err(e) => {
            return BaudProbeResult {
                baud,
                score: 0.0,
                bytes_read: 0,
                printable_count: 0,
                sample_ascii: String::new(),
                error: Some(e.to_string()),
            };
        }
    };

    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(1000);
    let mut buf = vec![0u8; 1024];
    let mut bytes_read = 0usize;
    let mut printable_count = 0usize;
    let mut sample: Vec<u8> = Vec::with_capacity(80);

    while std::time::Instant::now() < deadline {
        match serial.read(&mut buf) {
            Ok(0) => {}
            Ok(n) => {
                bytes_read += n;
                for &b in &buf[..n] {
                    if is_printable_byte(b) {
                        printable_count += 1;
                    }
                }
                if sample.len() < 80 {
                    let take = std::cmp::min(80 - sample.len(), n);
                    sample.extend_from_slice(&buf[..take]);
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {}
            Err(_) => break,
        }
    }

    let score = if bytes_read > 0 {
        printable_count as f32 / bytes_read as f32
    } else {
        0.0
    };

    BaudProbeResult {
        baud,
        score,
        bytes_read,
        printable_count,
        sample_ascii: String::from_utf8_lossy(&sample)
            .chars()
            .map(|c| {
                if c.is_control() && c != '\n' && c != '\r' && c != '\t' {
                    '·'
                } else {
                    c
                }
            })
            .collect(),
        error: None,
    }
}

#[tauri::command]
async fn probe_single_baud(
    registry: tauri::State<'_, MonitorRegistry>,
    port: String,
    baud: u32,
) -> Result<BaudProbeResult, String> {
    {
        let map = registry.lock().map_err(|e| e.to_string())?;
        if map.get(&port).map(|r| r.active).unwrap_or(false) {
            return Err(format!(
                "{} is currently being monitored — stop it first",
                port
            ));
        }
    }

    tauri::async_runtime::spawn_blocking(move || probe_single_sync(&port, baud))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn stop_port_monitor(
    registry: tauri::State<'_, MonitorRegistry>,
    port: String,
) -> Result<(), String> {
    let mut map = registry.lock().map_err(|e| e.to_string())?;
    if let Some(reg) = map.get_mut(&port) {
        reg.active = false;
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
struct WebhookResponse {
    ok: bool,
    status: u16,
    body_preview: String,
}

/// Forward a webhook from the Rust side so the request is NOT subject to the
/// WebView's CORS policy. Legacy/self-hosted endpoints (e.g. CodeIgniter REST
/// APIs) rarely send Access-Control-* headers, so a browser `fetch()` fails the
/// preflight with "TypeError: Failed to fetch" before the request leaves the
/// app. A native client has no such restriction.
#[tauri::command]
async fn send_webhook_request(
    url: String,
    method: String,
    headers: HashMap<String, String>,
    body: String,
) -> Result<WebhookResponse, String> {
    let http_method = webhook_http::validate_webhook_method(&method)?;
    let parsed_url = webhook_http::validate_webhook_url(&url)?;
    webhook_http::validate_webhook_body(&body)?;
    let filtered = webhook_http::filter_webhook_request_headers(headers);
    let client = webhook_http::build_webhook_http_client()?;

    let mut request = client.request(http_method, parsed_url);
    for (key, value) in filtered {
        request = request.header(key, value);
    }
    request = request.body(body);

    let response = request.send().await.map_err(|e| e.to_string())?;
    let status = response.status().as_u16();
    let ok = response.status().is_success();
    let text = response.text().await.unwrap_or_default();
    let body_preview: String = text.chars().take(120).collect();

    Ok(WebhookResponse {
        ok,
        status,
        body_preview,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let registry: MonitorRegistry = Arc::new(Mutex::new(HashMap::new()));

    tauri::Builder::default()
        .manage(registry)
        .setup(|app| {
            let policy_path = app
                .path()
                .app_config_dir()
                .map_err(|e| e.to_string())?
                .join("safemode-policy.json");
            let audit_path = app
                .path()
                .app_data_dir()
                .map_err(|e| e.to_string())?
                .join("safemode-audit.jsonl");
            let enabled_path = app
                .path()
                .app_config_dir()
                .map_err(|e| e.to_string())?
                .join("safemode-enabled.json");
            let safe_state = safemode::gate::init_state(&policy_path, audit_path, enabled_path)?;
            app.manage(safe_state);
            meta_api::init_meta_api(app.handle())?;
            Ok(())
        })
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            discover_devices,
            start_port_monitor,
            stop_port_monitor,
            probe_single_baud,
            send_bytes_to_port,
            send_webhook_request,
            safemode::gate::safemode_begin_session,
            safemode::gate::safemode_authorize_tool,
            safemode::gate::safemode_resolve_approval,
            safemode::gate::safemode_agent_send_bytes,
            safemode::gate::safemode_read_audit,
            safemode::gate::safemode_get_enabled,
            safemode::gate::safemode_set_enabled,
            meta_api::meta_set_api_key,
            meta_api::meta_clear_api_key,
            meta_api::meta_key_status,
            meta_api::meta_messages_create,
            meta_api::meta_count_tokens,
            meta_api::meta_list_models,
            meta_api::meta_explain_block,
            meta_api::meta_validate_parser_json,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
