//! Herramientas externas embebidas en el ejecutable (7-Zip y par2cmdline-turbo).
use std::path::Path;

static FILES: [(&str, &[u8]); 4] = [
    ("7z.exe", include_bytes!("../bin/7z.exe")),
    ("7z.dll", include_bytes!("../bin/7z.dll")),
    ("par2.exe", include_bytes!("../bin/par2.exe")),
    ("nebula.png", include_bytes!("../icons/128x128@2x.png")),
];

/// Extrae las herramientas a `dir` si no existen o su tamaño no coincide.
pub fn ensure(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    for (name, data) in FILES.iter() {
        let target = dir.join(name);
        let ok = std::fs::metadata(&target).map(|m| m.len() == data.len() as u64).unwrap_or(false);
        if ok {
            continue;
        }
        let tmp = dir.join(format!("{}.tmp", name));
        std::fs::write(&tmp, data).map_err(|e| t!("err.prepareTool", name = name, error = e))?;
        let _ = std::fs::remove_file(&target);
        std::fs::rename(&tmp, &target).map_err(|e| t!("err.prepareTool", name = name, error = e))?;
    }
    Ok(())
}
