use qloudicable_core::bench::{self, ChargeTest, DataTest};
use qloudicable_core::evolution::{EvolutionManager, QaScorecard};
use qloudicable_core::virtual_engine::{
    VirtualCableProfile, VirtualTarget, VirtualTestReport, VirtualTester,
};
use qloudicable_core::{Report, Snapshot, diagnose};
use std::path::PathBuf;
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
fn virtual_test(profile: String, target: String) -> Result<VirtualTestReport, String> {
    let prof = match profile.as_str() {
        "apple_woven_epr_240w" => VirtualCableProfile::AppleWovenEpr240W,
        "usb4_gen3x2_40gbps_240w" => VirtualCableProfile::Usb4Gen3x2_40Gbps_240W,
        "usb3_gen2_10gbps_100w" => VirtualCableProfile::Usb3Gen2_10Gbps_100W,
        "usb2_charging_60w" => VirtualCableProfile::Usb2Charging60W,
        "thunderbolt4_40gbps" => VirtualCableProfile::Thunderbolt4_40Gbps,
        "counterfeit_or_damaged" => VirtualCableProfile::CounterfeitOrDamaged,
        "unmarked_passive" => VirtualCableProfile::UnmarkedPassive,
        other => return Err(format!("Unknown profile: {other}")),
    };

    let tgt = match target.as_str() {
        "charger_140w" => VirtualTarget::Charger { max_watts: 140 },
        "charger_65w" => VirtualTarget::Charger { max_watts: 65 },
        "nvme_storage_10gbps" => VirtualTarget::HighSpeedStorage {
            capable_mbps: 10_000,
        },
        "display_4k" => VirtualTarget::HighResolutionDisplay { bandwidth_gbps: 20 },
        "legacy_device" => VirtualTarget::LegacyPeripheral,
        other => return Err(format!("Unknown target: {other}")),
    };

    Ok(VirtualTester::evaluate(&prof, &tgt))
}

#[tauri::command]
fn qa_audit<R: Runtime>(app: AppHandle<R>) -> Result<QaScorecard, String> {
    let snap = snapshot(&app)?;
    let rep = diagnose(&snap);
    Ok(QaScorecard::evaluate(&rep, &snap))
}

#[tauri::command]
fn evolution_status() -> EvolutionManager {
    EvolutionManager::default()
}

#[tauri::command]
fn trip_circuit_breaker() -> EvolutionManager {
    let mut mgr = EvolutionManager::default();
    mgr.rollback_to_safe_state();
    mgr
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
            virtual_test,
            qa_audit,
            evolution_status,
            trip_circuit_breaker,
            write_test
        ])
        .run(tauri::generate_context!())
        .expect("error while running QloudiCable");
}
