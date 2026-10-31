use std::path::PathBuf;
use tauri::{AppHandle, Runtime};
use qloudicable_core::bench::{self, ChargeTest, DataTest};
use qloudicable_core::{Report, Snapshot, diagnose};

#[cfg(target_os = "android")]
fn snapshot<R: Runtime>(app: &AppHandle<R>) -> Result<Snapshot, String> {
    use qloudicable_backend::{android, linux};
    let facts = serde_json::from_value(tauri_plugin_qloudicable::facts(app)?).map_err(|e| e.to_string())?;
    Ok(android::snapshot(facts, linux::collect(std::path::Path::new("/"))))
}

#[cfg(not(target_os = "android"))]
fn snapshot<R: Runtime>(_app: &AppHandle<R>) -> Result<Snapshot, String> {
    Ok(qloudicable_backend::collect())
}

/// The OS calls can block, so they stay off the UI thread.
async fn off_ui<T: Send + 'static>(job: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(job).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn report<R: Runtime>(app: AppHandle<R>) -> Result<Report, String> {
    off_ui(move || Ok(diagnose(&snapshot(&app)?))).await
}

#[tauri::command]
async fn power_sample<R: Runtime>(app: AppHandle<R>) -> Result<Option<u32>, String> {
    off_ui(move || Ok(snapshot(&app)?.power.and_then(|p| p.charging_milliwatts()))).await
}

#[tauri::command]
fn charge_result(watts: Vec<f64>) -> ChargeTest {
    bench::charge_test(&watts)
}

#[tauri::command]
async fn write_test(dir: String, megabytes: u32) -> Result<DataTest, String> {
    if !(16..=4096).contains(&megabytes) {
        return Err("Test size must be between 16 MB and 4096 MB.".into());
    }
    off_ui(move || {
        qloudicable_backend::tests_active::write_speed(&PathBuf::from(dir), megabytes, None)
            .map_err(|e| format!("Couldn't write to that folder: {e}"))
    })
    .await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_qloudicable::init())
        .invoke_handler(tauri::generate_handler![report, power_sample, charge_result, write_test])
        .run(tauri::generate_context!())
        .expect("error while running QloudiCable");
}
