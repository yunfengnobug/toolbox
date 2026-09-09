use serde::{Deserialize, Serialize};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use sysinfo::{Components, System};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x08000000;
const MIN_FAN_PERCENT: u32 = 20;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SensorInfo {
    pub name: String,
    pub temperature: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FanInfo {
    pub id: String,
    pub name: String,
    pub rpm: Option<u32>,
    pub percent: Option<u32>,
    pub controllable: bool,
    pub source: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GpuInfo {
    pub name: String,
    pub temperature: Option<f32>,
    pub fan_percent: Option<u32>,
    pub usage: Option<f32>,
    pub power_watt: Option<f32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CoolingState {
    pub mode: String,
    pub policy: String,
    pub processor_min: Option<u32>,
    pub processor_max: Option<u32>,
    pub boost: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ThermalStatus {
    pub cpu_name: String,
    pub cpu_usage: f32,
    pub cpu_temp: Option<f32>,
    pub memory_used: u64,
    pub memory_total: u64,
    pub sensors: Vec<SensorInfo>,
    pub fans: Vec<FanInfo>,
    pub gpus: Vec<GpuInfo>,
    pub cooling: CoolingState,
    pub portable: bool,
    pub modes_available: bool,
    pub notes: Vec<String>,
}

struct Monitor {
    sys: System,
    primed: bool,
}

fn monitor() -> &'static Mutex<Monitor> {
    static M: OnceLock<Mutex<Monitor>> = OnceLock::new();
    M.get_or_init(|| {
        let mut sys = System::new();
        sys.refresh_memory();
        sys.refresh_cpu_all();
        Mutex::new(Monitor { sys, primed: false })
    })
}

fn run_hidden(cmd: &str, args: &[&str]) -> Option<String> {
    let mut command = Command::new(cmd);
    command.args(args);
    #[cfg(target_os = "windows")]
    command.creation_flags(CREATE_NO_WINDOW);
    command.output().ok().and_then(|o| {
        if !o.status.success() && o.stdout.is_empty() {
            return None;
        }
        let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    })
}

#[cfg(target_os = "windows")]
fn powershell(script: &str) -> Option<String> {
    run_hidden(
        "powershell",
        &[
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ],
    )
}

fn value_as_array(v: &serde_json::Value) -> Vec<serde_json::Value> {
    match v {
        serde_json::Value::Array(a) => a.clone(),
        serde_json::Value::Object(_) => vec![v.clone()],
        _ => vec![],
    }
}

fn parse_num_f32(v: &serde_json::Value) -> Option<f32> {
    v.as_f64()
        .map(|n| n as f32)
        .or_else(|| v.as_u64().map(|n| n as f32))
        .or_else(|| v.as_i64().map(|n| n as f32))
        .or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
}

fn parse_num_u32(v: &serde_json::Value) -> Option<u32> {
    v.as_u64()
        .map(|n| n as u32)
        .or_else(|| v.as_i64().and_then(|n| u32::try_from(n).ok()))
        .or_else(|| v.as_f64().map(|n| n as u32))
        .or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
}

fn sysinfo_sensors() -> Vec<SensorInfo> {
    let mut sensors = Vec::new();
    let components = Components::new_with_refreshed_list();
    for component in &components {
        if let Some(temp) = component.temperature() {
            if temp.is_finite() && temp > 0.5 && temp < 150.0 {
                sensors.push(SensorInfo {
                    name: component.label().to_string(),
                    temperature: (temp * 10.0).round() / 10.0,
                });
            }
        }
    }
    sensors
}

fn pick_cpu_temp(sensors: &[SensorInfo]) -> Option<f32> {
    let preferred = sensors.iter().find(|s| {
        let n = s.name.to_lowercase();
        n.contains("cpu") || n.contains("package") || n.contains("tctl") || n.contains("tdie")
    });
    preferred
        .or_else(|| sensors.iter().max_by(|a, b| a.temperature.total_cmp(&b.temperature)))
        .map(|s| s.temperature)
}

fn nvidia_gpus() -> Vec<GpuInfo> {
    let output = run_hidden(
        "nvidia-smi",
        &[
            "--query-gpu=name,temperature.gpu,fan.speed,utilization.gpu,power.draw",
            "--format=csv,noheader,nounits",
        ],
    );
    let Some(output) = output else {
        return Vec::new();
    };

    output
        .lines()
        .filter_map(|line| {
            let cols: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            if cols.is_empty() || cols[0].is_empty() {
                return None;
            }
            let parse_opt = |idx: usize| {
                cols.get(idx)
                    .and_then(|s| {
                        if *s == "[N/A]" || s.eq_ignore_ascii_case("n/a") || s.is_empty() {
                            None
                        } else {
                            s.parse::<f32>().ok()
                        }
                    })
            };
            Some(GpuInfo {
                name: cols[0].to_string(),
                temperature: parse_opt(1),
                fan_percent: parse_opt(2).map(|n| n.round() as u32),
                usage: parse_opt(3),
                power_watt: parse_opt(4),
            })
        })
        .collect()
}

fn infer_mode(cooling: &CoolingState) -> String {
    match cooling.policy.as_str() {
        "passive" => "quiet".into(),
        "active" => {
            if cooling.boost == Some(2) || cooling.processor_min.unwrap_or(0) >= 15 {
                "performance".into()
            } else {
                "balanced".into()
            }
        }
        _ => "unknown".into(),
    }
}

pub fn get_status() -> ThermalStatus {
    let (cpu_name, cpu_usage, memory_used, memory_total) = {
        let mut guard = monitor().lock().unwrap_or_else(|e| e.into_inner());
        guard.sys.refresh_cpu_usage();
        guard.sys.refresh_memory();
        if !guard.primed {
            std::thread::sleep(Duration::from_millis(220));
            guard.sys.refresh_cpu_usage();
            guard.primed = true;
        }

        let cpu_name = guard
            .sys
            .cpus()
            .first()
            .map(|c| c.brand().trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "未知处理器".into());
        (
            cpu_name,
            guard.sys.global_cpu_usage(),
            guard.sys.used_memory(),
            guard.sys.total_memory(),
        )
    };

    let mut sensors = sysinfo_sensors();
    let mut fans = Vec::new();
    let mut notes = Vec::new();
    #[cfg_attr(target_os = "windows", allow(unused_assignments))]
    let mut portable = false;

    #[cfg(target_os = "windows")]
    {
        let extra = windows_probe();
        portable = extra.portable;
        for zone in extra.zones {
            if zone.temperature > 0.5 && zone.temperature < 150.0 {
                if !sensors.iter().any(|s| {
                    (s.temperature - zone.temperature).abs() < 0.8 && s.name == zone.name
                }) {
                    sensors.push(zone);
                }
            }
        }
        fans.extend(extra.fans);
    }

    #[cfg(target_os = "linux")]
    {
        let (hw_sensors, hw_fans) = linux_hwmon();
        for s in hw_sensors {
            if !sensors.iter().any(|x| x.name == s.name) {
                sensors.push(s);
            }
        }
        fans.extend(hw_fans);
    }

    let gpus = nvidia_gpus();
    for gpu in &gpus {
        if let Some(temp) = gpu.temperature {
            let name = format!("GPU {}", gpu.name);
            if !sensors.iter().any(|s| s.name == name) {
                sensors.push(SensorInfo {
                    name,
                    temperature: temp,
                });
            }
        }
        if let Some(percent) = gpu.fan_percent {
            fans.push(FanInfo {
                id: format!("nvidia:{}", gpu.name),
                name: format!("{} 风扇", gpu.name),
                rpm: None,
                percent: Some(percent),
                controllable: false,
                source: "nvidia".into(),
            });
        }
    }

    let mut cooling = current_cooling();
    cooling.mode = infer_mode(&cooling);

    if fans.iter().any(|f| f.controllable) {
        notes.push("已检测到可直接调速的风扇，可用滑条手动设置转速。".into());
    } else {
        notes.push("当前设备通常不能直接写入风扇转速（多数主板由 BIOS/EC 控制）。下面的散热模式会改变 Windows 散热策略，笔记本上能明显影响风扇表现。".into());
    }
    if portable {
        notes.push("检测到便携设备，散热模式对风扇的影响通常更明显。".into());
    }
    if sensors.is_empty() {
        notes.push("未能读取温度传感器，可尝试以管理员身份运行。".into());
    }

    ThermalStatus {
        cpu_temp: pick_cpu_temp(&sensors),
        cpu_name,
        cpu_usage,
        memory_used,
        memory_total,
        sensors,
        fans,
        gpus,
        cooling,
        portable,
        modes_available: cfg!(any(target_os = "windows", target_os = "linux")),
        notes,
    }
}

pub fn set_cooling_mode(mode: &str) -> Result<String, String> {
    match mode {
        "quiet" | "balanced" | "performance" => {}
        _ => return Err("未知的散热模式".into()),
    }

    #[cfg(target_os = "windows")]
    {
        return windows_set_mode(mode);
    }

    #[cfg(target_os = "linux")]
    {
        return linux_set_mode(mode);
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        let _ = mode;
        Err("当前系统不支持软件调节风扇，请使用系统电源/散热设置".into())
    }
}

pub fn set_fan_percent(fan_id: &str, percent: u32) -> Result<String, String> {
    let percent = percent.clamp(MIN_FAN_PERCENT, 100);

    if let Some(rest) = fan_id.strip_prefix("wmi:") {
        #[cfg(target_os = "windows")]
        {
            return windows_set_fan(Some(rest), percent);
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = rest;
        }
    }

    if let Some(rest) = fan_id.strip_prefix("hwmon:") {
        #[cfg(target_os = "linux")]
        {
            return linux_set_pwm(rest, percent);
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = rest;
        }
    }

    Err("该风扇不支持软件调速".into())
}

fn current_cooling() -> CoolingState {
    #[cfg(target_os = "windows")]
    {
        return windows_cooling_state();
    }
    #[cfg(not(target_os = "windows"))]
    {
        CoolingState {
            mode: "unknown".into(),
            policy: "unknown".into(),
            processor_min: None,
            processor_max: None,
            boost: None,
        }
    }
}

#[cfg(target_os = "windows")]
struct WindowsProbe {
    fans: Vec<FanInfo>,
    zones: Vec<SensorInfo>,
    portable: bool,
}

#[cfg(target_os = "windows")]
fn windows_probe() -> WindowsProbe {
    let script = r#"
$ErrorActionPreference = 'SilentlyContinue'
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$fans = @()
Get-CimInstance Win32_Fan | ForEach-Object {
  $speed = 0
  try { $speed = [uint32]$_.DesiredSpeed } catch {}
  $fans += @{
    deviceId = [string]$_.DeviceID
    name = [string]$(if ($_.Name) { $_.Name } else { $_.DeviceID })
    desiredSpeed = $speed
    status = [string]$_.Status
    variableSpeed = [bool]$_.VariableSpeed
  }
}
$zones = @()
Get-CimInstance -Namespace root/wmi -ClassName MSAcpi_ThermalZoneTemperature | ForEach-Object {
  $temp = [math]::Round(($_.CurrentTemperature / 10.0) - 273.15, 1)
  $zones += @{ name = [string]$_.InstanceName; temp = $temp }
}
Get-CimInstance Win32_PerfFormattedData_Counters_ThermalZoneInformation | ForEach-Object {
  if ($_.Temperature) {
    $temp = [math]::Round(([double]$_.Temperature - 273.15), 1)
    $zones += @{ name = [string]$(if ($_.Name) { $_.Name } else { 'Thermal' }); temp = $temp }
  }
}
$portable = $false
Get-CimInstance Win32_SystemEnclosure | ForEach-Object {
  foreach ($t in @($_.ChassisTypes)) {
    if ($t -in 8,9,10,14,30,31,32) { $portable = $true }
  }
}
@{ fans = $fans; zones = $zones; portable = $portable } | ConvertTo-Json -Compress -Depth 6
"#;

    let mut probe = WindowsProbe {
        fans: Vec::new(),
        zones: Vec::new(),
        portable: false,
    };

    let Some(raw) = powershell(script) else {
        return probe;
    };
    let json_str = raw
        .find('{')
        .map(|i| &raw[i..])
        .unwrap_or(raw.as_str());
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) else {
        return probe;
    };

    probe.portable = value
        .get("portable")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if let Some(fans) = value.get("fans") {
        for (idx, fan) in value_as_array(fans).iter().enumerate() {
            let device_id = fan
                .get("deviceId")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let name = fan
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("系统风扇")
                .trim()
                .to_string();
            let speed = fan
                .get("desiredSpeed")
                .and_then(parse_num_u32)
                .filter(|n| *n > 0);
            let (rpm, percent) = match speed {
                Some(v) if v > 100 => (Some(v), None),
                Some(v) => (None, Some(v)),
                None => (None, None),
            };
            let controllable = fan
                .get("variableSpeed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let id = if device_id.is_empty() {
                format!("wmi:{idx}")
            } else {
                format!("wmi:{device_id}")
            };
            probe.fans.push(FanInfo {
                id,
                name: if name.is_empty() {
                    format!("风扇 {}", idx + 1)
                } else {
                    name
                },
                rpm,
                percent,
                controllable,
                source: "wmi".into(),
            });
        }
    }

    if let Some(zones) = value.get("zones") {
        for zone in value_as_array(zones) {
            let name = zone
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("热区")
                .trim()
                .to_string();
            if let Some(temp) = zone.get("temp").and_then(parse_num_f32) {
                probe.zones.push(SensorInfo { name, temperature: temp });
            }
        }
    }

    probe
}

#[cfg(target_os = "windows")]
fn windows_cooling_state() -> CoolingState {
    use winreg::enums::*;
    use winreg::RegKey;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let schemes_path = "SYSTEM\\CurrentControlSet\\Control\\Power\\User\\PowerSchemes";
    let active = hklm
        .open_subkey(schemes_path)
        .ok()
        .and_then(|k| k.get_value::<String, _>("ActivePowerScheme").ok());

    let Some(active) = active else {
        return CoolingState {
            mode: "unknown".into(),
            policy: "unknown".into(),
            processor_min: None,
            processor_max: None,
            boost: None,
        };
    };

    let cpu_guid = "54533251-82be-4824-96c1-47b60b740d00";
    let read_setting = |setting: &str| -> Option<u32> {
        let path = format!("{schemes_path}\\{active}\\{cpu_guid}\\{setting}");
        hklm.open_subkey(path)
            .ok()
            .and_then(|k| k.get_value::<u32, _>("ACSettingIndex").ok())
    };

    let policy_raw = read_setting("94d3a615-a899-4acd-ae73-12a1a0f7f64c");
    let policy = match policy_raw {
        Some(0) => "passive",
        Some(1) => "active",
        _ => "unknown",
    };

    CoolingState {
        mode: "unknown".into(),
        policy: policy.into(),
        processor_min: read_setting("893dee8e-2bef-41e0-89c6-b61131a210bd"),
        processor_max: read_setting("bc5038f7-23e0-4960-96da-33abaf5935ec"),
        boost: read_setting("be337238-0d82-4146-a960-4f3749d470c7"),
    }
}

#[cfg(target_os = "windows")]
fn powercfg_run(args: &[&str]) -> Result<(), String> {
    let mut cmd = Command::new("powercfg");
    cmd.args(args);
    cmd.creation_flags(CREATE_NO_WINDOW);
    let status = cmd
        .status()
        .map_err(|e| format!("无法执行 powercfg: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err("设置电源选项失败，请以管理员身份运行".into())
    }
}

#[cfg(target_os = "windows")]
fn powercfg_set(sub: &str, setting: &str, value: &str) -> Result<(), String> {
    powercfg_run(&[
        "/setacvalueindex",
        "SCHEME_CURRENT",
        sub,
        setting,
        value,
    ])?;
    let _ = powercfg_run(&[
        "/setdcvalueindex",
        "SCHEME_CURRENT",
        sub,
        setting,
        value,
    ]);
    Ok(())
}

#[cfg(target_os = "windows")]
fn windows_set_mode(mode: &str) -> Result<String, String> {
    let (cool, max, min, boost, fan_percent) = match mode {
        "quiet" => (0u32, 85u32, 5u32, 1u32, Some(35u32)),
        "balanced" => (1, 100, 5, 1, None),
        "performance" => (1, 100, 20, 2, Some(100)),
        _ => return Err("未知的散热模式".into()),
    };

    powercfg_set("SUB_PROCESSOR", "SYSCOOLPOL", &cool.to_string())?;
    let _ = powercfg_set("SUB_PROCESSOR", "PROCTHROTTLEMAX", &max.to_string());
    let _ = powercfg_set("SUB_PROCESSOR", "PROCTHROTTLEMIN", &min.to_string());
    let _ = powercfg_set("SUB_PROCESSOR", "PERFBOOSTMODE", &boost.to_string());
    powercfg_run(&["/setactive", "SCHEME_CURRENT"])
        .map_err(|_| "应用电源方案失败，请以管理员身份运行".to_string())?;

    if let Some(percent) = fan_percent {
        let _ = windows_set_fan(None, percent);
    }

    let label = match mode {
        "quiet" => "静音",
        "performance" => "性能",
        _ => "均衡",
    };
    Ok(format!("已切换为{label}模式"))
}

#[cfg(target_os = "windows")]
fn windows_set_fan(device_id: Option<&str>, percent: u32) -> Result<String, String> {
    let filter = match device_id {
        Some(id) if !id.is_empty() => format!(
            "Where-Object {{ $_.DeviceID -eq '{}' -or $_.DeviceID -eq 'wmi:{}' }}",
            id.replace('\'', "''"),
            id.replace('\'', "''")
        ),
        _ => "ForEach-Object { $_ }".into(),
    };

    // Win32_Fan.SetSpeed 的 DesiredSpeed 在实现它的机器上通常是 0-100 的百分比
    let script = format!(
        r#"
$ErrorActionPreference = 'SilentlyContinue'
$percent = [uint32]{percent}
$ok = 0
$err = ''
$fans = @(Get-CimInstance Win32_Fan | {filter})
foreach ($fan in $fans) {{
  try {{
    Invoke-CimMethod -InputObject $fan -MethodName SetSpeed -Arguments @{{ DesiredSpeed = $percent }} | Out-Null
    $ok++
  }} catch {{
    try {{
      $fan.SetSpeed($percent) | Out-Null
      $ok++
    }} catch {{
      $err = $_.Exception.Message
    }}
  }}
}}
@{{ ok = $ok; error = $err; total = $fans.Count }} | ConvertTo-Json -Compress
"#
    );

    let raw = powershell(&script).ok_or_else(|| "无法调用风扇控制接口".to_string())?;
    let json_str = raw.find('{').map(|i| &raw[i..]).unwrap_or(raw.as_str());
    let value: serde_json::Value =
        serde_json::from_str(json_str).map_err(|_| "风扇控制接口返回异常".to_string())?;
    let ok = value.get("ok").and_then(parse_num_u32).unwrap_or(0);
    if ok > 0 {
        Ok(format!("已将风扇转速设为 {percent}%"))
    } else {
        Err("主板未实现风扇调速接口，已改用散热策略（若刚才切换了模式）".into())
    }
}

#[cfg(target_os = "linux")]
fn linux_hwmon() -> (Vec<SensorInfo>, Vec<FanInfo>) {
    let mut sensors = Vec::new();
    let mut fans = Vec::new();
    let Ok(entries) = std::fs::read_dir("/sys/class/hwmon") else {
        return (sensors, fans);
    };

    for entry in entries.flatten() {
        let dir = entry.path();
        let chip = std::fs::read_to_string(dir.join("name"))
            .unwrap_or_else(|_| "hwmon".into())
            .trim()
            .to_string();

        if let Ok(files) = std::fs::read_dir(&dir) {
            let mut names: Vec<String> = files
                .flatten()
                .filter_map(|f| f.file_name().into_string().ok())
                .collect();
            names.sort();

            for name in &names {
                if let Some(idx) = name.strip_prefix("temp").and_then(|s| s.strip_suffix("_input")) {
                    if let Ok(raw) = std::fs::read_to_string(dir.join(name)) {
                        if let Ok(milli) = raw.trim().parse::<f32>() {
                            let label = std::fs::read_to_string(dir.join(format!("temp{idx}_label")))
                                .ok()
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .unwrap_or_else(|| format!("{chip} temp{idx}"));
                            sensors.push(SensorInfo {
                                name: label,
                                temperature: ((milli / 1000.0) * 10.0).round() / 10.0,
                            });
                        }
                    }
                }

                if let Some(idx) = name.strip_prefix("fan").and_then(|s| s.strip_suffix("_input")) {
                    let rpm = std::fs::read_to_string(dir.join(name))
                        .ok()
                        .and_then(|s| s.trim().parse::<u32>().ok())
                        .filter(|n| *n > 0);
                    let pwm_path = dir.join(format!("pwm{idx}"));
                    let percent = std::fs::read_to_string(&pwm_path)
                        .ok()
                        .and_then(|s| s.trim().parse::<u32>().ok())
                        .map(|v| ((v as f32) * 100.0 / 255.0).round() as u32);
                    let controllable = pwm_path.exists()
                        && std::fs::OpenOptions::new()
                            .write(true)
                            .open(&pwm_path)
                            .is_ok();
                    let label = std::fs::read_to_string(dir.join(format!("fan{idx}_label")))
                        .ok()
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| format!("{chip} fan{idx}"));
                    fans.push(FanInfo {
                        id: format!("hwmon:{chip}:{idx}"),
                        name: label,
                        rpm,
                        percent,
                        controllable,
                        source: "hwmon".into(),
                    });
                }
            }
        }
    }

    (sensors, fans)
}

#[cfg(target_os = "linux")]
fn linux_set_pwm(spec: &str, percent: u32) -> Result<String, String> {
    let mut parts = spec.splitn(2, ':');
    let chip = parts.next().ok_or("风扇标识无效")?;
    let idx = parts.next().ok_or("风扇标识无效")?;
    let percent = percent.clamp(MIN_FAN_PERCENT, 100);
    let pwm = (percent as u32 * 255 / 100).min(255);

    let Ok(entries) = std::fs::read_dir("/sys/class/hwmon") else {
        return Err("找不到 hwmon 设备".into());
    };
    for entry in entries.flatten() {
        let dir = entry.path();
        let name = std::fs::read_to_string(dir.join("name")).unwrap_or_default();
        if name.trim() != chip {
            continue;
        }
        let enable = dir.join(format!("pwm{idx}_enable"));
        let pwm_path = dir.join(format!("pwm{idx}"));
        let _ = std::fs::write(&enable, "1");
        std::fs::write(&pwm_path, pwm.to_string()).map_err(|e| {
            format!("写入转速失败: {e}。请使用 sudo 运行，并确认该风扇支持 PWM")
        })?;
        return Ok(format!("已将风扇设为 {percent}%"));
    }
    Err("未找到对应的 hwmon 风扇".into())
}

#[cfg(target_os = "linux")]
fn linux_set_mode(mode: &str) -> Result<String, String> {
    let target = match mode {
        "quiet" => 35u32,
        "balanced" => {
            return linux_set_auto()
        }
        "performance" => 100,
        _ => return Err("未知的散热模式".into()),
    };

    let (_, fans) = linux_hwmon();
    let controllable: Vec<_> = fans.into_iter().filter(|f| f.controllable).collect();
    if controllable.is_empty() {
        return Err("没有可写的 PWM 风扇，请用 sudo 运行或检查 hwmon 权限".into());
    }
    let mut ok = 0;
    let mut last_err = String::new();
    for fan in &controllable {
        if let Some(spec) = fan.id.strip_prefix("hwmon:") {
            match linux_set_pwm(spec, target) {
                Ok(_) => ok += 1,
                Err(e) => last_err = e,
            }
        }
    }
    if ok == 0 {
        Err(last_err)
    } else {
        Ok(format!("已将 {ok} 个风扇设为 {target}%"))
    }
}

#[cfg(target_os = "linux")]
fn linux_set_auto() -> Result<String, String> {
    let Ok(entries) = std::fs::read_dir("/sys/class/hwmon") else {
        return Err("找不到 hwmon 设备".into());
    };
    let mut ok = 0;
    for entry in entries.flatten() {
        let dir = entry.path();
        if let Ok(files) = std::fs::read_dir(&dir) {
            for file in files.flatten() {
                let name = file.file_name().to_string_lossy().to_string();
                if name.starts_with("pwm") && name.ends_with("_enable") {
                    if std::fs::write(file.path(), "2").is_ok() {
                        ok += 1;
                    }
                }
            }
        }
    }
    if ok == 0 {
        Err("无法恢复自动调速，请使用 sudo 运行".into())
    } else {
        Ok("已恢复 BIOS/驱动自动调速".into())
    }
}
