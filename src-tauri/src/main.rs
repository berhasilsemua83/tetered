// main.rs
// Backend Tauri: menyimpan & membaca konfigurasi (kredensial, toggle, folder)
// ke file config.json di app data directory. Frontend (React) memanggil
// command ini lewat invoke("save_config") / invoke("load_config").

use serde::{Deserialize, Serialize};
use std::fs;
use tauri::Manager;

// ====== STRUKTUR DATA CONFIG ======
// Setiap field diberi nama jelas supaya tidak ketuker saat disimpan/dibaca.

#[derive(Serialize, Deserialize, Clone, Default)]
struct CloudinaryConfig {
    cloud_name: String,
    api_key: String,
    api_secret: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
struct AppConfig {
    threads_user_id: String,
    threads_access_token: String,
    cloudinary: CloudinaryConfig,
    // Daftar Gemini API key, bisa lebih dari satu (rotasi kalau kuota habis)
    gemini_api_keys: Vec<String>,
    // Toggle utama: aktif/nonaktifkan balasan komentar otomatis pakai AI
    ai_reply_enabled: bool,
    queue_folder: String,
    posted_folder: String,
}

// Lokasi file config.json disimpan (folder data aplikasi milik OS,
// otomatis dibuat kalau belum ada)
fn config_file_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Gagal menemukan folder data aplikasi: {e}"))?;

    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| format!("Gagal membuat folder data: {e}"))?;
    }

    Ok(dir.join("config.json"))
}

#[tauri::command]
fn load_config(app: tauri::AppHandle) -> Result<AppConfig, String> {
    let path = config_file_path(&app)?;

    if !path.exists() {
        // Belum pernah disimpan sebelumnya -> kembalikan config kosong (default)
        return Ok(AppConfig::default());
    }

    let content = fs::read_to_string(&path).map_err(|e| format!("Gagal membaca config: {e}"))?;
    serde_json::from_str(&content).map_err(|e| format!("Config rusak/tidak valid: {e}"))
}

#[tauri::command]
fn save_config(app: tauri::AppHandle, config: AppConfig) -> Result<(), String> {
    let path = config_file_path(&app)?;

    let content = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Gagal mengubah config jadi JSON: {e}"))?;

    fs::write(&path, content).map_err(|e| format!("Gagal menyimpan config: {e}"))?;

    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![load_config, save_config])
        .run(tauri::generate_context!())
        .expect("Gagal menjalankan aplikasi Tauri");
}
