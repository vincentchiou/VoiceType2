// VoiceType - Windows auto-start via Registry Run key
// HKCU\Software\Microsoft\Windows\CurrentVersion\Run\VoiceType2 = "<exe path>"

use winreg::{enums::*, RegKey};

const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const APP_NAME: &str = "VoiceType2";

fn run_key_write() -> Result<RegKey, String> {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(RUN_KEY, KEY_WRITE)
        .map_err(|e| format!("開啟登錄檔失敗：{}", e))
}

fn current_exe_quoted() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| format!("取得程式路徑失敗：{}", e))?;
    Ok(format!("\"{}\"", exe.display()))
}

/// Enable or disable auto-start on Windows login.
pub fn set_autostart(enabled: bool) -> Result<(), String> {
    let run = run_key_write()?;
    if enabled {
        let exe = current_exe_quoted()?;
        run.set_value(APP_NAME, &exe)
            .map_err(|e| format!("寫入開機啟動失敗：{}", e))?;
        eprintln!("[Autostart] enabled: {}", exe);
    } else {
        match run.delete_value(APP_NAME) {
            Ok(_) => eprintln!("[Autostart] disabled"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                eprintln!("[Autostart] already disabled")
            }
            Err(e) => return Err(format!("移除開機啟動失敗：{}", e)),
        }
    }
    Ok(())
}

/// Check whether auto-start is currently enabled for this exe path.
pub fn get_autostart() -> bool {
    let exe = match current_exe_quoted() {
        Ok(e) => e,
        Err(_) => return false,
    };
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run = match hkcu.open_subkey_with_flags(RUN_KEY, KEY_READ) {
        Ok(k) => k,
        Err(_) => return false,
    };
    let stored: Result<String, _> = run.get_value(APP_NAME);
    matches!(stored, Ok(v) if v == exe)
}
