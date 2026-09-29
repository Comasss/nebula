use std::time::Duration;

pub struct Fetched {
    pub data: Vec<u8>,
    pub filename: Option<String>,
    /// true si el nombre viene de Content-Disposition (fiable)
    pub strong: bool,
}

fn describe(e: reqwest::Error) -> String {
    if e.is_timeout() {
        "Tiempo de espera agotado al descargar el NZB.".into()
    } else if e.is_connect() {
        "No se pudo conectar con el servidor del enlace. Comprueba tu conexión o la URL.".into()
    } else if e.is_redirect() {
        "Demasiadas redirecciones en el enlace.".into()
    } else if e.is_builder() {
        "La URL no es válida.".into()
    } else {
        // Sin la URL: puede contener la API key del indexador.
        format!("Error al descargar el NZB: {}", e.without_url())
    }
}

fn content_disposition_name(v: &str) -> Option<String> {
    // filename*=UTF-8''nombre.nzb tiene preferencia
    for part in v.split(';') {
        let p = part.trim();
        if let Some(rest) = p.strip_prefix("filename*=") {
            let rest = rest.trim_matches('"');
            let enc = rest.splitn(3, '\'').nth(2).unwrap_or(rest);
            return Some(percent_decode(enc));
        }
    }
    for part in v.split(';') {
        let p = part.trim();
        if let Some(rest) = p.strip_prefix("filename=") {
            return Some(rest.trim_matches('"').to_string());
        }
    }
    None
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(if b[i] == b'+' { b' ' } else { b[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

pub async fn fetch(url: &str) -> Result<Fetched, String> {
    let url = url.trim();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("El enlace debe empezar por http:// o https://".into());
    }
    let client = reqwest::Client::builder()
        .user_agent(concat!("Nebula/", env!("CARGO_PKG_VERSION"), " (Usenet downloader)"))
        .redirect(reqwest::redirect::Policy::limited(10))
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(90))
        .build()
        .map_err(describe)?;
    let resp = client.get(url).send().await.map_err(describe)?;
    let status = resp.status();
    if !status.is_success() {
        return Err(match status.as_u16() {
            401 | 403 => format!("Acceso denegado (HTTP {}): el enlace requiere iniciar sesión o una API key válida.", status.as_u16()),
            404 | 410 => "El NZB no existe en esa dirección (HTTP 404).".into(),
            429 => "El indexador ha limitado las descargas (HTTP 429). Inténtalo más tarde.".into(),
            500..=599 => format!("El servidor del enlace tiene un error (HTTP {}).", status.as_u16()),
            c => format!("El servidor respondió con un error HTTP {}.", c),
        });
    }
    if let Some(len) = resp.content_length() {
        if len > 200 * 1024 * 1024 {
            return Err("El archivo es demasiado grande para ser un NZB.".into());
        }
    }
    let cd = resp
        .headers()
        .get(reqwest::header::CONTENT_DISPOSITION)
        .and_then(|v| v.to_str().ok())
        .and_then(content_disposition_name);
    let final_url = resp.url().clone();
    let data = resp.bytes().await.map_err(describe)?.to_vec();
    let (filename, strong) = match cd {
        Some(n) => (Some(n), true),
        None => (
            final_url
                .path_segments()
                .and_then(|mut s| s.next_back())
                .filter(|s| !s.is_empty())
                .map(percent_decode),
            false,
        ),
    };
    Ok(Fetched { data, filename, strong })
}

/// Explica por qué un contenido descargado no es un NZB.
pub fn explain_not_nzb(data: &[u8]) -> String {
    let head = String::from_utf8_lossy(&data[..data.len().min(8192)]).to_string();
    let lower = head.to_ascii_lowercase();
    if let Some(pos) = lower.find("<error") {
        let frag = &head[pos..];
        let attr = |name: &str| {
            let key = format!("{}=\"", name);
            frag.find(&key).and_then(|d| {
                let rest = &frag[d + key.len()..];
                rest.find('"').map(|end| rest[..end].to_string())
            })
        };
        let code = attr("code").unwrap_or_default();
        let desc = attr("description").unwrap_or_default();
        if code.is_empty() && desc.is_empty() {
            return "El indexador devolvió un error en lugar del NZB.".into();
        }
        return crate::indexer::newznab_error(&code, &desc);
    }
    if lower.contains("<html") || lower.contains("<!doctype html") {
        return "El enlace devuelve una página web, no un NZB. Puede que requiera iniciar sesión o que no sea un enlace directo.".into();
    }
    if lower.trim_start().starts_with('{') {
        return "El enlace devuelve datos JSON, no un NZB.".into();
    }
    "El contenido descargado no es un archivo NZB válido.".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cd() {
        assert_eq!(content_disposition_name("attachment; filename=\"My.Release.nzb\"").as_deref(), Some("My.Release.nzb"));
        assert_eq!(content_disposition_name("attachment; filename*=UTF-8''Caf%C3%A9.nzb").as_deref(), Some("Café.nzb"));
    }
}

/// Guarda un NZB en `dir` con un nombre único a partir de `name`. Devuelve la ruta final.
pub fn save_nzb_file(data: &[u8], dir: &str, name: &str) -> Result<String, String> {
    let dir = std::path::PathBuf::from(dir.trim());
    if dir.as_os_str().is_empty() {
        return Err("No hay una carpeta configurada para guardar los .nzb.".into());
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("No se puede crear la carpeta «{}»: {}", dir.display(), e))?;
    let base = name.trim().trim_end_matches(".gz").trim_end_matches(".nzb").trim_end_matches(".NZB");
    let base = crate::util::sanitize_filename(if base.is_empty() { "descarga" } else { base });
    let mut path = dir.join(format!("{}.nzb", base));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{} ({}).nzb", base, n));
        n += 1;
    }
    std::fs::write(&path, data).map_err(|e| format!("No se pudo guardar el NZB: {}", e))?;
    crate::linfo!("NZB guardado en {}", path.display());
    Ok(path.to_string_lossy().to_string())
}
