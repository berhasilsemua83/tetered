// main.rs
// Backend Tauri: menyimpan config + mengatur Windows Task Scheduler
// lewat command line "schtasks.exe" (bawaan Windows, tidak perlu library
// tambahan). App ini TIDAK perlu nyala terus - begitu jadwal diterapkan,
// Windows Task Scheduler yang menjalankan tugasnya secara mandiri.

use serde::{Deserialize, Serialize};
use std::fs;
use std::process::Command;
use tauri::Manager;

// ====== STRUKTUR DATA CONFIG ======

#[derive(Serialize, Deserialize, Clone, Default)]
struct CloudinaryConfig {
    cloud_name: String,
    api_key: String,
    api_secret: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
struct ScheduleConfig {
    // Daftar jam posting utama, format 24 jam "HH:MM", contoh: ["07:00","11:00","15:00"]
    thread_poster_times: Vec<String>,
    // Interval dalam menit
    reply_checker_interval_minutes: u32,
    comment_responder_interval_minutes: u32,
    // Hari (MON/TUE/WED/THU/FRI/SAT/SUN) + jam "HH:MM"
    refresh_token_day: String,
    refresh_token_time: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
struct AppConfig {
    threads_user_id: String,
    threads_access_token: String,
    cloudinary: CloudinaryConfig,
    gemini_api_keys: Vec<String>,
    ai_reply_enabled: bool,
    queue_folder: String,
    posted_folder: String,
    // BARU: lokasi node.exe dan folder proyek (tempat file .js berada)
    node_exe_path: String,
    project_folder: String,
    schedule: ScheduleConfig,
}

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
        return Ok(AppConfig::default());
    }

    let content = fs::read_to_string(&path).map_err(|e| format!("Gagal membaca config: {e}"))?;
    serde_json::from_str(&content).map_err(|e| format!("Config rusak/tidak valid: {e}"))
}

fn write_config_to_disk(app: &tauri::AppHandle, config: &AppConfig) -> Result<(), String> {
    let path = config_file_path(app)?;
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Gagal mengubah config jadi JSON: {e}"))?;
    fs::write(&path, content).map_err(|e| format!("Gagal menyimpan config: {e}"))
}

#[tauri::command]
fn save_config(app: tauri::AppHandle, config: AppConfig) -> Result<(), String> {
    write_config_to_disk(&app, &config)
}

// ====================================================
// MANAJEMEN TASK SCHEDULER LEWAT schtasks.exe
// ====================================================

const MAX_THREAD_POSTER_SLOTS: u32 = 30; // batas aman jumlah slot yang pernah dibuat

fn task_name_thread_poster(index: u32) -> String {
    format!("ThreadsAutomator_ThreadPoster_{index}")
}
const TASK_NAME_REPLY_CHECKER: &str = "ThreadsAutomator_ReplyChecker";
const TASK_NAME_COMMENT_RESPONDER: &str = "ThreadsAutomator_CommentResponder";
const TASK_NAME_REFRESH_TOKEN: &str = "ThreadsAutomator_RefreshToken";

// Jalankan schtasks.exe dengan argumen tertentu, kembalikan pesan hasil (untuk log)
fn run_schtasks(args: &[&str]) -> String {
    let output = Command::new("schtasks").args(args).output();

    match output {
        Ok(out) => {
            if out.status.success() {
                format!("OK: schtasks {}", args.join(" "))
            } else {
                let stderr = String::from_utf8_lossy(&out.stderr);
                format!("GAGAL: schtasks {} -> {}", args.join(" "), stderr.trim())
            }
        }
        Err(e) => format!("ERROR menjalankan schtasks: {e}"),
    }
}

fn delete_task_silent(name: &str) {
    let _ = Command::new("schtasks")
        .args(["/Delete", "/TN", name, "/F"])
        .output();
}

fn build_tr_value(node_exe: &str, script_path: &str) -> String {
    format!("\"{node_exe}\" \"{script_path}\"")
}

#[tauri::command]
fn apply_schedule(app: tauri::AppHandle, config: AppConfig) -> Result<Vec<String>, String> {
    write_config_to_disk(&app, &config)?;

    if config.node_exe_path.trim().is_empty() {
        return Err("Path node.exe belum diisi.".into());
    }
    if config.project_folder.trim().is_empty() {
        return Err("Folder proyek (tempat file .js) belum diisi.".into());
    }

    let mut log: Vec<String> = Vec::new();
    let node_exe = &config.node_exe_path;
    let project = config.project_folder.trim_end_matches('\\');

    // ---------- 1. THREAD POSTER (bisa banyak slot jam per hari) ----------
    for i in 1..=MAX_THREAD_POSTER_SLOTS {
        delete_task_silent(&task_name_thread_poster(i));
    }

    let script_path = format!("{project}\\thread-poster.js");
    let tr_value = build_tr_value(node_exe, &script_path);

    for (idx, time) in config.schedule.thread_poster_times.iter().enumerate() {
        let name = task_name_thread_poster((idx as u32) + 1);
        let result = run_schtasks(&[
            "/Create",
            "/SC", "DAILY",
            "/ST", time,
            "/TN", &name,
            "/TR", &tr_value,
            "/RU", "SYSTEM",
            "/F",
        ]);
        log.push(format!("[Thread Poster @ {time}] {result}"));
    }

    // ---------- 2. REPLY CHECKER ----------
    delete_task_silent(TASK_NAME_REPLY_CHECKER);
    if config.schedule.reply_checker_interval_minutes > 0 {
        let script_path = format!("{project}\\reply-checker.js");
        let tr_value = build_tr_value(node_exe, &script_path);
        let interval = config.schedule.reply_checker_interval_minutes.to_string();

        let result = run_schtasks(&[
            "/Create",
            "/SC", "MINUTE",
            "/MO", &interval,
            "/TN", TASK_NAME_REPLY_CHECKER,
            "/TR", &tr_value,
            "/RU", "SYSTEM",
            "/F",
        ]);
        log.push(format!("[Reply Checker tiap {interval} menit] {result}"));
    } else {
        log.push("[Reply Checker] dilewati (interval 0 / dinonaktifkan).".into());
    }

    // ---------- 3. COMMENT RESPONDER ----------
    delete_task_silent(TASK_NAME_COMMENT_RESPONDER);
    if config.ai_reply_enabled && config.schedule.comment_responder_interval_minutes > 0 {
        let script_path = format!("{project}\\comment-responder.js");
        let tr_value = build_tr_value(node_exe, &script_path);
        let interval = config.schedule.comment_responder_interval_minutes.to_string();

        let result = run_schtasks(&[
            "/Create",
            "/SC", "MINUTE",
            "/MO", &interval,
            "/TN", TASK_NAME_COMMENT_RESPONDER,
            "/TR", &tr_value,
            "/RU", "SYSTEM",
            "/F",
        ]);
        log.push(format!("[Comment Responder tiap {interval} menit] {result}"));
    } else {
        log.push("[Comment Responder] dilewati (toggle AI nonaktif, atau interval 0).".into());
    }

    // ---------- 4. REFRESH TOKEN (mingguan) ----------
    delete_task_silent(TASK_NAME_REFRESH_TOKEN);
    if !config.schedule.refresh_token_day.trim().is_empty()
        && !config.schedule.refresh_token_time.trim().is_empty()
    {
        let script_path = format!("{project}\\refresh-token.js");
        let tr_value = build_tr_value(node_exe, &script_path);

        let result = run_schtasks(&[
            "/Create",
            "/SC", "WEEKLY",
            "/D", &config.schedule.refresh_token_day,
            "/ST", &config.schedule.refresh_token_time,
            "/TN", TASK_NAME_REFRESH_TOKEN,
            "/TR", &tr_value,
            "/RU", "SYSTEM",
            "/F",
        ]);
        log.push(format!(
            "[Refresh Token setiap {} jam {}] {}",
            config.schedule.refresh_token_day, config.schedule.refresh_token_time, result
        ));
    } else {
        log.push("[Refresh Token] dilewati (hari/jam belum diisi).".into());
    }

    Ok(log)
}

#[tauri::command]
fn detect_node_path() -> Result<String, String> {
    let output = Command::new("where")
        .arg("node")
        .output()
        .map_err(|e| format!("Gagal menjalankan 'where node': {e}"))?;

    if !output.status.success() {
        return Err("node.exe tidak ditemukan di PATH. Isi manual path-nya.".into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next().unwrap_or("").trim();

    if first_line.is_empty() {
        Err("node.exe tidak ditemukan.".into())
    } else {
        Ok(first_line.to_string())
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            apply_schedule,
            detect_node_path
        ])
        .run(tauri::generate_context!())
        .expect("Gagal menjalankan aplikasi Tauri");
}
