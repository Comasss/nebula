use crate::engine::{Cancel, Engine, JobOptions, JobStatus};
use crate::util::{self, Speedo};
use crate::{lerror, linfo, lwarn};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, LazyLock, OnceLock};
use std::time::{Duration, Instant};
use tokio::io::AsyncReadExt;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x0000_4000;

pub struct PpCtx {
    pub id: String,
    pub name: String,
    pub work_dir: PathBuf,
    pub final_dir: PathBuf,
    pub password: Option<String>,
    pub options: JobOptions,
    pub pp_only: bool,
    pub failed_segs: u32,
    pub total: u64,
    pub cancel: Arc<Cancel>,
    pub tools: PathBuf,
}

struct PpError {
    stage: &'static str,
    msg: String,
    retry_extract: bool,
}

impl PpError {
    fn new(stage: &'static str, msg: impl Into<String>) -> Self {
        Self { stage, msg: msg.into(), retry_extract: false }
    }
}

enum Flow {
    Done(Option<String>),
    Redownload,
}

pub async fn run(engine: Arc<Engine>, id: String) {
    let Ok(_permit) = engine.pp_sem.acquire().await else { return };
    let Some(ctx) = engine.pp_context(&id) else { return };
    if ctx.cancel.is() {
        return;
    }
    linfo!("Post-procesado de «{}»", ctx.name);
    match pipeline(&engine, &ctx).await {
        Ok(Flow::Done(w)) => engine.finish_job(&id, w),
        Ok(Flow::Redownload) => {}
        Err(e) => {
            if !ctx.cancel.is() {
                engine.fail_job(&id, e.msg, e.stage, e.retry_extract);
            }
        }
    }
}

async fn pipeline(engine: &Arc<Engine>, c: &PpCtx) -> Result<Flow, PpError> {
    let mut warnings: Vec<String> = Vec::new();
    if !c.work_dir.exists() {
        return Err(PpError::new(
            "download",
            if c.pp_only {
                "La carpeta temporal ya no existe."
            } else {
                "No se ha podido descargar ningún archivo (todos los artículos faltan en el servidor)."
            },
        ));
    }

    // 1) Verificación y reparación PAR2
    if c.options.par2 && !c.pp_only {
        match find_main_par2(&c.work_dir) {
            None => {
                if c.failed_segs > 0 {
                    warnings.push(format!("Faltan {} artículos y no hay archivos PAR2 para repararlos.", c.failed_segs));
                }
            }
            Some(par2) => {
                engine.set_stage(&c.id, JobStatus::Verifying, "Verificando", 0.0, 0, "");
                let result = run_par2(engine, c, "v", &par2).await?;
                let mut need_repair = false;
                match result {
                    Par2Result::AllCorrect => linfo!("«{}»: verificación correcta", c.name),
                    Par2Result::RepairPossible => need_repair = true,
                    Par2Result::NeedBlocks(n) => {
                        if engine.unhold_vols(&c.id, n) {
                            return Ok(Flow::Redownload);
                        }
                        return Err(PpError::new(
                            "verify",
                            format!("No se puede reparar: faltan {} bloques de recuperación y no hay más archivos PAR2 disponibles.", n),
                        ));
                    }
                    Par2Result::Repaired => {}
                    Par2Result::Failed(msg) => {
                        if c.failed_segs == 0 {
                            lwarn!("«{}»: par2 no pudo verificar ({}); se continúa", c.name, msg);
                            warnings.push(format!("No se pudo verificar con PAR2: {}", msg));
                        } else {
                            return Err(PpError::new("verify", format!("Error de verificación PAR2: {}", msg)));
                        }
                    }
                }
                if need_repair {
                    let before: HashSet<PathBuf> = util::list_files(&c.work_dir).into_iter().map(|(p, _)| p).collect();
                    engine.set_stage(&c.id, JobStatus::Repairing, "Reparando", 0.0, 0, "");
                    match run_par2(engine, c, "r", &par2).await? {
                        Par2Result::Repaired | Par2Result::AllCorrect => {
                            linfo!("«{}»: reparación completada", c.name);
                            cleanup_par2_backups(&c.work_dir, &before);
                        }
                        Par2Result::NeedBlocks(n) => {
                            if engine.unhold_vols(&c.id, n) {
                                return Ok(Flow::Redownload);
                            }
                            return Err(PpError::new("repair", format!("No se puede reparar: faltan {} bloques de recuperación.", n)));
                        }
                        Par2Result::RepairPossible => {}
                        Par2Result::Failed(msg) => return Err(PpError::new("repair", format!("La reparación falló: {}", msg))),
                    }
                }
            }
        }
    } else if c.failed_segs > 0 && !c.pp_only {
        warnings.push(format!("Faltan {} artículos (verificación PAR2 desactivada).", c.failed_segs));
    }

    // 2) Extracción
    let mut extracted: HashSet<PathBuf> = HashSet::new();
    if c.options.extract {
        let sets = find_archives(&c.work_dir);
        let total: u64 = sets.iter().map(|s| s.size).sum::<u64>().max(1);
        let mut before = 0u64;
        let n = sets.len();
        for (i, set) in sets.iter().enumerate() {
            let detail = if n > 1 { format!("{} ({}/{})", set.label, i + 1, n) } else { set.label.clone() };
            engine.set_stage(&c.id, JobStatus::Extracting, "Extrayendo", before as f64 / total as f64 * 100.0, 0, &detail);
            extract_set(engine, c, set, before, total, &detail).await?;
            linfo!("«{}»: extraído {}", c.name, set.label);
            before += set.size;
            extracted.extend(set.members.iter().cloned());
        }
    }

    // 3) Mover al destino final
    engine.set_stage(&c.id, JobStatus::Moving, "Moviendo archivos", 0.0, 0, "");
    let e2 = engine.clone();
    let id = c.id.clone();
    let work = c.work_dir.clone();
    let fin = c.final_dir.clone();
    let opts = c.options.clone();
    tokio::task::spawn_blocking(move || -> Result<(), PpError> {
        std::fs::create_dir_all(&fin)
            .map_err(|e| PpError::new("move", format!("No se puede crear la carpeta de destino: {}", e)))?;
        let par2 = par2_files(&work);
        let files = util::list_files(&work);
        let to_move: Vec<&(PathBuf, u64)> = files
            .iter()
            .filter(|(p, _)| !(opts.delete_archives && extracted.contains(p)) && !(opts.delete_par2 && par2.contains(p)))
            .collect();
        let total: u64 = to_move.iter().map(|(_, s)| *s).sum::<u64>().max(1);
        let mut done = 0u64;
        let start = Instant::now();
        for (p, size) in to_move {
            let target = fin.join(util::file_name(p));
            util::move_file(p, &target)
                .map_err(|e| PpError::new("move", format!("No se pudo mover {}: {}", util::file_name(p), e)))?;
            done += size;
            let secs = start.elapsed().as_secs_f64().max(0.001);
            e2.set_stage(
                &id,
                JobStatus::Moving,
                "Moviendo archivos",
                done as f64 / total as f64 * 100.0,
                (done as f64 / secs) as u64,
                &util::file_name(p),
            );
        }
        // Subcarpetas (poco habitual)
        if let Ok(rd) = std::fs::read_dir(&work) {
            for e in rd.flatten() {
                if e.path().is_dir() {
                    let _ = util::move_tree(&e.path(), &fin.join(e.file_name()));
                }
            }
        }
        util::remove_dir_retry(&work);
        if let Some(parent) = work.parent() {
            if parent.file_name().map(|n| n == "_incompleto").unwrap_or(false) {
                let _ = std::fs::remove_dir(parent);
            }
        }
        Ok(())
    })
    .await
    .map_err(|_| PpError::new("move", "Error interno al mover archivos"))??;

    Ok(Flow::Done(if warnings.is_empty() { None } else { Some(warnings.join(" ")) }))
}

// ---------------------------------------------------------------------------
// Procesos externos
// ---------------------------------------------------------------------------

fn job_object() -> Option<usize> {
    static JOB: OnceLock<usize> = OnceLock::new();
    let h = *JOB.get_or_init(|| unsafe {
        use windows_sys::Win32::System::JobObjects::*;
        let h = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if h.is_null() {
            return 0;
        }
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        SetInformationJobObject(
            h,
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const _,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        );
        h as usize
    });
    if h == 0 {
        None
    } else {
        Some(h)
    }
}

/// Asocia el proceso hijo a un Job Object para que muera si Nebula se cierra.
fn attach_child(child: &tokio::process::Child) {
    if let (Some(job), Some(handle)) = (job_object(), child.raw_handle()) {
        unsafe {
            windows_sys::Win32::System::JobObjects::AssignProcessToJobObject(job as _, handle as _);
        }
    }
}

fn command(exe: &Path) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(exe);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .creation_flags(CREATE_NO_WINDOW | BELOW_NORMAL_PRIORITY_CLASS);
    cmd
}

/// Lee stdout troceando por \r, \n y \x08; llama a `on_piece` con cada fragmento.
async fn pump<F: FnMut(&str)>(
    child: &mut tokio::process::Child,
    cancel: &Cancel,
    mut on_piece: F,
) -> Result<(i32, String), &'static str> {
    let mut stdout = child.stdout.take().ok_or("sin stdout")?;
    let mut stderr = child.stderr.take().ok_or("sin stderr")?;
    let err_task = tokio::spawn(async move {
        let mut s = Vec::new();
        let _ = stderr.read_to_end(&mut s).await;
        String::from_utf8_lossy(&s).to_string()
    });
    let mut buf = vec![0u8; 16 * 1024];
    let mut acc: Vec<u8> = Vec::new();
    loop {
        let n = tokio::select! {
            r = stdout.read(&mut buf) => r.unwrap_or(0),
            _ = cancel.wait() => {
                let _ = child.kill().await;
                return Err("cancelado");
            }
        };
        if n == 0 {
            break;
        }
        for &b in &buf[..n] {
            if b == b'\r' || b == b'\n' || b == 0x08 {
                if !acc.is_empty() {
                    let s = String::from_utf8_lossy(&acc).to_string();
                    acc.clear();
                    let t = s.trim();
                    if !t.is_empty() {
                        on_piece(t);
                    }
                }
            } else {
                acc.push(b);
            }
        }
    }
    if !acc.is_empty() {
        let s = String::from_utf8_lossy(&acc).to_string();
        if !s.trim().is_empty() {
            on_piece(s.trim());
        }
    }
    let status = child.wait().await.map_err(|_| "error esperando al proceso")?;
    let err = err_task.await.unwrap_or_default();
    Ok((status.code().unwrap_or(-1), err))
}

// ---------------------------------------------------------------------------
// PAR2
// ---------------------------------------------------------------------------

enum Par2Result {
    AllCorrect,
    RepairPossible,
    NeedBlocks(u32),
    Repaired,
    Failed(String),
}

static RE_NEED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"You need (\d+) more recovery blocks").unwrap());
static RE_TOTAL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"total size of the data files is (\d+) bytes").unwrap());
static RE_PROGRESS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([A-Za-z ]+):\s*([\d.]+)%").unwrap());

fn is_par2_magic(p: &Path) -> bool {
    let mut b = [0u8; 8];
    std::fs::File::open(p).and_then(|mut f| f.read_exact(&mut b)).map(|_| &b == b"PAR2\0PKT").unwrap_or(false)
}

fn par2_files(dir: &Path) -> HashSet<PathBuf> {
    util::list_files(dir)
        .into_iter()
        .filter(|(p, _)| {
            util::file_name(p).to_ascii_lowercase().ends_with(".par2") || is_par2_magic(p)
        })
        .map(|(p, _)| p)
        .collect()
}

fn find_main_par2(dir: &Path) -> Option<PathBuf> {
    let files = util::list_files(dir);
    let mut named: Vec<&(PathBuf, u64)> = files.iter().filter(|(p, _)| util::file_name(p).to_ascii_lowercase().ends_with(".par2")).collect();
    if named.is_empty() {
        let mut magic: Vec<&(PathBuf, u64)> = files.iter().filter(|(p, _)| is_par2_magic(p)).collect();
        magic.sort_by_key(|(_, s)| *s);
        return magic.first().map(|(p, _)| p.clone());
    }
    named.sort_by_key(|(p, s)| (crate::nzb::vol_blocks_of(&util::file_name(p)).is_some(), *s));
    named.first().map(|(p, _)| p.clone())
}

fn cleanup_par2_backups(dir: &Path, before: &HashSet<PathBuf>) {
    let re = Regex::new(r"^(.+)\.\d+$").unwrap();
    for (p, _) in util::list_files(dir) {
        if before.contains(&p) {
            continue;
        }
        let name = util::file_name(&p);
        if let Some(c) = re.captures(&name) {
            if dir.join(&c[1]).exists() {
                let _ = std::fs::remove_file(&p);
            }
        }
    }
}

async fn run_par2(engine: &Arc<Engine>, c: &PpCtx, mode: &str, par2: &Path) -> Result<Par2Result, PpError> {
    let exe = c.tools.join("par2.exe");
    let mut cmd = command(&exe);
    cmd.current_dir(&c.work_dir).arg(mode).arg(util::file_name(par2));
    // Archivos extra para detectar archivos con nombre cambiado (ofuscados).
    let par2_set = par2_files(&c.work_dir);
    let extras: Vec<String> = util::list_files(&c.work_dir)
        .into_iter()
        .filter(|(p, _)| !par2_set.contains(p))
        .map(|(p, _)| util::file_name(&p))
        .collect();
    if extras.iter().map(|s| s.len() + 3).sum::<usize>() < 24_000 {
        cmd.args(&extras);
    }
    let stage_err: &'static str = if mode == "r" { "repair" } else { "verify" };
    let mut child = cmd
        .spawn()
        .map_err(|e| PpError::new(stage_err, format!("No se pudo ejecutar par2: {}", e)))?;
    attach_child(&child);

    let mut total_data = c.total.max(1);
    let mut text = String::new();
    let mut speedo = Speedo::new(2500);
    let mut last_stage = String::new();
    let mut last_emit = Instant::now() - Duration::from_secs(1);
    let mut verifying_repair = false;
    let status_for = |label: &str| if label == "Reparando" { JobStatus::Repairing } else if mode == "r" { JobStatus::Repairing } else { JobStatus::Verifying };

    let res = pump(&mut child, &c.cancel, |piece| {
        if let Some(cap) = RE_PROGRESS.captures(piece) {
            let key = cap[1].trim().to_string();
            let pct: f64 = cap[2].parse().unwrap_or(0.0);
            let label = match key.as_str() {
                "Loading" => "Cargando PAR2",
                "Scanning" if verifying_repair => "Comprobando reparación",
                "Scanning" => "Verificando",
                "Repairing" => "Reparando",
                "Constructing" | "Solving" => "Calculando reparación",
                _ => "Verificando",
            };
            if label != last_stage {
                last_stage = label.to_string();
                speedo.reset();
            }
            let bytes = (pct / 100.0 * total_data as f64) as u64;
            speedo.push(bytes);
            if last_emit.elapsed() >= Duration::from_millis(250) {
                last_emit = Instant::now();
                let speed = if matches!(label, "Verificando" | "Reparando" | "Comprobando reparación") { speedo.rate() } else { 0 };
                engine.set_stage(&c.id, status_for(label), label, pct, speed, "");
            }
            return;
        }
        if let Some(cap) = RE_TOTAL.captures(piece) {
            total_data = cap[1].parse().unwrap_or(total_data).max(1);
        }
        if piece.starts_with("Verifying repaired files") {
            verifying_repair = true;
        }
        text.push_str(piece);
        text.push('\n');
    })
    .await;

    let (code, stderr) = match res {
        Ok(r) => r,
        Err(e) => return Err(PpError::new(stage_err, format!("par2 interrumpido ({})", e))),
    };
    let all = format!("{}\n{}", text, stderr);
    let need = RE_NEED.captures(&all).and_then(|c| c[1].parse::<u32>().ok());
    let result = match (mode, code) {
        ("v", 0) => Par2Result::AllCorrect,
        ("v", 1) => Par2Result::RepairPossible,
        (_, 2) if need.is_some() => Par2Result::NeedBlocks(need.unwrap_or(0)),
        ("r", 0) => {
            if all.contains("All files are correct") {
                Par2Result::AllCorrect
            } else {
                Par2Result::Repaired
            }
        }
        (_, code) => {
            let reason = match code {
                2 => "no hay bloques suficientes para reparar",
                3 => "argumentos no válidos",
                4 => "los archivos PAR2 no contienen los datos críticos necesarios",
                5 => "la reparación falló",
                6 => "error de lectura/escritura de archivos",
                7 => "error interno de par2",
                8 => "memoria insuficiente",
                _ => "error desconocido",
            };
            let last = all
                .lines()
                .rev()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("")
                .chars()
                .take(160)
                .collect::<String>();
            lerror!("par2 ({}) código {}: {}", mode, code, last);
            Par2Result::Failed(format!("{} (código {})", reason, code))
        }
    };
    Ok(result)
}

// ---------------------------------------------------------------------------
// Extracción (7-Zip)
// ---------------------------------------------------------------------------

pub struct ArchiveSet {
    pub first: PathBuf,
    pub members: Vec<PathBuf>,
    pub size: u64,
    pub label: String,
}

static RE_PART: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^(.*)\.part(\d+)\.rar$").unwrap());
static RE_RAR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^(.*)\.rar$").unwrap());
static RE_ZOLD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^(.*)\.z(\d{2})$").unwrap());
static RE_ROLD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^(.*)\.[r-y](\d{2})$").unwrap());
static RE_7ZSPLIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^(.*)\.7z\.(\d{3})$").unwrap());
static RE_7Z: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^(.*)\.7z$").unwrap());
static RE_ZIP: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^(.*)\.zip$").unwrap());
static RE_SPLIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^(.*)\.(\d{3})$").unwrap());

pub fn find_archives(dir: &Path) -> Vec<ArchiveSet> {
    struct Acc {
        first: Option<(u32, PathBuf)>,
        members: Vec<PathBuf>,
        size: u64,
        label: String,
    }
    let mut sets: HashMap<(u8, String), Acc> = HashMap::new();
    let mut add = |kind: u8, base: &str, path: &PathBuf, size: u64, first_rank: Option<u32>, label: String| {
        let acc = sets.entry((kind, base.to_ascii_lowercase())).or_insert(Acc { first: None, members: Vec::new(), size: 0, label });
        acc.members.push(path.clone());
        acc.size += size;
        if let Some(r) = first_rank {
            if acc.first.as_ref().map_or(true, |(fr, _)| r < *fr) {
                acc.first = Some((r, path.clone()));
            }
        }
    };
    for (p, size) in util::list_files(dir) {
        let name = util::file_name(&p);
        if name.to_ascii_lowercase().ends_with(".par2") {
            continue;
        }
        if let Some(c) = RE_PART.captures(&name) {
            let n: u32 = c[2].parse().unwrap_or(9999);
            add(0, &c[1], &p, size, Some(n), format!("{}.rar", &c[1]));
        } else if let Some(c) = RE_RAR.captures(&name) {
            add(1, &c[1], &p, size, Some(0), name.clone());
        } else if let Some(c) = RE_ZOLD.captures(&name) {
            add(3, &c[1], &p, size, None, format!("{}.zip", &c[1]));
        } else if let Some(c) = RE_ROLD.captures(&name) {
            add(1, &c[1], &p, size, None, format!("{}.rar", &c[1]));
        } else if let Some(c) = RE_7ZSPLIT.captures(&name) {
            let n: u32 = c[2].parse().unwrap_or(9999);
            add(2, &c[1], &p, size, if n == 1 { Some(0) } else { None }, format!("{}.7z", &c[1]));
        } else if let Some(c) = RE_7Z.captures(&name) {
            add(2, &c[1], &p, size, Some(0), name.clone());
        } else if let Some(c) = RE_ZIP.captures(&name) {
            add(3, &c[1], &p, size, Some(0), name.clone());
        } else if let Some(c) = RE_SPLIT.captures(&name) {
            let n: u32 = c[2].parse().unwrap_or(9999);
            add(4, &c[1], &p, size, if n == 1 { Some(0) } else { None }, c[1].to_string());
        }
    }
    let mut out: Vec<ArchiveSet> = sets
        .into_values()
        .filter_map(|a| a.first.map(|(_, f)| ArchiveSet { first: f, members: a.members, size: a.size, label: a.label }))
        .collect();
    out.sort_by(|a, b| a.first.cmp(&b.first));
    out
}

static RE_PCT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\d{1,3})%").unwrap());

async fn extract_set(engine: &Arc<Engine>, c: &PpCtx, set: &ArchiveSet, before: u64, total: u64, detail: &str) -> Result<(), PpError> {
    std::fs::create_dir_all(&c.final_dir)
        .map_err(|e| PpError::new("extract", format!("No se puede crear la carpeta de destino: {}", e)))?;
    let exe = c.tools.join("7z.exe");
    let pw = c.password.clone().filter(|p| !p.is_empty()).unwrap_or_else(|| "nebula-no-password".into());
    let mut cmd = command(&exe);
    cmd.current_dir(&c.work_dir)
        .arg("x")
        .arg("-y")
        .arg("-aoa")
        .arg("-bsp1")
        .arg("-bso1")
        .arg("-bse2")
        .arg("-sccUTF-8")
        .arg(format!("-p{}", pw))
        .arg(format!("-o{}", c.final_dir.display()))
        .arg(&set.first);
    let mut child = cmd
        .spawn()
        .map_err(|e| PpError::new("extract", format!("No se pudo ejecutar 7-Zip: {}", e)))?;
    attach_child(&child);

    let mut speedo = Speedo::new(2500);
    let mut last_emit = Instant::now() - Duration::from_secs(1);
    let mut text = String::new();
    let res = pump(&mut child, &c.cancel, |piece| {
        if let Some(cap) = RE_PCT.captures(piece) {
            let pct: f64 = cap[1].parse().unwrap_or(0.0);
            let done = before + (pct / 100.0 * set.size as f64) as u64;
            speedo.push(done);
            if last_emit.elapsed() >= Duration::from_millis(250) {
                last_emit = Instant::now();
                engine.set_stage(&c.id, JobStatus::Extracting, "Extrayendo", done as f64 / total as f64 * 100.0, speedo.rate(), detail);
            }
        } else {
            text.push_str(piece);
            text.push('\n');
        }
    })
    .await;
    let (code, stderr) = match res {
        Ok(r) => r,
        Err(e) => return Err(PpError::new("extract", format!("7-Zip interrumpido ({})", e))),
    };
    if code == 0 {
        return Ok(());
    }
    let all = format!("{}\n{}", text, stderr);
    if code == 1 {
        lwarn!("7-Zip terminó con avisos al extraer {}: {}", set.label, stderr.trim());
        return Ok(());
    }
    let lower = all.to_ascii_lowercase();
    let msg = if lower.contains("wrong password") || lower.contains("encrypted archive") {
        if c.password.as_deref().map_or(true, |p| p.is_empty()) {
            "El archivo está protegido con contraseña. Introdúcela y reintenta la extracción.".to_string()
        } else {
            "Contraseña incorrecta para el archivo comprimido.".to_string()
        }
    } else if lower.contains("not enough space") || lower.contains("disk is full") {
        "No hay espacio suficiente en el disco de destino.".to_string()
    } else if lower.contains("unexpected end of archive") || lower.contains("missing volume") {
        format!("Faltan volúmenes o {} está incompleto.", set.label)
    } else if lower.contains("crc failed") || lower.contains("data error") {
        format!("Datos corruptos (error CRC) en {}.", set.label)
    } else if lower.contains("can not open the file as archive") || lower.contains("can't open as archive") || lower.contains("cannot open the file as archive") {
        format!("No se pudo abrir {} como archivo comprimido.", set.label)
    } else {
        let last = stderr.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("").trim().to_string();
        format!("Error al extraer {} (código {}){}", set.label, code, if last.is_empty() { String::new() } else { format!(": {}", last) })
    };
    lerror!("7-Zip ({}): {}", set.label, all.trim().lines().rev().take(4).collect::<Vec<_>>().join(" | "));
    Err(PpError { stage: "extract", msg, retry_extract: true })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archive_sets() {
        let dir = std::env::temp_dir().join(format!("nebula-test-{}", util::new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        for n in [
            "a.part01.rar", "a.part02.rar", "a.part03.rar", "b.rar", "b.r00", "b.r01", "c.7z.001", "c.7z.002",
            "d.zip", "movie.mkv", "x.par2", "x.vol00+01.par2", "e.001", "e.002",
        ] {
            std::fs::write(dir.join(n), b"x").unwrap();
        }
        let sets = find_archives(&dir);
        let firsts: Vec<String> = sets.iter().map(|s| util::file_name(&s.first)).collect();
        assert_eq!(firsts, vec!["a.part01.rar", "b.rar", "c.7z.001", "d.zip", "e.001"]);
        assert_eq!(sets[0].members.len(), 3);
        assert_eq!(sets[1].members.len(), 3);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
