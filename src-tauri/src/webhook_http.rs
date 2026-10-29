use reqwest::redirect::Policy;
use reqwest::Client;
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::Duration;

pub const MAX_WEBHOOK_URL_LEN: usize = 2048;
pub const MAX_WEBHOOK_BODY_BYTES: usize = 256 * 1024;
pub const WEBHOOK_TIMEOUT: Duration = Duration::from_secs(10);

const BLOCKED_METADATA_HOST: &str = "metadata.google.internal";

/// Allowed webhook HTTP verbs (matches front-end `WebhookMethod`).
pub fn validate_webhook_method(method: &str) -> Result<reqwest::Method, String> {
    match method {
        "POST" | "PUT" | "PATCH" => reqwest::Method::from_bytes(method.as_bytes())
            .map_err(|e| format!("Invalid method '{}': {}", method, e)),
        _ => Err(format!(
            "Webhook method must be POST, PUT, or PATCH (got '{}')",
            method
        )),
    }
}

/// Parse and validate webhook target URL (scheme, host blocklist, no credentials).
pub fn validate_webhook_url(url_str: &str) -> Result<reqwest::Url, String> {
    if url_str.len() > MAX_WEBHOOK_URL_LEN {
        return Err("Webhook URL exceeds maximum length".into());
    }
    let url = reqwest::Url::parse(url_str).map_err(|e| format!("Invalid URL: {}", e))?;
    if !url.username().is_empty() || url.password().is_some() {
        return Err("Webhook URL must not contain credentials".into());
    }
    match url.scheme() {
        "http" | "https" => {}
        other => {
            return Err(format!(
                "Webhook URL must be http or https (got '{}')",
                other
            ))
        }
    }
    let host = url
        .host_str()
        .ok_or_else(|| "Webhook URL must have a host".to_string())?;
    if host.eq_ignore_ascii_case(BLOCKED_METADATA_HOST) {
        return Err("Webhook URL host is not allowed".into());
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_blocked_webhook_ip(&ip) {
            return Err("Webhook URL target is not allowed".into());
        }
    }
    Ok(url)
}

fn is_blocked_webhook_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            // Link-local (includes cloud metadata 169.254.169.254)
            o[0] == 169 && o[1] == 254
        }
        IpAddr::V6(_) => false,
    }
}

/// Drop hop-by-hop / spoofable headers; keys compared case-insensitively.
pub fn filter_webhook_request_headers(headers: HashMap<String, String>) -> HashMap<String, String> {
    let blocked = ["host", "content-length", "transfer-encoding", "connection"];
    headers
        .into_iter()
        .filter(|(k, _)| {
            let lower = k.to_ascii_lowercase();
            !blocked.contains(&lower.as_str())
        })
        .collect()
}

pub fn validate_webhook_body(body: &str) -> Result<(), String> {
    if body.len() > MAX_WEBHOOK_BODY_BYTES {
        return Err(format!(
            "Webhook body exceeds {} bytes",
            MAX_WEBHOOK_BODY_BYTES
        ));
    }
    Ok(())
}

pub fn build_webhook_http_client() -> Result<Client, String> {
    Client::builder()
        .timeout(WEBHOOK_TIMEOUT)
        .redirect(Policy::none())
        .build()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_post_put_patch_only() {
        assert!(validate_webhook_method("POST").is_ok());
        assert!(validate_webhook_method("PUT").is_ok());
        assert!(validate_webhook_method("PATCH").is_ok());
        assert!(validate_webhook_method("GET").is_err());
        assert!(validate_webhook_method("DELETE").is_err());
    }

    #[test]
    fn allows_https_and_localhost() {
        assert!(validate_webhook_url("https://example.com/webhook").is_ok());
        assert!(validate_webhook_url("http://127.0.0.1:8080/api").is_ok());
        assert!(validate_webhook_url("http://localhost/hook").is_ok());
    }

    #[test]
    fn rejects_credentials_and_metadata() {
        assert!(validate_webhook_url("https://user:pass@example.com/").is_err());
        assert!(validate_webhook_url("http://metadata.google.internal/").is_err());
        assert!(validate_webhook_url("http://169.254.169.254/latest/").is_err());
    }

    #[test]
    fn rejects_non_http_scheme() {
        assert!(validate_webhook_url("ftp://example.com/").is_err());
    }

    #[test]
    fn filters_spoofable_headers() {
        let headers = filter_webhook_request_headers(HashMap::from([
            ("Content-Type".into(), "application/json".into()),
            ("Host".into(), "evil".into()),
            ("gs-key".into(), "secret".into()),
            ("Connection".into(), "close".into()),
        ]));
        assert!(headers.contains_key("Content-Type"));
        assert!(headers.contains_key("gs-key"));
        assert!(!headers.contains_key("Host"));
        assert!(!headers.contains_key("Connection"));
    }
}
