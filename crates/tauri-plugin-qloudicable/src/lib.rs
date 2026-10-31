//! Android-only bridge to `QloudiCablePlugin.kt`. Desktop builds never register it.
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{AppHandle, Manager, Runtime};

#[cfg(target_os = "android")]
struct Handle<R: Runtime>(tauri::plugin::PluginHandle<R>);

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("qloudicable")
        .setup(|app, _api| {
            #[cfg(target_os = "android")]
            app.manage(Handle(_api.register_android_plugin("com.qloudihub.qloudicable.plugin", "QloudiCablePlugin")?));
            let _ = app;
            Ok(())
        })
        .build()
}

#[cfg(target_os = "android")]
pub fn facts<R: Runtime>(app: &AppHandle<R>) -> Result<serde_json::Value, String> {
    app.state::<Handle<R>>().0.run_mobile_plugin("facts", ()).map_err(|e| e.to_string())
}

#[cfg(not(target_os = "android"))]
pub fn facts<R: Runtime>(app: &AppHandle<R>) -> Result<serde_json::Value, String> {
    let _ = app.path();
    Err("Android only".into())
}
