use reqwest::redirect::Policy;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Manager, State};

pub const META_MESSAGES_URL: &str = "https://api.meta.ai/v1/messages";
pub const META_COUNT_TOKENS_URL: &str = "https://api.meta.ai/v1/messages/count_tokens";
pub const META_MODELS_URL: &str = "https://api.meta.ai/v1/models";
pub const META_MODEL_SPARK: &str = "muse-spark-1.3";
const KEY_FILE_NAME: &str = "meta-model-api-key.txt";
const MAX_MESSAGES_BODY_BYTES: usize = 2 * 1024 * 1024;
const META_HTTP_TIMEOUT: Duration = Duration::from_secs(60);

pub struct MetaApiState {
    key_path: PathBuf,
    client: Client,
}

impl MetaApiState {
    pub fn new(key_path: PathBuf) -> Result<Self, String> {
        if let Some(parent) = key_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config dir: {}", e))?;
        }
        let client = Client::builder()
            .redirect(Policy::none())
            .timeout(META_HTTP_TIMEOUT)
            .build()
            .map_err(|e| format!("HTTP client error: {}", e))?;
        Ok(Self { key_path, client })
    }
}

fn validate_meta_key(key: &str) -> Result<String, String> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err("API key cannot be empty".into());
    }
    if !trimmed.starts_with("LLM_") {
        return Err(
            "Not a Meta Model API Standard key. Paste a key from dev.meta.ai that starts with LLM_."
                .into(),
        );
    }
    Ok(trimmed.to_string())
}

fn read_stored_key(path: &Path) -> Result<Option<String>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let raw =
        fs::read_to_string(path).map_err(|e| format!("Failed to read API key file: {}", e))?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    validate_meta_key(trimmed).map(Some)
}

fn write_stored_key(path: &Path, key: &str) -> Result<(), String> {
    let valid = validate_meta_key(key)?;
    fs::write(path, format!("{}\n", valid)).map_err(|e| format!("Failed to save API key: {}", e))
}

fn clear_stored_key(path: &Path) -> Result<(), String> {
    if path.exists() {
        fs::remove_file(path).map_err(|e| format!("Failed to remove API key file: {}", e))?;
    }
    Ok(())
}

fn load_key_for_request(state: &MetaApiState) -> Result<String, String> {
    match read_stored_key(&state.key_path)? {
        Some(k) => Ok(k),
        None => Err(
            "No Meta Model API key. Open Settings in the desktop app and paste a Standard key (LLM_…) from dev.meta.ai."
                .into(),
        ),
    }
}

fn scrub_error_message(msg: &str, key: &str) -> String {
    if key.is_empty() {
        return msg.to_string();
    }
    msg.replace(key, "[redacted]")
}

/// Meta accepts `cache_control` on the request in some SDK shapes; strip per-block
/// fields that have caused `invalid_request_error` on the Messages adapter.
fn sanitize_messages_body(obj: &mut serde_json::Map<String, serde_json::Value>) {
    obj.insert(
        "model".into(),
        serde_json::Value::String(META_MODEL_SPARK.into()),
    );
    if let Some(system) = obj.get_mut("system") {
        if let Some(arr) = system.as_array_mut() {
            for block in arr {
                if let Some(b) = block.as_object_mut() {
                    b.remove("cache_control");
                }
            }
        }
    }
    obj.remove("cache_control");
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaKeyStatus {
    pub configured: bool,
    pub key_class: String,
}

#[tauri::command]
pub fn meta_set_api_key(state: State<'_, Mutex<MetaApiState>>, key: String) -> Result<(), String> {
    let valid = validate_meta_key(&key)?;
    let guard = state.lock().map_err(|e| e.to_string())?;
    write_stored_key(&guard.key_path, &valid)
}

#[tauri::command]
pub fn meta_clear_api_key(state: State<'_, Mutex<MetaApiState>>) -> Result<(), String> {
    let guard = state.lock().map_err(|e| e.to_string())?;
    clear_stored_key(&guard.key_path)
}

#[tauri::command]
pub fn meta_key_status(state: State<'_, Mutex<MetaApiState>>) -> Result<MetaKeyStatus, String> {
    let guard = state.lock().map_err(|e| e.to_string())?;
    match read_stored_key(&guard.key_path)? {
        Some(_) => Ok(MetaKeyStatus {
            configured: true,
            key_class: "meta".into(),
        }),
        None => Ok(MetaKeyStatus {
            configured: false,
            key_class: "empty".into(),
        }),
    }
}

#[tauri::command]
pub async fn meta_messages_create(
    state: State<'_, Mutex<MetaApiState>>,
    body: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let (client, key) = {
        let guard = state.lock().map_err(|e| e.to_string())?;
        let key = load_key_for_request(&guard)?;
        (guard.client.clone(), key)
    };

    let body_str = serde_json::to_string(&body).map_err(|e| e.to_string())?;
    if body_str.len() > MAX_MESSAGES_BODY_BYTES {
        return Err("Request body exceeds maximum size".into());
    }

    let mut payload = body;
    let obj = payload
        .as_object_mut()
        .ok_or_else(|| "Request body must be a JSON object".to_string())?;
    sanitize_messages_body(obj);

    let response = client
        .post(META_MESSAGES_URL)
        .header("Authorization", format!("Bearer {}", key))
        .header("Content-Type", "application/json")
        .header("anthropic-version", "2023-06-01")
        .json(&payload)
        .send()
        .await
        .map_err(|e| scrub_error_message(&e.to_string(), &key))?;

    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| scrub_error_message(&e.to_string(), &key))?;

    if !status.is_success() {
        let preview = if text.len() > 400 {
            format!("{}…", text.chars().take(400).collect::<String>())
        } else {
            text.clone()
        };
        return Err(scrub_error_message(
            &format!("Meta Model API error ({}): {}", status.as_u16(), preview),
            &key,
        ));
    }

    serde_json::from_str(&text)
        .map_err(|e| scrub_error_message(&format!("Invalid JSON from Meta Model API: {}", e), &key))
}

#[tauri::command]
pub async fn meta_count_tokens(
    state: State<'_, Mutex<MetaApiState>>,
    body: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let (client, key) = {
        let guard = state.lock().map_err(|e| e.to_string())?;
        let key = load_key_for_request(&guard)?;
        (guard.client.clone(), key)
    };

    let mut payload = body;
    let obj = payload
        .as_object_mut()
        .ok_or_else(|| "Request body must be a JSON object".to_string())?;
    sanitize_messages_body(obj);
    obj.remove("max_tokens");

    let response = client
        .post(META_COUNT_TOKENS_URL)
        .header("Authorization", format!("Bearer {}", key))
        .header("Content-Type", "application/json")
        .header("anthropic-version", "2023-06-01")
        .json(&payload)
        .send()
        .await
        .map_err(|e| scrub_error_message(&e.to_string(), &key))?;

    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| scrub_error_message(&e.to_string(), &key))?;

    if !status.is_success() {
        return Err(scrub_error_message(
            &format!(
                "Meta Model API error ({}): {}",
                status.as_u16(),
                text.chars().take(200).collect::<String>()
            ),
            &key,
        ));
    }

    serde_json::from_str(&text).map_err(|e| format!("Invalid count_tokens JSON: {}", e))
}

#[derive(Debug, Deserialize)]
struct ModelsListResponse {
    data: Option<Vec<ModelEntry>>,
}

#[derive(Debug, Deserialize)]
struct ModelEntry {
    id: String,
}

#[tauri::command]
pub async fn meta_list_models(
    state: State<'_, Mutex<MetaApiState>>,
) -> Result<Vec<String>, String> {
    let (client, key) = {
        let guard = state.lock().map_err(|e| e.to_string())?;
        let key = load_key_for_request(&guard)?;
        (guard.client.clone(), key)
    };

    let response = client
        .get(META_MODELS_URL)
        .header("Authorization", format!("Bearer {}", key))
        .header("anthropic-version", "2023-06-01")
        .send()
        .await
        .map_err(|e| scrub_error_message(&e.to_string(), &key))?;

    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| scrub_error_message(&e.to_string(), &key))?;

    if !status.is_success() {
        return Err(scrub_error_message(
            &format!(
                "Meta Model API error ({}): {}",
                status.as_u16(),
                text.chars().take(200).collect::<String>()
            ),
            &key,
        ));
    }

    let parsed: ModelsListResponse =
        serde_json::from_str(&text).map_err(|e| format!("Invalid models list JSON: {}", e))?;
    Ok(parsed
        .data
        .unwrap_or_default()
        .into_iter()
        .map(|m| m.id)
        .collect())
}

/// Generate a plain-language explanation of why Safe Mode blocked a command.
/// This is explanation-only — cannot bypass the gate.
#[tauri::command]
pub async fn meta_explain_block(
    state: State<'_, Mutex<MetaApiState>>,
    blocked_hex: String,
    block_reason: String,
) -> Result<String, String> {
    let (client, key) = {
        let guard = state.lock().map_err(|e| e.to_string())?;
        let key = load_key_for_request(&guard)?;
        (guard.client.clone(), key)
    };

    let prompt = format!(
        "SerialBridge Safe Mode blocked this command:\nHex: {}\nReason: {}\n\nExplain in 2-3 sentences why this was blocked and what risk it posed. Be specific to the hex bytes shown.",
        blocked_hex, block_reason
    );

    let payload = serde_json::json!({
        "model": META_MODEL_SPARK,
        "max_tokens": 200,
        "messages": [{
            "role": "user",
            "content": prompt
        }]
    });

    let response = client
        .post(META_MESSAGES_URL)
        .header("Authorization", format!("Bearer {}", key))
        .header("Content-Type", "application/json")
        .header("anthropic-version", "2023-06-01")
        .json(&payload)
        .send()
        .await
        .map_err(|e| scrub_error_message(&e.to_string(), &key))?;

    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| scrub_error_message(&e.to_string(), &key))?;

    if !status.is_success() {
        return Err(scrub_error_message(
            &format!(
                "Meta Model API error ({}): {}",
                status.as_u16(),
                text.chars().take(200).collect::<String>()
            ),
            &key,
        ));
    }

    let parsed: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("Invalid JSON from Meta Model API: {}", e))?;

    // Extract text content from response
    let content = parsed
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|block| block.get("text"))
        .and_then(|t| t.as_str())
        .ok_or("No text content in API response")?;

    Ok(content.to_string())
}

/// Validate that a JSON value conforms to ProtocolAnalysis schema.
/// Returns Ok(()) if valid, Err with description of what's missing/wrong.
#[tauri::command]
pub fn meta_validate_parser_json(body: serde_json::Value) -> Result<(), String> {
    let obj = body.as_object().ok_or("Parser must be a JSON object")?;

    // Required fields
    obj.get("protocol")
        .and_then(|v| v.as_str())
        .ok_or("Missing or invalid 'protocol' string")?;

    let confidence = obj
        .get("confidence")
        .and_then(|v| v.as_u64())
        .ok_or("Missing or invalid 'confidence' number")?;
    if confidence > 100 {
        return Err("'confidence' must be 0-100".into());
    }

    obj.get("device_hint")
        .and_then(|v| v.as_str())
        .ok_or("Missing or invalid 'device_hint' string")?;

    // Validate fields array
    let fields = obj
        .get("fields")
        .and_then(|v| v.as_array())
        .ok_or("Missing or invalid 'fields' array")?;

    for (i, field) in fields.iter().enumerate() {
        let f = field
            .as_object()
            .ok_or(format!("fields[{}] must be object", i))?;

        f.get("name")
            .and_then(|v| v.as_str())
            .ok_or(format!("fields[{}].name missing or invalid", i))?;

        f.get("description")
            .and_then(|v| v.as_str())
            .ok_or(format!("fields[{}].description missing or invalid", i))?;

        // If regex present, it must be a string
        if let Some(regex_val) = f.get("regex") {
            regex_val
                .as_str()
                .ok_or(format!("fields[{}].regex must be string", i))?;
        }

        // If data_type present, validate it's one of the known types
        if let Some(dt) = f.get("data_type").and_then(|v| v.as_str()) {
            match dt {
                "number" | "string" | "boolean" | "hex_u16_be" | "hex_u16_le" | "hex_s16_be"
                | "hex_u8" => {}
                _ => return Err(format!("fields[{}].data_type '{}' not recognized", i, dt)),
            }
        }
    }

    obj.get("notes")
        .and_then(|v| v.as_str())
        .ok_or("Missing or invalid 'notes' string")?;

    Ok(())
}

pub fn init_meta_api(app: &AppHandle) -> Result<(), String> {
    let key_path = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join(KEY_FILE_NAME);
    let state = MetaApiState::new(key_path)?;
    app.manage(Mutex::new(state));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_llm_prefix_only() {
        assert!(validate_meta_key("LLM_abc").is_ok());
        assert!(validate_meta_key("  LLM_x  ").is_ok());
        assert!(validate_meta_key("sk-ant").is_err());
        assert!(validate_meta_key("").is_err());
    }

    #[test]
    fn scrubs_key_from_errors() {
        let key = "LLM_secret_key_value";
        let msg = format!("failed with {}", key);
        assert!(!scrub_error_message(&msg, key).contains(key));
    }

    #[test]
    fn messages_url_is_pinned() {
        assert!(META_MESSAGES_URL.starts_with("https://api.meta.ai"));
        assert!(META_MODELS_URL.starts_with("https://api.meta.ai"));
        assert!(META_COUNT_TOKENS_URL.starts_with("https://api.meta.ai"));
    }

    #[test]
    fn sanitize_strips_block_cache_control() {
        let mut body = serde_json::json!({
            "model": "other",
            "system": [{ "type": "text", "text": "hi", "cache_control": { "type": "ephemeral" } }],
            "messages": [{ "role": "user", "content": "x" }]
        });
        let map = body.as_object_mut().unwrap();
        sanitize_messages_body(map);
        let sys = map.get("system").unwrap().as_array().unwrap();
        assert!(sys[0].get("cache_control").is_none());
        assert_eq!(map.get("model").unwrap().as_str(), Some(META_MODEL_SPARK));
    }

    #[test]
    fn validate_parser_accepts_valid_schema() {
        let valid = serde_json::json!({
            "protocol": "CAS Scale",
            "confidence": 95,
            "device_hint": "CAS SW-1 weighing scale",
            "fields": [{
                "name": "weight",
                "value": "1.234",
                "description": "Weight in kg",
                "unit": "kg",
                "regex": "GS,\\s*([\\d.]+)\\s*kg",
                "data_type": "number"
            }],
            "notes": "Test notes"
        });
        assert!(super::meta_validate_parser_json(valid).is_ok());
    }

    #[test]
    fn validate_parser_rejects_missing_protocol() {
        let invalid = serde_json::json!({
            "confidence": 90,
            "device_hint": "Unknown",
            "fields": [],
            "notes": "Missing protocol"
        });
        let err = super::meta_validate_parser_json(invalid).unwrap_err();
        assert!(err.contains("protocol"));
    }

    #[test]
    fn validate_parser_rejects_invalid_confidence() {
        let invalid = serde_json::json!({
            "protocol": "Test",
            "confidence": 150,
            "device_hint": "Device",
            "fields": [],
            "notes": "Over 100"
        });
        let err = super::meta_validate_parser_json(invalid).unwrap_err();
        assert!(err.contains("confidence"));
    }

    #[test]
    fn validate_parser_rejects_invalid_data_type() {
        let invalid = serde_json::json!({
            "protocol": "Test",
            "confidence": 90,
            "device_hint": "Device",
            "fields": [{
                "name": "test",
                "description": "Test field",
                "data_type": "invalid_type"
            }],
            "notes": "Bad type"
        });
        let err = super::meta_validate_parser_json(invalid).unwrap_err();
        assert!(err.contains("data_type"));
        assert!(err.contains("not recognized"));
    }

    #[test]
    fn validate_parser_accepts_hex_data_types() {
        let valid = serde_json::json!({
            "protocol": "Modbus RTU",
            "confidence": 85,
            "device_hint": "Temperature sensor",
            "fields": [{
                "name": "temp",
                "description": "Temperature",
                "regex": "^01 03 14 ([0-9A-F]{2} [0-9A-F]{2})",
                "match_hex": true,
                "data_type": "hex_u16_be",
                "unit": "°C"
            }],
            "notes": "Modbus test"
        });
        assert!(super::meta_validate_parser_json(valid).is_ok());
    }
}
