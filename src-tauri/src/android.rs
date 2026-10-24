//! Integración con Android. La parte nativa está en
//! gen/android/app/src/main/java/com/nebula/downloader/NebulaPlugin.kt: notificaciones, el
//! servicio que mantiene las descargas con la pantalla apagada, abrir carpetas y enlaces,
//! archivos .nzb abiertos desde otras apps y el permiso de acceso a los archivos.
use parking_lot::Mutex;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tauri::plugin::{Builder, PluginHandle, TauriPlugin};
use tauri::Wry;

static BRIDGE: OnceLock<PluginHandle<Wry>> = OnceLock::new();

pub fn plugin() -> TauriPlugin<Wry> {
    Builder::new("nebula")
        .setup(|_app, api| {
            let handle = api.register_android_plugin("com.nebula.downloader", "NebulaPlugin")?;
            let _ = BRIDGE.set(handle);
            Ok(())
        })
        .build()
}

/// Llama a un comando del plugin de Kotlin. Nunca desde el hilo principal de Android.
async fn call<T: DeserializeOwned>(cmd: &str, payload: serde_json::Value) -> Result<T, String> {
    let bridge = BRIDGE.get().ok_or_else(|| "Android bridge not ready".to_string())?;
    bridge.run_mobile_plugin_async(cmd, payload).await.map_err(|e| e.to_string())
}

/// Para lo que no necesita respuesta: se lanza en segundo plano.
fn fire(cmd: &'static str, payload: serde_json::Value) {
    tauri::async_runtime::spawn(async move {
        if let Err(e) = call::<serde_json::Value>(cmd, payload).await {
            crate::lwarn!("Android {}: {}", cmd, e);
        }
    });
}

pub fn notify(title: &str, body: &str) {
    fire("showNotification", json!({ "title": title, "body": body }));
}

/// Mantiene (o retira) el servicio en primer plano mientras haya descargas activas.
pub fn keep_alive(active: bool, text: &str) {
    static LAST: Mutex<Option<(bool, String)>> = Mutex::new(None);
    let next = (active, if active { text.to_string() } else { String::new() });
    {
        let mut last = LAST.lock();
        if last.as_ref() == Some(&next) {
            return;
        }
        *last = Some(next);
    }
    fire("keepAlive", json!({ "active": active, "text": text }));
}

pub fn open_path(path: &str) -> Result<(), String> {
    fire("openPath", json!({ "path": path }));
    Ok(())
}

pub fn move_to_back() {
    fire("moveToBack", json!({}));
}

pub fn haptic(kind: &str) {
    fire("haptic", json!({ "kind": kind }));
}

pub fn open_url(url: &str) -> Result<(), String> {
    fire("openUrl", json!({ "url": url }));
    Ok(())
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Resume {
    files: Vec<String>,
    urls: Vec<String>,
    clipboard: String,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResumeInfo {
    /// .nzb abiertos o compartidos desde otra app (copiados a la carpeta de la app).
    pub files: Vec<String>,
    /// Enlaces compartidos con Nebula desde otra app.
    pub urls: Vec<String>,
    /// Texto del portapapeles, para buscar enlaces a NZB (solo si está activado).
    pub clipboard: String,
    /// Si Nebula puede escribir en el almacenamiento compartido (Descargas…).
    pub storage: bool,
}

/// La app vuelve a primer plano: archivos pendientes, portapapeles y permiso de almacenamiento.
pub async fn resume(read_clipboard: bool) -> ResumeInfo {
    let r: Resume = call("resume", json!({ "clipboard": read_clipboard })).await.unwrap_or_default();
    ResumeInfo { files: r.files, urls: r.urls, clipboard: r.clipboard, storage: storage_access().await }
}

#[derive(Default, Deserialize)]
struct Granted {
    granted: bool,
}

pub async fn storage_access() -> bool {
    call::<Granted>("storageAccess", json!({})).await.map(|g| g.granted).unwrap_or(false)
}

/// Abre la pantalla del sistema para dar acceso a los archivos (o pide el permiso clásico).
pub async fn request_storage_access() -> bool {
    call::<Granted>("requestStorageAccess", json!({})).await.map(|g| g.granted).unwrap_or(false)
}

/// Carpeta de las bibliotecas nativas del APK, donde también van 7-Zip y par2 (lib7zz.so y
/// libpar2.so). Es la carpeta de este mismo .so, que Android extrae al instalar la app.
pub fn native_lib_dir() -> PathBuf {
    let mut info: libc::Dl_info = unsafe { std::mem::zeroed() };
    let addr = native_lib_dir as *const libc::c_void;
    if unsafe { libc::dladdr(addr, &mut info) } != 0 && !info.dli_fname.is_null() {
        let file = unsafe { std::ffi::CStr::from_ptr(info.dli_fname) }.to_string_lossy().to_string();
        if let Some(dir) = Path::new(&file).parent() {
            return dir.to_path_buf();
        }
    }
    PathBuf::from(".")
}

/// Antes de arrancar: certificados para TLS y carpeta temporal para los procesos hijos.
pub fn prepare(data: &Path) {
    let _ = std::fs::create_dir_all(data);
    // OpenSSL (incluido en la app) no conoce el almacén de certificados de Android: se juntan
    // los del sistema en un archivo PEM y se le indica con SSL_CERT_FILE.
    let bundle = data.join("cacerts.pem");
    let mut pem = Vec::new();
    for dir in ["/apex/com.android.conscrypt/cacerts", "/system/etc/security/cacerts"] {
        let Ok(rd) = std::fs::read_dir(dir) else { continue };
        for entry in rd.flatten() {
            if let Ok(data) = std::fs::read(entry.path()) {
                pem.extend_from_slice(&data);
                pem.push(b'\n');
            }
        }
        if !pem.is_empty() {
            break;
        }
    }
    if !pem.is_empty() && std::fs::write(&bundle, &pem).is_ok() {
        std::env::set_var("SSL_CERT_FILE", &bundle);
    }
    let tmp = data.join("tmp");
    let _ = std::fs::create_dir_all(&tmp);
    std::env::set_var("TMPDIR", &tmp);
}
