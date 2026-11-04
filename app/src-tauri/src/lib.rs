mod ccid;
mod fido;
mod phy;
mod rescue;

use phy::Phy;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

type CmdResult<T> = Result<T, String>;

/// Run blocking key I/O off the main thread. Errors go to the UI as text.
async fn blocking<T, F>(f: F) -> CmdResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> anyhow::Result<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("{e:#}"))
}

#[derive(Serialize)]
struct AppStatus {
    admin: bool,
    platform: &'static str,
}

#[tauri::command]
fn app_status() -> AppStatus {
    AppStatus { admin: fido::is_admin(), platform: std::env::consts::OS }
}

#[tauri::command]
async fn device_info() -> CmdResult<rescue::DeviceInfo> {
    blocking(rescue::device_info).await
}

#[tauri::command]
async fn firmware_version() -> CmdResult<String> {
    blocking(rescue::management_version).await
}

#[tauri::command]
async fn read_phy() -> CmdResult<Phy> {
    blocking(rescue::read_phy).await
}

#[tauri::command]
async fn write_phy(phy: Phy) -> CmdResult<()> {
    blocking(move || rescue::write_phy(&phy)).await
}

#[tauri::command]
async fn reboot(to_bootsel: bool) -> CmdResult<()> {
    blocking(move || rescue::reboot(to_bootsel)).await
}

#[tauri::command]
async fn fido_info() -> CmdResult<fido::FidoInfo> {
    blocking(fido::info).await
}

#[tauri::command]
async fn fido_set_pin(new_pin: String) -> CmdResult<()> {
    blocking(move || fido::set_pin(&new_pin)).await
}

#[tauri::command]
async fn fido_change_pin(current_pin: String, new_pin: String) -> CmdResult<()> {
    blocking(move || fido::change_pin(&current_pin, &new_pin)).await
}

#[tauri::command]
async fn fido_list_passkeys(pin: String) -> CmdResult<Vec<fido::Passkey>> {
    blocking(move || fido::list_passkeys(&pin)).await
}

#[tauri::command]
async fn fido_delete_passkey(pin: String, credential_id: String) -> CmdResult<()> {
    blocking(move || fido::delete_passkey(&pin, &credential_id)).await
}

/// Restart this program with administrator rights (Windows UAC).
#[tauri::command]
fn relaunch_as_admin(app: tauri::AppHandle) -> CmdResult<()> {
    #[cfg(windows)]
    {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let status = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-WindowStyle",
                "Hidden",
                "-Command",
                &format!("Start-Process -FilePath '{}' -Verb RunAs", exe.display()),
            ])
            .status()
            .map_err(|e| e.to_string())?;
        if !status.success() {
            return Err("Перезапуск отменён.".into());
        }
        app.exit(0);
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        Err("Функция есть только в Windows.".into())
    }
}

fn find_bootsel_drive() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        for letter in b'D'..=b'Z' {
            let root = PathBuf::from(format!("{}:\\", letter as char));
            if let Ok(s) = std::fs::read_to_string(root.join("INFO_UF2.TXT")) {
                if s.contains("RPI-RP2") || s.contains("RP2350") {
                    return Some(root);
                }
            }
        }
        None
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// Reboot the key to BOOTSEL (if needed) and copy a .uf2 file to the RPI-RP2 drive.
#[tauri::command]
async fn flash_firmware(path: String) -> CmdResult<String> {
    blocking(move || {
        let src = Path::new(&path);
        if src.extension().and_then(|e| e.to_str()) != Some("uf2") {
            anyhow::bail!("Выберите файл с расширением .uf2.");
        }
        if find_bootsel_drive().is_none() {
            rescue::reboot(true)?;
        }
        let start = Instant::now();
        let drive = loop {
            if let Some(d) = find_bootsel_drive() {
                break d;
            }
            if start.elapsed() > Duration::from_secs(20) {
                anyhow::bail!(
                    "Диск RPI-RP2 не появился. Нажмите и удерживайте BOOT, нажмите RST, отпустите BOOT."
                );
            }
            std::thread::sleep(Duration::from_millis(500));
        };
        std::thread::sleep(Duration::from_millis(800));
        let name = src.file_name().unwrap_or_default();
        std::fs::copy(src, drive.join(name))?;
        Ok(format!("Файл записан на {}. Ключ перезагружается.", drive.display()))
    })
    .await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            app_status,
            device_info,
            firmware_version,
            read_phy,
            write_phy,
            reboot,
            fido_info,
            fido_set_pin,
            fido_change_pin,
            fido_list_passkeys,
            fido_delete_passkey,
            relaunch_as_admin,
            flash_firmware,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
