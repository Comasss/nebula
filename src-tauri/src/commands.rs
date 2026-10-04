use crate::config::{DebridConfig, IndexerConfig, ServerConfig, Settings};
use crate::engine::{AddRequest, Engine, FileView, HistoryEntry, NzbPreview, Snapshot, TestResult};
use crate::logger::LogEntry;
use crate::stats::{IndexerEvent, StatRecord};
use crate::{debrid, fetch, indexer, logger, nzb, winint};
use serde::Serialize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};

type Eng<'a> = State<'a, Arc<Engine>>;

#[tauri::command]
pub fn get_settings(e: Eng) -> Settings {
    e.settings.read().clone()
}

/// La interfaz avisa del idioma que usa (el elegido o el del sistema) para el menú de la bandeja,
/// las notificaciones y los mensajes del motor.
#[tauri::command]
pub fn set_language(app: AppHandle, e: Eng, lang: String) {
    if crate::i18n::set(&lang) {
        let paused = e.st.lock().paused;
        crate::refresh_tray_texts(&app, paused);
    }
}

#[tauri::command]
pub fn save_settings(e: Eng, settings: Settings) -> Result<(), String> {
    e.inner().apply_settings(settings)
}

#[tauri::command]
pub async fn test_server(server: ServerConfig) -> TestResult {
    crate::engine::test_server(server).await
}

#[tauri::command]
pub async fn inspect_nzb_file(e: Eng<'_>, path: String) -> Result<NzbPreview, String> {
    let data = tokio::fs::read(&path).await.map_err(|err| t!("err.readFile", error = err))?;
    let name = std::path::Path::new(&path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let engine = e.inner().clone();
    tokio::task::spawn_blocking(move || engine.register_pending(data, &name, true, path))
        .await
        .map_err(|_| t!("err.internal"))?
}

/// Descarga un NZB de un enlace y devuelve sus datos y nombre. Si viene de un indexador,
/// anota en las estadísticas si se pudo obtener y cuánto tardó.
async fn grab_nzb(e: &Engine, url: &str, indexer_id: Option<&str>) -> Result<(Vec<u8>, fetch::Fetched), String> {
    let start = std::time::Instant::now();
    let res = async {
        let f = fetch::fetch(url).await?;
        let data = nzb::maybe_gunzip(&f.data)?;
        if !nzb::looks_like_nzb(&data) {
            return Err(fetch::explain_not_nzb(&data));
        }
        Ok((data, f))
    }
    .await;
    if let Some(id) = indexer_id {
        e.record_indexer(IndexerEvent {
            at: crate::util::now_secs(),
            id: id.to_string(),
            name: indexer_label(e, id),
            kind: "grab".into(),
            ms: start.elapsed().as_millis() as u64,
            ok: res.is_ok(),
            ..Default::default()
        });
    }
    res
}

fn indexer_label(e: &Engine, id: &str) -> String {
    e.settings.read().indexers.iter().find(|i| i.id == id).map(|i| i.label()).unwrap_or_default()
}

#[tauri::command]
pub async fn inspect_nzb_url(e: Eng<'_>, url: String, indexer_id: Option<String>) -> Result<NzbPreview, String> {
    let (data, f) = grab_nzb(e.inner(), &url, indexer_id.as_deref()).await?;
    let engine = e.inner().clone();
    let name = f.filename.unwrap_or_default();
    let preview = tokio::task::spawn_blocking(move || engine.register_pending(data, &name, f.strong, url))
        .await
        .map_err(|_| t!("err.internal"))??;
    if let Some(id) = indexer_id {
        e.set_pending_indexer(&preview.token, id);
    }
    Ok(preview)
}

#[tauri::command]
pub fn discard_pending(e: Eng, token: String) {
    e.discard_pending(&token);
}

#[tauri::command]
pub fn add_job(e: Eng, req: AddRequest) -> Result<String, String> {
    e.inner().add_job(req)
}

#[tauri::command]
pub fn get_stats(e: Eng) -> Vec<StatRecord> {
    e.stats()
}

#[tauri::command]
pub fn get_queue(e: Eng) -> Snapshot {
    e.snapshot()
}

#[tauri::command]
pub fn get_job_files(e: Eng, id: String) -> Vec<FileView> {
    e.job_files(&id)
}

#[tauri::command]
pub fn pause_job(e: Eng, id: String) {
    e.pause_job(&id);
}

#[tauri::command]
pub fn resume_job(e: Eng, id: String) {
    e.inner().resume_job(&id);
}

#[tauri::command]
pub fn remove_job(e: Eng, id: String, delete_files: bool) {
    e.remove_job(&id, delete_files);
}

#[tauri::command]
pub fn move_job(e: Eng, id: String, to: usize) {
    e.move_job(&id, to);
}

#[tauri::command]
pub fn set_paused(e: Eng, paused: bool) {
    e.set_paused(paused);
}

#[tauri::command]
pub fn set_speed_limit(e: Eng, kbps: u64) -> Result<(), String> {
    e.set_speed_limit(kbps)
}

#[tauri::command]
pub fn get_history(e: Eng) -> Vec<HistoryEntry> {
    e.history()
}

#[tauri::command]
pub fn remove_history(e: Eng, id: String, delete_files: bool) {
    e.remove_history(&id, delete_files);
}

#[tauri::command]
pub fn clear_history(e: Eng, only_completed: bool) {
    e.clear_history(only_completed);
}

#[tauri::command]
pub fn retry_history(e: Eng, id: String) -> Result<String, String> {
    e.inner().retry_history(&id)
}

#[tauri::command]
pub fn retry_extract(e: Eng, id: String, password: Option<String>) -> Result<(), String> {
    e.inner().retry_extract(&id, password)
}

#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    winint::open_path(&path)
}

#[tauri::command]
pub fn reveal_path(path: String) -> Result<(), String> {
    winint::reveal_path(&path)
}

#[tauri::command]
pub fn get_logs() -> Vec<LogEntry> {
    logger::entries()
}

#[tauri::command]
pub fn clear_logs() {
    logger::clear();
}

#[tauri::command]
pub fn disk_free(path: String) -> Option<u64> {
    winint::disk_free(std::path::Path::new(&path))
}

#[tauri::command]
pub fn default_patterns() -> Vec<String> {
    crate::config::default_patterns()
}

#[tauri::command]
pub fn detect_nzb_urls(e: Eng, text: String) -> Vec<String> {
    let patterns = e.settings.read().clipboard_patterns.clone();
    winint::detect_nzb_urls(&text, &patterns)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    version: String,
    data_dir: String,
    tools_dir: String,
    log_dir: String,
}

#[tauri::command]
pub fn app_info(e: Eng) -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION").into(),
        data_dir: e.paths.data.to_string_lossy().to_string(),
        tools_dir: e.paths.tools.to_string_lossy().to_string(),
        log_dir: e.paths.logs.to_string_lossy().to_string(),
    }
}

/// La interfaz ya está lista: se muestra la ventana y se entregan los archivos pendientes.
#[tauri::command]
pub fn app_ready(app: AppHandle, e: Eng) {
    let first = !e.ui_ready.swap(true, Ordering::SeqCst);
    if first {
        let minimized = std::env::args().any(|a| a == "--minimized") || e.settings.read().start_minimized;
        if let Some(w) = app.get_webview_window("main") {
            if !minimized {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }
    }
    let pending: Vec<_> = e.pending_open.lock().drain(..).collect();
    if !pending.is_empty() {
        let _ = app.emit("open-items", pending);
    }
}

#[tauri::command]
pub fn window_minimize(window: WebviewWindow, e: Eng) {
    if e.settings.read().minimize_to_tray {
        let _ = window.hide();
    } else {
        let _ = window.minimize();
    }
}

#[tauri::command]
pub fn window_close(app: AppHandle, window: WebviewWindow, e: Eng) {
    if e.settings.read().close_to_tray {
        let _ = window.hide();
    } else {
        app.exit(0);
    }
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

// ---- Indexadores ----

#[tauri::command]
pub async fn search_indexer(
    e: Eng<'_>,
    id: String,
    query: String,
    category: Option<String>,
    offset: u32,
    search_id: Option<String>,
) -> Result<indexer::SearchResult, String> {
    let (ix, limit) = {
        let s = e.settings.read();
        let ix = s.indexers.iter().find(|i| i.id == id).cloned().ok_or_else(|| t!("err.indexerNotFound"))?;
        (ix, s.search_limit)
    };
    let start = std::time::Instant::now();
    let res = indexer::search(&ix, &query, category.as_deref(), offset, limit).await;
    e.record_indexer(IndexerEvent {
        at: crate::util::now_secs(),
        id: ix.id.clone(),
        name: ix.label(),
        kind: "search".into(),
        ms: res.as_ref().map(|r| r.ms).unwrap_or_else(|_| start.elapsed().as_millis() as u64),
        ok: res.is_ok(),
        results: res.as_ref().map(|r| r.items.len() as u64).unwrap_or(0),
        offset,
        search: search_id,
    });
    res.map_err(|err| {
        crate::lwarn!("{}: {}", ix.label(), err);
        err
    })
}

#[tauri::command]
pub fn get_indexer_stats(e: Eng) -> Vec<IndexerEvent> {
    e.indexer_stats()
}

#[tauri::command]
pub async fn test_indexer(indexer: IndexerConfig) -> indexer::IndexerTest {
    indexer::test(&indexer).await
}

/// Descarga un NZB desde un enlace y lo guarda en una carpeta del PC.
#[tauri::command]
pub async fn save_nzb_url(e: Eng<'_>, url: String, dir: String, name: String, indexer_id: Option<String>) -> Result<String, String> {
    let (data, f) = grab_nzb(e.inner(), &url, indexer_id.as_deref()).await?;
    let name = if name.trim().is_empty() { f.filename.unwrap_or_default() } else { name };
    fetch::save_nzb_file(&data, &dir, &name)
}

/// Guarda el NZB de una vista previa ya analizada (sin volver a pedirlo al indexador).
#[tauri::command]
pub fn save_pending_nzb(e: Eng, token: String, dir: String, name: String) -> Result<String, String> {
    let (raw, pending_name) = e.pending_nzb(&token).ok_or_else(|| t!("err.previewExpiredRetry"))?;
    let data = nzb::maybe_gunzip(&raw)?;
    let path = fetch::save_nzb_file(&data, &dir, if name.trim().is_empty() { &pending_name } else { &name })?;
    e.discard_pending(&token);
    Ok(path)
}

#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(t!("err.invalidLink"));
    }
    std::process::Command::new("explorer").arg(url).spawn().map(|_| ()).map_err(|e| e.to_string())
}

// ---- Servicios debrid ----

#[tauri::command]
pub async fn test_debrid(provider: DebridConfig) -> debrid::DebridTest {
    debrid::test(&provider).await
}

/// Servicios debrid que ya tienen en caché el NZB de una vista previa (descarga instantánea).
#[tauri::command]
pub async fn debrid_cached(e: Eng<'_>, token: String) -> Result<Vec<String>, String> {
    let providers: Vec<DebridConfig> = e.settings.read().debrid.iter().filter(|d| d.usable()).cloned().collect();
    if !providers.iter().any(|p| p.kind == "torbox") {
        return Ok(Vec::new());
    }
    let (raw, _) = e.pending_nzb(&token).ok_or_else(|| t!("err.previewExpired"))?;
    let hash = debrid::nzb_hash(&nzb::maybe_gunzip(&raw)?);
    let mut out = Vec::new();
    for p in providers {
        if let Ok(true) = debrid::cached(&p, &hash).await {
            out.push(p.id);
        }
    }
    Ok(out)
}
