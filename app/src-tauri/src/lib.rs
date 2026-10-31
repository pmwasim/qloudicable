use qloudicable_core::bench::{self, ChargeTest, DataTest};
use qloudicable_core::orientation::{self, End, OrientationMatrix, PlugRun};
use qloudicable_core::selfcheck::SelfCheck;
use qloudicable_core::watch::{self, PortSample, PortWatch};
use qloudicable_core::{Report, Snapshot, diagnose};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Runtime};

#[cfg(target_os = "android")]
fn snapshot<R: Runtime>(app: &AppHandle<R>) -> Result<Snapshot, String> {
    use qloudicable_backend::{android, linux};
    let facts =
        serde_json::from_value(tauri_plugin_qloudicable::facts(app)?).map_err(|e| e.to_string())?;
    Ok(android::snapshot(
        facts,
        linux::collect(std::path::Path::new("/")),
    ))
}

#[cfg(not(target_os = "android"))]
fn snapshot<R: Runtime>(_app: &AppHandle<R>) -> Result<Snapshot, String> {
    Ok(qloudicable_backend::collect())
}

/// The OS calls can block, so they stay off the UI thread.
async fn off_ui<T: Send + 'static>(
    job: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(job)
        .await
        .map_err(|e| e.to_string())?
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
fn self_check<R: Runtime>(app: AppHandle<R>) -> Result<SelfCheck, String> {
    let snap = snapshot(&app)?;
    let rep = diagnose(&snap);
    Ok(SelfCheck::evaluate(&rep, &snap))
}

#[tauri::command]
fn build_info() -> String {
    qloudicable_core::version_label()
}

/// How often ports are read while watching. Fast enough to see a cable that cycles about once a second.
const WATCH_INTERVAL_MS: u64 = 250;

#[tauri::command]
async fn watch_ports<R: Runtime>(app: AppHandle<R>, seconds: u32) -> Result<PortWatch, String> {
    if !(3..=60).contains(&seconds) {
        return Err("Watch for between 3 and 60 seconds.".into());
    }
    off_ui(move || {
        let start = Instant::now();
        let window = Duration::from_secs(seconds as u64);
        let mut samples: Vec<PortSample> = Vec::new();
        loop {
            let t_ms = start.elapsed().as_millis() as u64;
            for p in snapshot(&app)?.ports {
                samples.push(PortSample {
                    t_ms,
                    port_id: p.id,
                    connected: p.connected,
                });
            }
            if start.elapsed() >= window {
                break;
            }
            std::thread::sleep(Duration::from_millis(WATCH_INTERVAL_MS));
        }
        Ok(watch::analyze(
            &samples,
            seconds as u64 * 1000,
            WATCH_INTERVAL_MS,
        ))
    })
    .await
}

#[tauri::command]
async fn capture_run<R: Runtime>(
    app: AppHandle<R>,
    end: String,
    step: u8,
) -> Result<PlugRun, String> {
    let end = match end.as_str() {
        "A" => End::A,
        "B" => End::B,
        _ => return Err("End must be A or B.".into()),
    };
    if !(1..=2).contains(&step) {
        return Err("Position must be 1 (as inserted) or 2 (turned 180 degrees).".into());
    }
    off_ui(move || Ok(orientation::capture(&diagnose(&snapshot(&app)?), end, step))).await
}

#[tauri::command]
fn orientation_matrix(runs: Vec<PlugRun>) -> OrientationMatrix {
    orientation::matrix(&runs)
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
        .invoke_handler(tauri::generate_handler![
            report,
            power_sample,
            charge_result,
            self_check,
            write_test,
            build_info,
            watch_ports,
            capture_run,
            orientation_matrix
        ])
        .run(tauri::generate_context!())
        .expect("error while running QloudiCable");
}
