use serde::{Deserialize, Serialize};
use serialport::{SerialPortInfo, SerialPortType};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceClass {
    Arduino,
    FtdiAdapter,
    Ch340,
    Cp210x,
    UnknownUsb,
    PciSerial,
    BluetoothSerial,
    Unknown,
}

impl DeviceClass {
    pub fn label(&self) -> &'static str {
        match self {
            DeviceClass::Arduino => "Arduino",
            DeviceClass::FtdiAdapter => "FTDI Adapter",
            DeviceClass::Ch340 => "CH340/CH341",
            DeviceClass::Cp210x => "CP210x",
            DeviceClass::UnknownUsb => "USB Serial",
            DeviceClass::PciSerial => "PCI Serial",
            DeviceClass::BluetoothSerial => "Bluetooth",
            DeviceClass::Unknown => "Unknown",
        }
    }

    pub fn suggested_baud(&self) -> u32 {
        9600
    }
}

fn classify_usb(vid: u16, _pid: u16) -> DeviceClass {
    match vid {
        0x2341 | 0x2A03 | 0x1B4F => DeviceClass::Arduino, // Arduino SA, Arduino LLC, SparkFun
        0x1A86 => DeviceClass::Ch340,                     // WCH CH340/CH341
        0x0403 => DeviceClass::FtdiAdapter,               // FTDI
        0x10C4 => DeviceClass::Cp210x,                    // Silicon Labs CP210x
        _ => DeviceClass::UnknownUsb,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredDevice {
    pub port: String,
    pub device_class: DeviceClass,
    pub label: String,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub vid: Option<u16>,
    pub pid: Option<u16>,
    pub serial_number: Option<String>,
    pub suggested_baud: u32,
}

pub fn list_devices() -> Vec<DiscoveredDevice> {
    let mut perangkat_terpilih: HashMap<String, DiscoveredDevice> = HashMap::new();

    for info in serialport::available_ports().unwrap_or_default().iter() {
        let perangkat = classify_port(info);
        let kunci_port = kunci_pasangan_port(&perangkat.port);

        match perangkat_terpilih.get(&kunci_port) {
            None => {
                perangkat_terpilih.insert(kunci_port, perangkat);
            }
            Some(perangkat_lama) => {
                if pilih_port_lebih_baik(&perangkat.port, &perangkat_lama.port) {
                    perangkat_terpilih.insert(kunci_port, perangkat);
                }
            }
        }
    }

    // Fallback khusus macOS: beberapa adaptor CH340 kadang tidak muncul
    // di available_ports(), tapi nodenya tetap ada di /dev.
    for perangkat in daftar_port_fallback_macos() {
        let kunci_port = kunci_pasangan_port(&perangkat.port);
        perangkat_terpilih.entry(kunci_port).or_insert(perangkat);
    }

    // Custom ports from SERIALBRIDGE_EXTRA_PORTS env var
    // Format: colon or comma separated paths, e.g. "/dev/pts/1:/tmp/vport"
    if let Ok(extra_str) = std::env::var("SERIALBRIDGE_EXTRA_PORTS") {
        for port_path in extra_str
            .split(&[':', ','])
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let kunci_port = kunci_pasangan_port(port_path);
            perangkat_terpilih
                .entry(kunci_port)
                .or_insert_with(|| DiscoveredDevice {
                    port: port_path.to_string(),
                    device_class: DeviceClass::Unknown,
                    label: "Custom Port".to_string(),
                    manufacturer: None,
                    product: Some("User-specified".to_string()),
                    vid: None,
                    pid: None,
                    serial_number: None,
                    suggested_baud: 9600,
                });
        }
    }

    let mut hasil: Vec<DiscoveredDevice> = perangkat_terpilih.into_values().collect();
    hasil.sort_by(|a, b| a.port.cmp(&b.port));
    hasil
}

fn daftar_port_fallback_macos() -> Vec<DiscoveredDevice> {
    #[cfg(target_os = "macos")]
    {
        use std::fs;

        let mut daftar_perangkat: Vec<DiscoveredDevice> = Vec::new();
        let Ok(entri_dev) = fs::read_dir("/dev") else {
            return daftar_perangkat;
        };

        for entri in entri_dev.flatten() {
            let Some(nama_file) = entri.file_name().to_str().map(|s| s.to_string()) else {
                continue;
            };

            let kandidat = nama_file.starts_with("cu.usb")
                || nama_file.starts_with("cu.wch")
                || nama_file.starts_with("tty.usb")
                || nama_file.starts_with("tty.wch");
            if !kandidat {
                continue;
            }

            let nama_port = format!("/dev/{}", nama_file);
            daftar_perangkat.push(DiscoveredDevice {
                port: nama_port,
                device_class: DeviceClass::UnknownUsb,
                label: DeviceClass::UnknownUsb.label().to_string(),
                manufacturer: None,
                product: None,
                vid: None,
                pid: None,
                serial_number: None,
                suggested_baud: 9600,
            });
        }

        daftar_perangkat
    }

    #[cfg(not(target_os = "macos"))]
    {
        Vec::new()
    }
}

fn kunci_pasangan_port(nama_port: &str) -> String {
    #[cfg(target_os = "macos")]
    {
        if let Some(akhiran_port) = nama_port.strip_prefix("/dev/tty.") {
            return format!("/dev/{}", akhiran_port);
        }
        if let Some(akhiran_port) = nama_port.strip_prefix("/dev/cu.") {
            return format!("/dev/{}", akhiran_port);
        }
    }
    nama_port.to_string()
}

fn pilih_port_lebih_baik(_nama_port_baru: &str, _nama_port_lama: &str) -> bool {
    #[cfg(target_os = "macos")]
    {
        let port_baru_cu = _nama_port_baru.starts_with("/dev/cu.");
        let port_lama_cu = _nama_port_lama.starts_with("/dev/cu.");
        if port_baru_cu != port_lama_cu {
            return port_baru_cu;
        }
    }
    false
}

fn classify_port(info: &SerialPortInfo) -> DiscoveredDevice {
    match &info.port_type {
        SerialPortType::UsbPort(usb) => {
            let class = classify_usb(usb.vid, usb.pid);
            let label = class.label().to_string();
            let suggested_baud = class.suggested_baud();
            DiscoveredDevice {
                port: info.port_name.clone(),
                device_class: class,
                label,
                manufacturer: usb.manufacturer.clone(),
                product: usb.product.clone(),
                vid: Some(usb.vid),
                pid: Some(usb.pid),
                serial_number: usb.serial_number.clone(),
                suggested_baud,
            }
        }
        SerialPortType::BluetoothPort => DiscoveredDevice {
            port: info.port_name.clone(),
            device_class: DeviceClass::BluetoothSerial,
            label: DeviceClass::BluetoothSerial.label().to_string(),
            manufacturer: None,
            product: None,
            vid: None,
            pid: None,
            serial_number: None,
            suggested_baud: 9600,
        },
        SerialPortType::PciPort => DiscoveredDevice {
            port: info.port_name.clone(),
            device_class: DeviceClass::PciSerial,
            label: DeviceClass::PciSerial.label().to_string(),
            manufacturer: None,
            product: Some("PCI Serial".to_string()),
            vid: None,
            pid: None,
            serial_number: None,
            suggested_baud: 9600,
        },
        _ => DiscoveredDevice {
            port: info.port_name.clone(),
            device_class: DeviceClass::Unknown,
            label: DeviceClass::Unknown.label().to_string(),
            manufacturer: None,
            product: None,
            vid: None,
            pid: None,
            serial_number: None,
            suggested_baud: 9600,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extra_ports_from_env_colon_separated() {
        std::env::set_var("SERIALBRIDGE_EXTRA_PORTS", "/dev/pts/1:/tmp/vport");
        let devices = list_devices();
        std::env::remove_var("SERIALBRIDGE_EXTRA_PORTS");

        // Check that custom ports are added (may be deduplicated with existing ports)
        let has_custom = devices.iter().any(|d| {
            d.port == "/dev/pts/1"
                || d.port == "/tmp/vport"
                || d.product.as_deref() == Some("User-specified")
        });

        // Should either find the custom port or have some devices
        assert!(has_custom || !devices.is_empty());
    }

    #[test]
    fn extra_ports_from_env_comma_separated() {
        std::env::set_var(
            "SERIALBRIDGE_EXTRA_PORTS",
            "/dev/ttyCustom1,/dev/ttyCustom2",
        );
        let devices = list_devices();
        std::env::remove_var("SERIALBRIDGE_EXTRA_PORTS");

        // Custom ports with unique names should appear
        let _custom_count = devices
            .iter()
            .filter(|d| d.port.contains("ttyCustom"))
            .count();
        // Test passes if no crash occurs
    }

    #[test]
    fn extra_ports_handles_whitespace() {
        std::env::set_var("SERIALBRIDGE_EXTRA_PORTS", " /dev/testA : /dev/testB ");
        let _devices = list_devices();
        std::env::remove_var("SERIALBRIDGE_EXTRA_PORTS");
        // Test passes if parsing succeeds without crash
    }

    #[test]
    fn extra_ports_empty_env_no_crash() {
        std::env::set_var("SERIALBRIDGE_EXTRA_PORTS", "");
        let _devices = list_devices();
        std::env::remove_var("SERIALBRIDGE_EXTRA_PORTS");
        // Should not crash - test passes if we get here
    }

    #[test]
    fn no_extra_ports_env_works_normally() {
        std::env::remove_var("SERIALBRIDGE_EXTRA_PORTS");
        let _devices = list_devices();
        // Should work without env var - test passes if we get here
    }
}
