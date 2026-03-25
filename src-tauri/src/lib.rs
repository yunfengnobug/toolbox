mod hardware;
mod modifier;

use hardware::IdentifierInfo;
use tauri::Manager;

#[tauri::command]
fn check_admin() -> bool {
    is_elevated()
}

#[cfg(target_os = "windows")]
fn is_elevated() -> bool {
    use winreg::enums::*;
    use winreg::RegKey;
    RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags("SOFTWARE\\Microsoft\\Cryptography", KEY_SET_VALUE)
        .is_ok()
}

#[cfg(not(target_os = "windows"))]
fn is_elevated() -> bool {
    unsafe { libc_geteuid() == 0 }
}

#[cfg(not(target_os = "windows"))]
extern "C" {
    #[link_name = "geteuid"]
    fn libc_geteuid() -> u32;
}

#[tauri::command]
fn get_all_identifiers() -> Result<Vec<IdentifierInfo>, String> {
    Ok(hardware::get_all())
}

#[tauri::command]
fn modify_identifier(id: String, new_value: String) -> Result<(), String> {
    modifier::modify(&id, &new_value)
}

#[tauri::command]
fn generate_random_value(id: String) -> Result<String, String> {
    modifier::generate_random(&id)
}

#[tauri::command]
fn backup_identifiers(app: tauri::AppHandle) -> Result<String, String> {
    let identifiers = hardware::get_all();
    let json = serde_json::to_string_pretty(&identifiers).map_err(|e| e.to_string())?;

    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;

    let backup_path = data_dir.join("backup.json");
    std::fs::write(&backup_path, &json).map_err(|e| e.to_string())?;

    Ok(backup_path.to_string_lossy().to_string())
}

#[tauri::command]
fn restore_identifiers(app: tauri::AppHandle) -> Result<(), String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let backup_path = data_dir.join("backup.json");

    if !backup_path.exists() {
        return Err("没有找到备份文件，请先执行备份操作".to_string());
    }

    let json = std::fs::read_to_string(&backup_path).map_err(|e| e.to_string())?;
    let identifiers: Vec<IdentifierInfo> =
        serde_json::from_str(&json).map_err(|e| e.to_string())?;

    let mut errors = Vec::new();
    for id_info in identifiers {
        if id_info.modifiable {
            if let Err(e) = modifier::modify(&id_info.id, &id_info.value) {
                errors.push(format!("{}: {}", id_info.label, e));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("部分还原失败:\n{}", errors.join("\n")))
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            check_admin,
            get_all_identifiers,
            modify_identifier,
            generate_random_value,
            backup_identifiers,
            restore_identifiers,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
