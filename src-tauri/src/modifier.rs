use rand::Rng;
use uuid::Uuid;

#[allow(unused_imports)]
use std::process::Command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

pub fn modify(id: &str, new_value: &str) -> Result<(), String> {
    match id {
        "mac_address" => modify_mac(new_value),
        "machine_guid" => modify_guid(new_value),
        _ => Err(format!("\"{}\" 不支持修改", id)),
    }
}

pub fn generate_random(id: &str) -> Result<String, String> {
    match id {
        "mac_address" => Ok(random_mac()),
        "machine_guid" => Ok(random_guid()),
        _ => Err(format!("\"{}\" 不支持随机生成", id)),
    }
}

fn random_mac() -> String {
    let mut rng = rand::thread_rng();
    let b: Vec<u8> = (0..6)
        .map(|i| {
            if i == 0 {
                (rng.gen::<u8>() & 0xFE) | 0x02
            } else {
                rng.gen()
            }
        })
        .collect();
    format!(
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        b[0], b[1], b[2], b[3], b[4], b[5]
    )
}

fn random_guid() -> String {
    Uuid::new_v4().to_string()
}

fn validate_mac(mac: &str) -> Result<String, String> {
    let clean = mac.replace([':', '-', ' '], "").to_uppercase();
    if clean.len() != 12 || !clean.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("MAC 地址格式无效，请使用 XX:XX:XX:XX:XX:XX 格式".into());
    }
    Ok(clean)
}

#[allow(dead_code)]
fn format_mac_colons(clean: &str) -> String {
    clean
        .as_bytes()
        .chunks(2)
        .map(|c| std::str::from_utf8(c).unwrap_or("00"))
        .collect::<Vec<&str>>()
        .join(":")
}

// ==================== Windows ====================

#[cfg(target_os = "windows")]
fn run_powershell(script: &str) -> Result<(), String> {
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("执行 PowerShell 失败: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "操作失败（可能需要管理员权限）: {}",
            stderr.trim()
        ))
    }
}

#[cfg(target_os = "windows")]
fn modify_mac(new_mac: &str) -> Result<(), String> {
    let mac_clean = validate_mac(new_mac)?;

    let script = format!(
        r#"
$adapter = Get-NetAdapter -Physical | Where-Object Status -eq 'Up' | Select-Object -First 1
if (-not $adapter) {{ throw '未找到活跃的物理网络适配器' }}
$regPath = 'HKLM:\SYSTEM\CurrentControlSet\Control\Class\{{4D36E972-E325-11CE-BFC1-08002BE10318}}'
Get-ChildItem $regPath -ErrorAction SilentlyContinue | ForEach-Object {{
    $id = (Get-ItemProperty $_.PSPath -Name 'NetCfgInstanceId' -ErrorAction SilentlyContinue).NetCfgInstanceId
    if ($id -eq $adapter.InterfaceGuid) {{
        Set-ItemProperty $_.PSPath -Name 'NetworkAddress' -Value '{mac}'
    }}
}}
Restart-NetAdapter -Name $adapter.Name -Confirm:$false
"#,
        mac = mac_clean
    );

    run_powershell(&script)
}

#[cfg(target_os = "windows")]
fn modify_guid(new_guid: &str) -> Result<(), String> {
    use winreg::enums::*;
    use winreg::RegKey;

    RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags("SOFTWARE\\Microsoft\\Cryptography", KEY_SET_VALUE)
        .and_then(|key| key.set_value("MachineGuid", &new_guid))
        .map_err(|e| format!("修改 Machine GUID 失败（需要管理员权限）: {}", e))
}

// ==================== Linux ====================

#[cfg(target_os = "linux")]
fn modify_mac(new_mac: &str) -> Result<(), String> {
    let mac_clean = validate_mac(new_mac)?;
    let mac_formatted = format_mac_colons(&mac_clean);

    let iface = Command::new("ip")
        .args(["route", "show", "default"])
        .output()
        .ok()
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout)
                .split_whitespace()
                .skip_while(|&w| w != "dev")
                .nth(1)
                .map(|s| s.to_string())
        })
        .ok_or_else(|| "无法确定主网络接口".to_string())?;

    let script = format!(
        "ip link set dev {iface} down && ip link set dev {iface} address {mac} && ip link set dev {iface} up",
        iface = iface,
        mac = mac_formatted
    );

    let output = Command::new("sh")
        .args(["-c", &script])
        .output()
        .map_err(|e| format!("执行命令失败: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "修改 MAC 地址失败（需要 root 权限）: {}",
            stderr.trim()
        ))
    }
}

#[cfg(target_os = "linux")]
fn modify_guid(new_guid: &str) -> Result<(), String> {
    let clean = new_guid.replace('-', "");
    std::fs::write("/etc/machine-id", format!("{}\n", clean))
        .map_err(|e| format!("修改 Machine ID 失败（需要 root 权限）: {}", e))?;

    let _ = std::fs::write("/var/lib/dbus/machine-id", format!("{}\n", clean));
    Ok(())
}

// ==================== macOS ====================

#[cfg(target_os = "macos")]
fn modify_mac(new_mac: &str) -> Result<(), String> {
    let mac_clean = validate_mac(new_mac)?;
    let mac_formatted = format_mac_colons(&mac_clean);

    let output = Command::new("sudo")
        .args(["ifconfig", "en0", "ether", &mac_formatted])
        .output()
        .map_err(|e| format!("执行命令失败: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "修改 MAC 地址失败（需要 sudo 权限）: {}",
            stderr.trim()
        ))
    }
}

#[cfg(target_os = "macos")]
fn modify_guid(_new_guid: &str) -> Result<(), String> {
    Err("macOS 不支持软件方式修改 Hardware UUID".into())
}
