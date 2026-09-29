//! Servicios debrid que descargan NZB en sus propios servidores (TorBox Pro, Premiumize).
//!
//! Nebula les envía el NZB, espera a que terminen (ellos reparan y extraen) y después baja los
//! archivos resultantes por HTTPS, uno a uno y por rangos (ver `debrid_job`). Así se obtiene el
//! mismo resultado que con NNTP pero a la velocidad de su CDN.
use crate::config::DebridConfig;
use md5::{Digest, Md5};
use serde::Serialize;
use serde_json::Value;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

/// Servicios admitidos: (id interno, nombre).
pub const KINDS: &[(&str, &str)] = &[("torbox", "TorBox"), ("premiumize", "Premiumize")];

pub fn kind_label(kind: &str) -> &'static str {
    KINDS.iter().find(|(k, _)| *k == kind).map(|(_, n)| *n).unwrap_or("Debrid")
}

pub fn is_known(kind: &str) -> bool {
    KINDS.iter().any(|(k, _)| *k == kind)
}

const UA: &str = concat!("Nebula/", env!("CARGO_PKG_VERSION"));

/// Cliente para la API de los servicios.
static API: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent(UA)
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(120))
        .build()
        .expect("cliente HTTP")
});

/// Cliente para bajar los archivos: sin compresión (los rangos deben ser bytes reales) ni tiempo total máximo.
pub static FILES: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent(UA)
        .no_gzip()
        .connect_timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::limited(10))
        .pool_max_idle_per_host(64)
        .build()
        .expect("cliente HTTP")
});

// ---------------------------------------------------------------------------
// Tipos comunes
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrKind {
    /// Fallo de red o error temporal del servicio: se reintenta.
    Transient,
    /// El servicio está ocupado (límite de descargas activas o de peticiones): se espera y se reintenta.
    Busy,
    /// API key rechazada.
    Auth,
    /// No se puede completar con este servicio (plan, NZB rechazado, descarga fallida…).
    Fatal,
}

#[derive(Debug, Clone)]
pub struct DebridError {
    pub kind: ErrKind,
    pub msg: String,
    /// Código de error del servicio (p. ej. DUPLICATE_ITEM), si lo hay.
    pub code: String,
}

impl DebridError {
    pub fn new(kind: ErrKind, msg: impl Into<String>) -> Self {
        Self { kind, msg: msg.into(), code: String::new() }
    }
    fn with_code(mut self, code: &str) -> Self {
        self.code = code.to_string();
        self
    }
}

impl std::fmt::Display for DebridError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.msg)
    }
}

/// NZB aceptado por el servicio. Si queda en su cola todavía no tiene `id`: se localiza por `hash`.
#[derive(Debug, Clone, PartialEq)]
pub struct Submitted {
    pub id: String,
    pub hash: String,
}

/// Archivo ya descargado en el servicio. `path` es relativo (sin la carpeta raíz del release).
#[derive(Debug, Clone, PartialEq)]
pub struct RemoteFile {
    pub id: String,
    pub path: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RemoteState {
    /// El servicio aún lo está descargando o procesando (o lo tiene en cola).
    Working { percent: f64, speed: u64, eta: u64, state: String },
    /// Terminado: archivos disponibles. `folder` identifica la carpeta remota (para borrarla después).
    Ready { files: Vec<RemoteFile>, folder: String },
    Failed(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Status {
    /// Id descubierto si antes no se conocía (estaba en la cola del servicio).
    pub id: Option<String>,
    pub state: RemoteState,
}

pub struct Upload<'a> {
    pub nzb: &'a [u8],
    pub name: &'a str,
    pub password: Option<&'a str>,
    /// Post-procesado de TorBox: -1 por defecto (reparar, extraer y borrar), 0 nada, 1 reparar,
    /// 2 reparar y extraer, 3 reparar, extraer y borrar.
    pub post_processing: i32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebridTest {
    pub ok: bool,
    pub message: String,
    pub warning: Option<String>,
    pub latency_ms: u64,
}

/// Hash que usa TorBox para identificar un NZB (md5 del archivo).
pub fn nzb_hash(data: &[u8]) -> String {
    Md5::digest(data).iter().map(|b| format!("{:02x}", b)).collect()
}

// ---------------------------------------------------------------------------
// Operaciones (según el servicio)
// ---------------------------------------------------------------------------

fn key(cfg: &DebridConfig) -> Result<&str, DebridError> {
    let k = cfg.api_key.trim();
    if k.is_empty() {
        Err(DebridError::new(ErrKind::Auth, format!("Falta la API key de {}.", cfg.label())))
    } else {
        Ok(k)
    }
}

fn unknown(kind: &str) -> DebridError {
    DebridError::new(ErrKind::Fatal, format!("Servicio debrid desconocido: {}", kind))
}

/// Envía el NZB al servicio.
pub async fn submit(cfg: &DebridConfig, up: &Upload<'_>) -> Result<Submitted, DebridError> {
    let k = key(cfg)?;
    match cfg.kind.as_str() {
        "torbox" => torbox::submit(k, up).await,
        "premiumize" => premiumize::submit(k, up).await,
        other => Err(unknown(other)),
    }
}

/// Estado de una descarga enviada.
pub async fn status(cfg: &DebridConfig, id: &str, hash: &str) -> Result<Status, DebridError> {
    let k = key(cfg)?;
    match cfg.kind.as_str() {
        "torbox" => torbox::status(k, id, hash).await,
        "premiumize" => premiumize::status(k, id).await,
        other => Err(unknown(other)),
    }
}

/// Enlace directo (temporal) para bajar un archivo.
pub async fn file_link(cfg: &DebridConfig, id: &str, file_id: &str) -> Result<String, DebridError> {
    let k = key(cfg)?;
    match cfg.kind.as_str() {
        "torbox" => torbox::link(k, id, file_id).await,
        "premiumize" => premiumize::link(k, file_id).await,
        other => Err(unknown(other)),
    }
}

/// Borra la descarga (y sus archivos) del servicio.
pub async fn remove(cfg: &DebridConfig, id: &str, folder: &str) -> Result<(), DebridError> {
    let k = key(cfg)?;
    match cfg.kind.as_str() {
        "torbox" => torbox::remove(k, id).await,
        "premiumize" => premiumize::remove(k, id, folder).await,
        other => Err(unknown(other)),
    }
}

/// ¿El servicio ya tiene este NZB en caché (descarga instantánea)? Solo TorBox lo permite.
pub async fn cached(cfg: &DebridConfig, hash: &str) -> Result<bool, DebridError> {
    let k = key(cfg)?;
    match cfg.kind.as_str() {
        "torbox" => torbox::cached(k, hash).await,
        _ => Ok(false),
    }
}

/// Comprueba la API key y el tipo de cuenta.
pub async fn test(cfg: &DebridConfig) -> DebridTest {
    let start = Instant::now();
    let res = match key(cfg) {
        Err(e) => Err(e),
        Ok(k) => match cfg.kind.as_str() {
            "torbox" => torbox::account(k).await,
            "premiumize" => premiumize::account(k).await,
            other => Err(unknown(other)),
        },
    };
    let latency_ms = start.elapsed().as_millis() as u64;
    match res {
        Ok((message, warning)) => DebridTest { ok: true, message, warning, latency_ms },
        Err(e) => DebridTest { ok: false, message: e.msg, warning: None, latency_ms },
    }
}

// ---------------------------------------------------------------------------
// Utilidades
// ---------------------------------------------------------------------------

fn net(label: &str, e: reqwest::Error) -> DebridError {
    // Sin la URL: puede llevar la API key.
    let msg = if e.is_timeout() {
        format!("{} no respondió a tiempo", label)
    } else if e.is_connect() {
        format!("No se pudo conectar con {}", label)
    } else {
        format!("Error de conexión con {}: {}", label, e.without_url())
    };
    DebridError::new(ErrKind::Transient, msg)
}

fn http_status(label: &str, status: u16) -> DebridError {
    match status {
        401 | 403 => DebridError::new(ErrKind::Auth, format!("{} rechazó la API key (HTTP {}).", label, status)),
        429 => DebridError::new(ErrKind::Busy, format!("{} ha limitado las peticiones; se reintentará.", label)),
        500..=599 => DebridError::new(ErrKind::Transient, format!("{} tiene un error temporal (HTTP {}).", label, status)),
        200..=299 => DebridError::new(ErrKind::Transient, format!("Respuesta no válida de {}.", label)),
        c => DebridError::new(ErrKind::Fatal, format!("{} respondió con un error HTTP {}.", label, c)),
    }
}

fn str_of(v: &Value, k: &str) -> String {
    match v.get(k) {
        Some(Value::String(s)) => s.trim().to_string(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

fn id_of(v: &Value, k: &str) -> Option<String> {
    Some(str_of(v, k)).filter(|s| !s.is_empty() && s != "0")
}

fn num(v: &Value, k: &str) -> f64 {
    match v.get(k) {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(Value::String(s)) => s.trim().parse().unwrap_or(0.0),
        _ => 0.0,
    }
}

fn bool_of(v: &Value, k: &str) -> bool {
    match v.get(k) {
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0) != 0.0,
        Some(Value::String(s)) => matches!(s.trim(), "true" | "1"),
        _ => false,
    }
}

/// Progreso a porcentaje: los servicios lo dan como fracción (0–1).
fn percent(p: f64) -> f64 {
    let p = if p <= 1.0 { p * 100.0 } else { p };
    p.clamp(0.0, 100.0)
}

/// Normaliza una ruta remota: separadores `/` y sin componentes vacíos, `.` ni `..`.
pub fn clean_path(p: &str) -> String {
    p.split(['/', '\\']).map(str::trim).filter(|c| !c.is_empty() && *c != "." && *c != "..").collect::<Vec<_>>().join("/")
}

/// Quita la carpeta raíz del release si todos los archivos cuelgan de la misma.
pub fn strip_common_root(mut files: Vec<RemoteFile>) -> Vec<RemoteFile> {
    for f in files.iter_mut() {
        f.path = clean_path(&f.path);
    }
    files.retain(|f| !f.path.is_empty());
    let root = files.first().and_then(|f| f.path.split_once('/')).map(|(r, _)| r.to_string());
    if let Some(root) = root {
        let prefix = format!("{}/", root);
        if files.iter().all(|f| f.path.starts_with(&prefix)) {
            for f in files.iter_mut() {
                f.path = f.path[prefix.len()..].to_string();
            }
        }
    }
    files
}

/// Fecha "dd/mm/aaaa" de un instante unix.
fn fmt_date(ts: u64) -> String {
    let days = (ts / 86400) as i64 + 719_468;
    let era = days.div_euclid(146_097);
    let doe = days - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{:02}/{:02}/{}", d, m, y)
}

fn nzb_part(up: &Upload<'_>) -> Result<reqwest::multipart::Part, DebridError> {
    reqwest::multipart::Part::bytes(up.nzb.to_vec())
        .file_name(format!("{}.nzb", up.name))
        .mime_str("application/x-nzb")
        .map_err(|e| DebridError::new(ErrKind::Fatal, e.to_string()))
}

// ---------------------------------------------------------------------------
// TorBox (https://api-docs.torbox.app). Usenet solo con el plan Pro.
// ---------------------------------------------------------------------------

mod torbox {
    use super::*;

    const LABEL: &str = "TorBox";

    fn base() -> String {
        // NEBULA_TORBOX_URL permite apuntar a la API simulada de dev/mock_nntp.py.
        std::env::var("NEBULA_TORBOX_URL")
            .ok()
            .map(|s| s.trim().trim_end_matches('/').to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "https://api.torbox.app/v1/api".into())
    }

    async fn send(req: reqwest::RequestBuilder) -> Result<Value, DebridError> {
        let resp = req.send().await.map_err(|e| net(LABEL, e))?;
        let status = resp.status().as_u16();
        let body = resp.bytes().await.map_err(|e| net(LABEL, e))?;
        envelope(status, &body)
    }

    /// Respuesta de TorBox: {"success": bool, "error": código, "detail": texto, "data": …}
    pub(super) fn envelope(status: u16, body: &[u8]) -> Result<Value, DebridError> {
        match serde_json::from_slice::<Value>(body) {
            Ok(v) if v.get("success").and_then(Value::as_bool) == Some(true) => Ok(v.get("data").cloned().unwrap_or(Value::Null)),
            Ok(v) => Err(error_for(status, &str_of(&v, "error"), &str_of(&v, "detail"))),
            Err(_) => Err(http_status(LABEL, status)),
        }
    }

    pub(super) fn error_for(status: u16, code: &str, detail: &str) -> DebridError {
        use ErrKind::*;
        let d = if detail.is_empty() { code } else { detail };
        let e = match code {
            "NO_AUTH" | "BAD_TOKEN" | "AUTH_ERROR" => {
                DebridError::new(Auth, "TorBox rechazó la API key. Revísala en la pestaña Debrid.")
            }
            "PLAN_RESTRICTED_FEATURE" => {
                DebridError::new(Fatal, "Tu plan de TorBox no permite descargar NZB (hace falta el plan Pro).")
            }
            "ACTIVE_LIMIT" => DebridError::new(
                Busy,
                "TorBox: has llegado al límite de descargas activas; se reintentará cuando haya hueco.",
            ),
            "COOLDOWN_LIMIT" => DebridError::new(Busy, format!("TorBox pide esperar antes de otra descarga ({}).", d)),
            "MONTHLY_LIMIT" => DebridError::new(Fatal, "Has alcanzado el límite mensual de tu cuenta de TorBox."),
            "DOWNLOAD_TOO_LARGE" | "TOO_MUCH_DATA" => {
                DebridError::new(Fatal, "La descarga es demasiado grande para tu plan de TorBox.")
            }
            "BOZO_NZB" => DebridError::new(Fatal, "TorBox rechazó el NZB por no ser válido."),
            "ITEM_NOT_FOUND" => DebridError::new(Fatal, "La descarga ya no existe en TorBox."),
            "DUPLICATE_ITEM" => DebridError::new(Fatal, "Este NZB ya estaba añadido en TorBox."),
            "DATABASE_ERROR" | "UNKNOWN_ERROR" | "NO_SERVERS_AVAILABLE_ERROR" | "DOWNLOAD_SERVER_ERROR" => {
                DebridError::new(Transient, format!("TorBox tiene un problema temporal: {}", d))
            }
            _ => match status {
                401 | 403 => DebridError::new(Auth, format!("TorBox rechazó la petición: {}", d)),
                429 => DebridError::new(Busy, "TorBox ha limitado las peticiones; se reintentará."),
                500..=599 => DebridError::new(Transient, format!("TorBox tiene un error temporal: {}", d)),
                _ if d.is_empty() => http_status(LABEL, status),
                _ => DebridError::new(Fatal, format!("TorBox: {}", d)),
            },
        };
        e.with_code(code)
    }

    pub async fn submit(key: &str, up: &Upload<'_>) -> Result<Submitted, DebridError> {
        let hash = nzb_hash(up.nzb);
        let mut form = reqwest::multipart::Form::new()
            .part("file", nzb_part(up)?)
            .text("name", up.name.to_string())
            .text("post_processing", up.post_processing.to_string());
        if let Some(p) = up.password.filter(|p| !p.is_empty()) {
            form = form.text("password", p.to_string());
        }
        let req = API.post(format!("{}/usenet/createusenetdownload", base())).bearer_auth(key).multipart(form);
        match send(req).await {
            Ok(data) => Ok(parse_created(&data, &hash)),
            // Ya estaba en la cuenta: se localiza por su hash.
            Err(e) if e.code == "DUPLICATE_ITEM" => Ok(Submitted { id: String::new(), hash }),
            Err(e) => Err(e),
        }
    }

    /// data: {"usenetdownload_id": 123, "hash": "…"} o, si queda en la cola de TorBox, {"queued_id": …, "hash": "…"}.
    pub(super) fn parse_created(data: &Value, hash: &str) -> Submitted {
        let h = str_of(data, "hash");
        Submitted {
            id: id_of(data, "usenetdownload_id").unwrap_or_default(),
            hash: if h.is_empty() { hash.to_string() } else { h },
        }
    }

    pub async fn status(key: &str, id: &str, hash: &str) -> Result<Status, DebridError> {
        let url = format!("{}/usenet/mylist", base());
        if !id.is_empty() {
            let data = send(API.get(&url).bearer_auth(key).query(&[("id", id), ("bypass_cache", "true")])).await?;
            let item = match data {
                Value::Array(a) => a.into_iter().next(),
                v @ Value::Object(_) => Some(v),
                _ => None,
            };
            return Ok(Status {
                id: None,
                state: item.map(|v| parse_item(&v)).unwrap_or_else(|| RemoteState::Failed("La descarga ya no existe en TorBox.".into())),
            });
        }
        let data = send(API.get(&url).bearer_auth(key).query(&[("bypass_cache", "true")])).await?;
        let found = data
            .as_array()
            .and_then(|a| a.iter().find(|x| !hash.is_empty() && str_of(x, "hash").eq_ignore_ascii_case(hash)));
        Ok(match found {
            Some(v) => Status { id: id_of(v, "id"), state: parse_item(v) },
            None => Status {
                id: None,
                state: RemoteState::Working { percent: 0.0, speed: 0, eta: 0, state: "En cola".into() },
            },
        })
    }

    /// Un elemento de /usenet/mylist.
    pub(super) fn parse_item(v: &Value) -> RemoteState {
        let state = str_of(v, "download_state");
        let lower = state.to_ascii_lowercase();
        let files = parse_files(v);
        if bool_of(v, "download_finished") && bool_of(v, "download_present") && !files.is_empty() {
            return RemoteState::Ready { files, folder: String::new() };
        }
        if lower.contains("fail") || lower.contains("error") {
            return RemoteState::Failed(format!("TorBox no pudo completar la descarga ({}).", state));
        }
        RemoteState::Working {
            percent: percent(num(v, "progress")),
            speed: num(v, "download_speed").max(0.0) as u64,
            eta: num(v, "eta").max(0.0) as u64,
            state: describe(&state),
        }
    }

    fn parse_files(v: &Value) -> Vec<RemoteFile> {
        let files: Vec<RemoteFile> = v
            .get("files")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|f| {
                        let id = str_of(f, "id");
                        if id.is_empty() {
                            return None;
                        }
                        let name = str_of(f, "name");
                        let path = if name.is_empty() { str_of(f, "short_name") } else { name };
                        Some(RemoteFile { id, path, size: num(f, "size").max(0.0) as u64 })
                    })
                    .collect()
            })
            .unwrap_or_default();
        strip_common_root(files)
    }

    fn describe(state: &str) -> String {
        let l = state.to_ascii_lowercase();
        let s = if l.starts_with("download") {
            "Descargando"
        } else if l.contains("queue") {
            "En cola"
        } else if l.starts_with("paused") {
            "En pausa"
        } else if l.starts_with("verif") || l.starts_with("check") {
            "Verificando"
        } else if l.starts_with("repair") {
            "Reparando"
        } else if l.contains("unpack") || l.starts_with("extract") {
            "Extrayendo"
        } else if l.starts_with("moving") {
            "Moviendo"
        } else if l.starts_with("fetch") || l.starts_with("grab") || l.starts_with("metadl") {
            "Preparando"
        } else if l.starts_with("completed") || l.starts_with("cached") {
            "Terminando"
        } else if state.is_empty() {
            "Preparando"
        } else {
            return state.to_string();
        };
        s.to_string()
    }

    pub async fn link(key: &str, id: &str, file_id: &str) -> Result<String, DebridError> {
        let req = API.get(format!("{}/usenet/requestdl", base())).bearer_auth(key).query(&[
            ("token", key),
            ("usenet_id", id),
            ("file_id", file_id),
            ("zip_link", "false"),
        ]);
        let data = send(req).await?;
        data.as_str()
            .filter(|s| s.starts_with("http"))
            .map(String::from)
            .ok_or_else(|| DebridError::new(ErrKind::Transient, "TorBox no devolvió el enlace de descarga."))
    }

    pub async fn remove(key: &str, id: &str) -> Result<(), DebridError> {
        let Ok(n) = id.parse::<i64>() else { return Ok(()) };
        let body = serde_json::json!({ "usenet_id": n, "operation": "delete" });
        let req = API
            .post(format!("{}/usenet/controlusenetdownload", base()))
            .bearer_auth(key)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body.to_string());
        send(req).await.map(|_| ())
    }

    pub async fn cached(key: &str, hash: &str) -> Result<bool, DebridError> {
        let req = API
            .get(format!("{}/usenet/checkcached", base()))
            .bearer_auth(key)
            .query(&[("hash", hash), ("format", "list")]);
        Ok(match send(req).await? {
            Value::Array(a) => !a.is_empty(),
            Value::Object(o) => !o.is_empty(),
            _ => false,
        })
    }

    pub async fn account(key: &str) -> Result<(String, Option<String>), DebridError> {
        let data = send(API.get(format!("{}/user/me", base())).bearer_auth(key)).await?;
        Ok(describe_account(&data))
    }

    pub(super) fn describe_account(data: &Value) -> (String, Option<String>) {
        let plan = num(data, "plan") as i64;
        let plan_name = match plan {
            0 => "Free",
            1 => "Essential",
            2 => "Pro",
            3 => "Standard",
            _ => "desconocido",
        };
        let email = str_of(data, "email");
        let until = str_of(data, "premium_expires_at");
        let mut msg = if email.is_empty() { "Conectado".to_string() } else { format!("Conectado como {}", email) };
        msg.push_str(&format!(" · plan {}", plan_name));
        if let (Some(y), Some(m), Some(d)) = (until.get(0..4), until.get(5..7), until.get(8..10)) {
            if plan > 0 {
                msg.push_str(&format!(" hasta {}/{}/{}", d, m, y));
            }
        }
        let warning = (plan != 2).then(|| "La descarga de NZB (Usenet) de TorBox solo está incluida en el plan Pro.".to_string());
        (msg, warning)
    }
}

// ---------------------------------------------------------------------------
// Premiumize (https://www.premiumize.me/api). NZB con cuenta premium.
// ---------------------------------------------------------------------------

mod premiumize {
    use super::*;
    use regex::Regex;

    const LABEL: &str = "Premiumize";

    fn base() -> String {
        std::env::var("NEBULA_PREMIUMIZE_URL")
            .ok()
            .map(|s| s.trim().trim_end_matches('/').to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "https://www.premiumize.me/api".into())
    }

    async fn send(req: reqwest::RequestBuilder) -> Result<Value, DebridError> {
        let resp = req.send().await.map_err(|e| net(LABEL, e))?;
        let status = resp.status().as_u16();
        let body = resp.bytes().await.map_err(|e| net(LABEL, e))?;
        envelope(status, &body)
    }

    /// Respuesta de Premiumize: {"status": "success" | "error", "message": …, …campos}
    pub(super) fn envelope(status: u16, body: &[u8]) -> Result<Value, DebridError> {
        let Ok(v) = serde_json::from_slice::<Value>(body) else { return Err(http_status(LABEL, status)) };
        if str_of(&v, "status") == "error" {
            return Err(error_for(status, &str_of(&v, "message")));
        }
        if !(200..300).contains(&status) {
            return Err(http_status(LABEL, status));
        }
        Ok(v)
    }

    pub(super) fn error_for(status: u16, message: &str) -> DebridError {
        use ErrKind::*;
        let l = message.to_ascii_lowercase();
        if l.contains("not logged in") || l.contains("apikey") || l.contains("api key") || status == 401 {
            DebridError::new(Auth, "Premiumize rechazó la API key. Revísala en la pestaña Debrid.")
        } else if l.contains("already added") {
            DebridError::new(Fatal, "Este NZB ya estaba añadido en Premiumize.").with_code("DUPLICATE")
        } else if status == 429 || l.contains("too many") {
            DebridError::new(Busy, "Premiumize ha limitado las peticiones; se reintentará.")
        } else if status >= 500 {
            DebridError::new(Transient, format!("Premiumize tiene un error temporal: {}", message))
        } else if message.is_empty() {
            http_status(LABEL, status)
        } else {
            DebridError::new(Fatal, format!("Premiumize: {}", message))
        }
    }

    pub async fn submit(key: &str, up: &Upload<'_>) -> Result<Submitted, DebridError> {
        let hash = nzb_hash(up.nzb);
        let form = reqwest::multipart::Form::new().part("src", nzb_part(up)?);
        let req = API.post(format!("{}/transfer/create", base())).query(&[("apikey", key)]).multipart(form);
        match send(req).await {
            Ok(v) => match id_of(&v, "id") {
                Some(id) => Ok(Submitted { id, hash }),
                None => Err(DebridError::new(ErrKind::Transient, "Premiumize no devolvió el id de la transferencia.")),
            },
            Err(e) if e.code == "DUPLICATE" => {
                // Ya estaba añadido: se busca la transferencia por nombre.
                let fname = format!("{}.nzb", up.name);
                transfers(key)
                    .await?
                    .iter()
                    .find(|t| {
                        let n = str_of(t, "name");
                        n == up.name || n == fname
                    })
                    .and_then(|t| id_of(t, "id"))
                    .map(|id| Submitted { id, hash })
                    .ok_or(e)
            }
            Err(e) => Err(e),
        }
    }

    async fn transfers(key: &str) -> Result<Vec<Value>, DebridError> {
        let v = send(API.get(format!("{}/transfer/list", base())).query(&[("apikey", key)])).await?;
        Ok(v.get("transfers").and_then(Value::as_array).cloned().unwrap_or_default())
    }

    pub async fn status(key: &str, id: &str) -> Result<Status, DebridError> {
        let list = transfers(key).await?;
        let Some(t) = list.iter().find(|t| id_of(t, "id").as_deref() == Some(id)) else {
            return Ok(Status { id: None, state: RemoteState::Failed("La transferencia ya no existe en Premiumize.".into()) });
        };
        let state = match parse_transfer(t) {
            Transfer::Folder(folder) => {
                let files = strip_common_root(list_folder(key, &folder).await?);
                if files.is_empty() {
                    RemoteState::Failed("Premiumize terminó la transferencia pero no contiene archivos.".into())
                } else {
                    RemoteState::Ready { files, folder }
                }
            }
            Transfer::File(file) => {
                let d = item(key, &file).await?;
                let name = str_of(&d, "name");
                let files = vec![RemoteFile { id: file.clone(), path: clean_path(&name), size: num(&d, "size").max(0.0) as u64 }];
                RemoteState::Ready { files, folder: format!("file:{}", file) }
            }
            Transfer::State(s) => s,
        };
        Ok(Status { id: None, state })
    }

    pub(super) enum Transfer {
        Folder(String),
        File(String),
        State(RemoteState),
    }

    static RE_SPEED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)([\d.,]+)\s*([KMG]?)B/s").unwrap());

    /// Un elemento de /transfer/list.
    pub(super) fn parse_transfer(t: &Value) -> Transfer {
        let st = str_of(t, "status");
        let msg = str_of(t, "message");
        match st.as_str() {
            "finished" | "seeding" => {
                if let Some(f) = id_of(t, "folder_id") {
                    Transfer::Folder(f)
                } else if let Some(f) = id_of(t, "file_id") {
                    Transfer::File(f)
                } else {
                    Transfer::State(RemoteState::Failed("Premiumize terminó la transferencia pero no contiene archivos.".into()))
                }
            }
            "error" | "timeout" | "deleted" | "banned" => Transfer::State(RemoteState::Failed(format!(
                "Premiumize no pudo descargarlo: {}.",
                if msg.is_empty() { st.as_str() } else { msg.trim_end_matches('.') }
            ))),
            _ => {
                let speed = RE_SPEED
                    .captures(&msg)
                    .and_then(|c| {
                        let v: f64 = c[1].replace(',', ".").parse().ok()?;
                        let mult = match c[2].to_ascii_uppercase().as_str() {
                            "K" => 1024.0,
                            "M" => 1024.0 * 1024.0,
                            "G" => 1024.0 * 1024.0 * 1024.0,
                            _ => 1.0,
                        };
                        Some((v * mult) as u64)
                    })
                    .unwrap_or(0);
                let state = match st.as_str() {
                    "waiting" | "queued" => "En cola",
                    _ => "Descargando",
                }
                .to_string();
                Transfer::State(RemoteState::Working { percent: percent(num(t, "progress")), speed, eta: 0, state })
            }
        }
    }

    async fn list_folder(key: &str, root: &str) -> Result<Vec<RemoteFile>, DebridError> {
        let mut out = Vec::new();
        let mut stack = vec![(root.to_string(), String::new(), 0u32)];
        while let Some((id, prefix, depth)) = stack.pop() {
            let v = send(API.get(format!("{}/folder/list", base())).query(&[("apikey", key), ("id", id.as_str())])).await?;
            for c in v.get("content").and_then(Value::as_array).into_iter().flatten() {
                let name = str_of(c, "name");
                let path = if prefix.is_empty() { name } else { format!("{}/{}", prefix, name) };
                let Some(cid) = id_of(c, "id") else { continue };
                match str_of(c, "type").as_str() {
                    "folder" if depth < 8 => stack.push((cid, path, depth + 1)),
                    "file" => out.push(RemoteFile { id: cid, path, size: num(c, "size").max(0.0) as u64 }),
                    _ => {}
                }
            }
        }
        out.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(out)
    }

    async fn item(key: &str, id: &str) -> Result<Value, DebridError> {
        send(API.get(format!("{}/item/details", base())).query(&[("apikey", key), ("id", id)])).await
    }

    pub async fn link(key: &str, file_id: &str) -> Result<String, DebridError> {
        let l = str_of(&item(key, file_id).await?, "link");
        if l.starts_with("http") {
            Ok(l)
        } else {
            Err(DebridError::new(ErrKind::Transient, "Premiumize no devolvió el enlace de descarga."))
        }
    }

    pub async fn remove(key: &str, id: &str, folder: &str) -> Result<(), DebridError> {
        let post = |path: &str, target: &str| {
            API.post(format!("{}/{}", base(), path)).query(&[("apikey", key)]).form(&[("id", target)])
        };
        if let Some(file) = folder.strip_prefix("file:") {
            send(post("item/delete", file)).await?;
        } else if !folder.is_empty() {
            send(post("folder/delete", folder)).await?;
        }
        if !id.is_empty() {
            // La transferencia terminada solo es una entrada en la lista: si falla, no importa.
            let _ = send(post("transfer/delete", id)).await;
        }
        Ok(())
    }

    pub async fn account(key: &str) -> Result<(String, Option<String>), DebridError> {
        let v = send(API.get(format!("{}/account/info", base())).query(&[("apikey", key)])).await?;
        Ok(describe_account(&v))
    }

    pub(super) fn describe_account(v: &Value) -> (String, Option<String>) {
        let until = num(v, "premium_until") as u64;
        let used = num(v, "limit_used");
        let mut msg = if until > 0 { format!("Conectado · premium hasta {}", fmt_date(until)) } else { "Conectado".to_string() };
        if used > 0.0 {
            msg.push_str(&format!(" · {:.0} % del límite usado", percent(used)));
        }
        let warning = (until == 0).then(|| "La cuenta no es premium: Premiumize solo descarga NZB con cuenta premium.".to_string());
        (msg, warning)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn hash() {
        assert_eq!(nzb_hash(b""), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(nzb_hash(b"nebula"), format!("{:x}", Md5::digest(b"nebula")));
    }

    #[test]
    fn paths() {
        assert_eq!(clean_path("a\\b/../c//./d.mkv"), "a/b/c/d.mkv");
        let f = |p: &str| RemoteFile { id: "1".into(), path: p.into(), size: 1 };
        let out = strip_common_root(vec![f("Rel/a.mkv"), f("Rel/Subs/b.srt")]);
        assert_eq!(out.iter().map(|x| x.path.as_str()).collect::<Vec<_>>(), vec!["a.mkv", "Subs/b.srt"]);
        let out = strip_common_root(vec![f("Rel/a.mkv"), f("Otra/b.srt")]);
        assert_eq!(out[1].path, "Otra/b.srt");
        let out = strip_common_root(vec![f("a.mkv"), f("")]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].path, "a.mkv");
    }

    #[test]
    fn dates() {
        assert_eq!(fmt_date(0), "01/01/1970");
        assert_eq!(fmt_date(1_790_000_000), "21/09/2026");
    }

    #[test]
    fn torbox_envelope() {
        let ok = torbox::envelope(200, br#"{"success":true,"error":null,"detail":"ok","data":{"usenetdownload_id":42,"hash":"abc"}}"#).unwrap();
        assert_eq!(torbox::parse_created(&ok, "x"), Submitted { id: "42".into(), hash: "abc".into() });
        let queued = json!({ "queued_id": 7, "hash": "" });
        assert_eq!(torbox::parse_created(&queued, "h"), Submitted { id: String::new(), hash: "h".into() });
        let e = torbox::envelope(403, br#"{"success":false,"error":"PLAN_RESTRICTED_FEATURE","detail":"x","data":null}"#).unwrap_err();
        assert_eq!(e.kind, ErrKind::Fatal);
        assert!(e.msg.contains("plan Pro"));
        let e = torbox::envelope(401, br#"{"success":false,"error":"BAD_TOKEN","detail":"x"}"#).unwrap_err();
        assert_eq!(e.kind, ErrKind::Auth);
        let e = torbox::envelope(429, br#"{"success":false,"error":"ACTIVE_LIMIT","detail":"x"}"#).unwrap_err();
        assert_eq!(e.kind, ErrKind::Busy);
        let e = torbox::envelope(502, b"<html>Bad gateway</html>").unwrap_err();
        assert_eq!(e.kind, ErrKind::Transient);
        let e = torbox::envelope(400, br#"{"success":false,"error":"DUPLICATE_ITEM","detail":"x"}"#).unwrap_err();
        assert_eq!(e.code, "DUPLICATE_ITEM");
    }

    #[test]
    fn torbox_items() {
        let working = json!({
            "id": 5, "download_state": "downloading", "progress": 0.42, "download_speed": 125829120, "eta": 30,
            "download_finished": false, "download_present": false, "files": []
        });
        assert_eq!(
            torbox::parse_item(&working),
            RemoteState::Working { percent: 42.0, speed: 125_829_120, eta: 30, state: "Descargando".into() }
        );
        let ready = json!({
            "id": 5, "download_state": "completed", "progress": 1, "download_finished": true, "download_present": true,
            "files": [
                { "id": 0, "name": "Mi.Release/Mi.Release.mkv", "short_name": "Mi.Release.mkv", "size": 1000 },
                { "id": 1, "name": "Mi.Release/Subs/es.srt", "short_name": "es.srt", "size": 10 }
            ]
        });
        match torbox::parse_item(&ready) {
            RemoteState::Ready { files, .. } => {
                assert_eq!(files.len(), 2);
                assert_eq!(files[0], RemoteFile { id: "0".into(), path: "Mi.Release.mkv".into(), size: 1000 });
                assert_eq!(files[1].path, "Subs/es.srt");
            }
            other => panic!("{:?}", other),
        }
        // Terminado pero aún sin copiar a su almacenamiento: se sigue esperando.
        let unpacking = json!({ "download_state": "Direct Unpack: Completed", "download_finished": true, "download_present": false,
            "files": [{ "id": 0, "name": "a/b.mkv", "size": 1 }] });
        assert!(matches!(torbox::parse_item(&unpacking), RemoteState::Working { .. }));
        let failed = json!({ "download_state": "Failed: not enough repair blocks", "download_finished": false });
        assert!(matches!(torbox::parse_item(&failed), RemoteState::Failed(m) if m.contains("repair blocks")));
    }

    #[test]
    fn torbox_account() {
        let (msg, warn) = torbox::describe_account(&json!({ "email": "a@b.c", "plan": 2, "premium_expires_at": "2026-12-01T00:00:00Z" }));
        assert_eq!(msg, "Conectado como a@b.c · plan Pro hasta 01/12/2026");
        assert!(warn.is_none());
        let (_, warn) = torbox::describe_account(&json!({ "plan": 1 }));
        assert!(warn.is_some());
    }

    #[test]
    fn premiumize() {
        let e = premiumize::envelope(200, br#"{"status":"error","message":"Not logged in."}"#).unwrap_err();
        assert_eq!(e.kind, ErrKind::Auth);
        let e = premiumize::envelope(200, br#"{"status":"error","message":"You already added this job."}"#).unwrap_err();
        assert_eq!(e.code, "DUPLICATE");
        assert!(premiumize::envelope(200, br#"{"status":"success","id":"abc"}"#).is_ok());

        let running = json!({ "id": "t1", "status": "running", "progress": 0.5,
            "message": "Downloading at 63.9 MB/s. 50% of 8.7 GB finished." });
        match premiumize::parse_transfer(&running) {
            premiumize::Transfer::State(RemoteState::Working { percent, speed, .. }) => {
                assert_eq!(percent, 50.0);
                assert_eq!(speed, (63.9 * 1024.0 * 1024.0) as u64);
            }
            _ => panic!("estado inesperado"),
        }
        let done = json!({ "id": "t1", "status": "finished", "folder_id": "f9", "file_id": null });
        assert!(matches!(premiumize::parse_transfer(&done), premiumize::Transfer::Folder(f) if f == "f9"));
        let single = json!({ "id": "t1", "status": "finished", "folder_id": "", "file_id": "x1" });
        assert!(matches!(premiumize::parse_transfer(&single), premiumize::Transfer::File(f) if f == "x1"));
        let err = json!({ "id": "t1", "status": "error", "message": "Not enough parity." });
        assert!(matches!(premiumize::parse_transfer(&err), premiumize::Transfer::State(RemoteState::Failed(m)) if m.contains("parity")));

        let (msg, warn) = premiumize::describe_account(&json!({ "status": "success", "premium_until": 1_790_000_000u64, "limit_used": 0.25 }));
        assert_eq!(msg, "Conectado · premium hasta 21/09/2026 · 25 % del límite usado");
        assert!(warn.is_none());
    }
}
