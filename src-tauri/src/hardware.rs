use serde::{Deserialize, Serialize};
use std::process::Command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IdentifierInfo {
    pub id: String,
    pub label: String,
    pub value: String,
    pub modifiable: bool,
    pub description: String,
}

fn or_unknown(val: Option<String>) -> String {
    val.filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "无法获取".to_string())
}

#[allow(dead_code)]
fn run_cmd(cmd: &str, args: &[&str]) -> Option<String> {
    let mut command = Command::new(cmd);
    command.args(args);
    #[cfg(target_os = "windows")]
    command.creation_flags(0x08000000);
    command
        .output()
        .ok()
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if s.is_empty() { None } else { Some(s) }
        })
}

pub fn get_all() -> Vec<IdentifierInfo> {
    let mut results = vec![get_mac_address(), get_machine_guid()];
    results.extend(get_system_identifiers());
    results
}

// ==================== MAC Address (cross-platform) ====================

fn get_mac_address() -> IdentifierInfo {
    let value = mac_address::get_mac_address()
        .ok()
        .flatten()
        .map(|m| m.to_string())
        .unwrap_or_else(|| "无法获取".to_string());

    IdentifierInfo {
        id: "mac_address".into(),
        label: "MAC 地址".into(),
        value,
        modifiable: true,
        description: "网络适配器物理地址".into(),
    }
}

// ==================== Machine GUID (platform-specific) ====================

#[cfg(target_os = "windows")]
fn get_machine_guid() -> IdentifierInfo {
    use winreg::enums::*;
    use winreg::RegKey;

    let value = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey("SOFTWARE\\Microsoft\\Cryptography")
        .and_then(|key| key.get_value::<String, _>("MachineGuid"))
        .ok();

    IdentifierInfo {
        id: "machine_guid".into(),
        label: "Machine GUID".into(),
        value: or_unknown(value),
        modifiable: true,
        description: "Windows 机器唯一标识符".into(),
    }
}

#[cfg(target_os = "linux")]
fn get_machine_guid() -> IdentifierInfo {
    let value = std::fs::read_to_string("/etc/machine-id")
        .ok()
        .map(|s| s.trim().to_string());

    IdentifierInfo {
        id: "machine_guid".into(),
        label: "Machine ID".into(),
        value: or_unknown(value),
        modifiable: true,
        description: "Linux 机器唯一标识符".into(),
    }
}

#[cfg(target_os = "macos")]
fn get_machine_guid() -> IdentifierInfo {
    let value = run_cmd("ioreg", &["-rd1", "-c", "IOPlatformExpertDevice"])
        .and_then(|output| {
            output
                .lines()
                .find(|line| line.contains("IOPlatformUUID"))
                .and_then(|line| line.split('"').nth(3))
                .map(|s| s.to_string())
        });

    IdentifierInfo {
        id: "machine_guid".into(),
        label: "Hardware UUID".into(),
        value: or_unknown(value),
        modifiable: false,
        description: "macOS 硬件 UUID".into(),
    }
}

// ==================== System Identifiers (batch per platform) ====================

#[cfg(target_os = "windows")]
#[derive(Deserialize, Default, Debug)]
struct WmiInfo {
    #[serde(default)]
    motherboard: Option<String>,
    #[serde(default)]
    bios: Option<String>,
    #[serde(default)]
    cpu: Option<String>,
    #[serde(default)]
    disk: Option<String>,
}

#[cfg(target_os = "windows")]
fn get_wmi_info() -> WmiInfo {
    let script = r#"
$ErrorActionPreference = 'SilentlyContinue'
$info = @{ motherboard = ''; bios = ''; cpu = ''; disk = '' }
try { $info.motherboard = [string](@(Get-CimInstance Win32_BaseBoard)[0].SerialNumber) } catch {}
try { $info.bios = [string](@(Get-CimInstance Win32_BIOS)[0].SerialNumber) } catch {}
try { $info.cpu = [string](@(Get-CimInstance Win32_Processor)[0].ProcessorId) } catch {}
try { $info.disk = [string](@(Get-CimInstance Win32_DiskDrive)[0].SerialNumber) } catch {}
$info | ConvertTo-Json -Compress
"#;
    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", script]);
    cmd.creation_flags(0x08000000);
    cmd.output()
        .ok()
        .and_then(|output| {
            let stdout = String::from_utf8_lossy(&output.stdout);
            serde_json::from_str(stdout.trim()).ok()
        })
        .unwrap_or_default()
}

#[cfg(target_os = "windows")]
fn get_system_identifiers() -> Vec<IdentifierInfo> {
    let wmi = get_wmi_info();
    vec![
        IdentifierInfo {
            id: "disk_serial".into(),
            label: "硬盘序列号".into(),
            value: or_unknown(wmi.disk),
            modifiable: false,
            description: "主硬盘硬件序列号".into(),
        },
        IdentifierInfo {
            id: "motherboard_serial".into(),
            label: "主板序列号".into(),
            value: or_unknown(wmi.motherboard),
            modifiable: false,
            description: "主板制造商序列号".into(),
        },
        IdentifierInfo {
            id: "bios_serial".into(),
            label: "BIOS 序列号".into(),
            value: or_unknown(wmi.bios),
            modifiable: false,
            description: "BIOS 固件序列号".into(),
        },
        IdentifierInfo {
            id: "cpu_id".into(),
            label: "CPU ID".into(),
            value: or_unknown(wmi.cpu),
            modifiable: false,
            description: "处理器唯一标识符".into(),
        },
    ]
}

#[cfg(target_os = "linux")]
fn get_system_identifiers() -> Vec<IdentifierInfo> {
    vec![
        {
            let value = run_cmd("lsblk", &["-ndo", "SERIAL"])
                .and_then(|s| s.lines().next().map(|l| l.trim().to_string()));
            IdentifierInfo {
                id: "disk_serial".into(),
                label: "硬盘序列号".into(),
                value: or_unknown(value),
                modifiable: false,
                description: "主硬盘硬件序列号".into(),
            }
        },
        {
            let value = std::fs::read_to_string("/sys/devices/virtual/dmi/id/board_serial")
                .ok()
                .or_else(|| run_cmd("dmidecode", &["-s", "baseboard-serial-number"]))
                .map(|s| s.trim().to_string());
            IdentifierInfo {
                id: "motherboard_serial".into(),
                label: "主板序列号".into(),
                value: or_unknown(value),
                modifiable: false,
                description: "主板制造商序列号".into(),
            }
        },
        {
            let value = std::fs::read_to_string("/sys/devices/virtual/dmi/id/bios_version")
                .ok()
                .or_else(|| run_cmd("dmidecode", &["-s", "bios-version"]))
                .map(|s| s.trim().to_string());
            IdentifierInfo {
                id: "bios_serial".into(),
                label: "BIOS 版本".into(),
                value: or_unknown(value),
                modifiable: false,
                description: "BIOS 固件版本信息".into(),
            }
        },
        {
            let value = std::fs::read_to_string("/proc/cpuinfo")
                .ok()
                .and_then(|content| {
                    content
                        .lines()
                        .find(|l| l.starts_with("model name"))
                        .and_then(|l| l.split(':').nth(1))
                        .map(|s| s.trim().to_string())
                });
            IdentifierInfo {
                id: "cpu_id".into(),
                label: "CPU 信息".into(),
                value: or_unknown(value),
                modifiable: false,
                description: "处理器型号信息".into(),
            }
        },
    ]
}

#[cfg(target_os = "macos")]
fn get_system_identifiers() -> Vec<IdentifierInfo> {
    let hw_info = run_cmd("system_profiler", &["SPHardwareDataType"]).unwrap_or_default();

    let extract = |key: &str| -> Option<String> {
        hw_info
            .lines()
            .find(|l| l.contains(key))
            .and_then(|l| l.split(':').nth(1))
            .map(|s| s.trim().to_string())
    };

    vec![
        {
            let value = run_cmd("diskutil", &["info", "/"])
                .and_then(|s| {
                    s.lines()
                        .find(|l| l.contains("Volume UUID") || l.contains("Disk / Partition UUID"))
                        .and_then(|l| l.split(':').nth(1))
                        .map(|v| v.trim().to_string())
                });
            IdentifierInfo {
                id: "disk_serial".into(),
                label: "硬盘标识".into(),
                value: or_unknown(value),
                modifiable: false,
                description: "主硬盘卷标识符".into(),
            }
        },
        IdentifierInfo {
            id: "motherboard_serial".into(),
            label: "序列号".into(),
            value: or_unknown(extract("Serial Number")),
            modifiable: false,
            description: "Mac 硬件序列号".into(),
        },
        IdentifierInfo {
            id: "bios_serial".into(),
            label: "Boot ROM 版本".into(),
            value: or_unknown(extract("Boot ROM Version")),
            modifiable: false,
            description: "Boot ROM 固件版本".into(),
        },
        IdentifierInfo {
            id: "cpu_id".into(),
            label: "CPU 信息".into(),
            value: or_unknown(extract("Chip").or_else(|| extract("Processor Name"))),
            modifiable: false,
            description: "处理器型号信息".into(),
        },
    ]
}
