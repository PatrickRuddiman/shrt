use crate::cli::Ctx;
use crate::config::{self, Entry, SidecarConfig};
use crate::paths;
use crate::win_path;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

pub const RUNNER_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/shrt-runner.exe"));

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct InitReport {
    pub shim_dir: PathBuf,
    pub created: bool,
    pub on_path: bool,
    pub path_added: bool,
    pub path_already_present: bool,
    pub path_error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct PathReport {
    pub path: PathBuf,
    pub on_path: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SyncReport {
    pub updated: usize,
    pub total: usize,
    pub errors: Vec<(String, String)>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Ok,
    Warn,
    Fail,
}

#[derive(Debug, Serialize)]
pub struct Check {
    pub name: String,
    pub status: Status,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct DoctorReport {
    pub summary: Status,
    pub checks: Vec<Check>,
}

#[derive(Debug)]
pub enum ShimError {
    Collision(String),
    Missing(String),
    ParseError(PathBuf, anyhow::Error),
    Io(anyhow::Error),
}

impl std::fmt::Display for ShimError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Collision(name) => write!(
                f,
                "shim '{}' already exists; use --force to overwrite",
                name
            ),
            Self::Missing(name) => write!(f, "shim '{}' does not exist", name),
            Self::ParseError(path, e) => write!(f, "{}: {:#}", path.display(), e),
            Self::Io(e) => write!(f, "{:#}", e),
        }
    }
}

impl std::error::Error for ShimError {}

impl ShimError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Collision(_) => 73,
            Self::Missing(_) => 66,
            Self::ParseError(_, _) => 78,
            Self::Io(_) => 1,
        }
    }
}

pub fn init(ctx: &Ctx) -> anyhow::Result<InitReport> {
    let created = if ctx.shim_dir.is_dir() {
        false
    } else {
        fs::create_dir_all(&ctx.shim_dir).map_err(|e| {
            anyhow::anyhow!("creating shim dir {}: {}", ctx.shim_dir.display(), e)
        })?;
        true
    };

    let (path_added, path_already_present, path_error) =
        match win_path::ensure_on_user_path(&ctx.shim_dir) {
            Ok(change) => (change.added, change.already_present, None),
            Err(e) => (false, false, Some(format!("{:#}", e))),
        };

    Ok(InitReport {
        shim_dir: ctx.shim_dir.clone(),
        created,
        on_path: paths::is_on_path(&ctx.shim_dir),
        path_added,
        path_already_present,
        path_error,
    })
}

pub fn path_report(ctx: &Ctx) -> PathReport {
    PathReport {
        path: ctx.shim_dir.clone(),
        on_path: paths::is_on_path(&ctx.shim_dir),
    }
}

pub fn add(
    ctx: &Ctx,
    name: &str,
    cfg: &SidecarConfig,
    force: bool,
) -> Result<(), ShimError> {
    fs::create_dir_all(&ctx.shim_dir).map_err(|e| {
        ShimError::Io(anyhow::anyhow!(
            "creating shim dir {}: {}",
            ctx.shim_dir.display(),
            e
        ))
    })?;

    let exe = ctx.shim_dir.join(format!("{}.exe", name));
    let sidecar = ctx.shim_dir.join(format!("{}.shrt", name));

    if !force && (exe.exists() || sidecar.exists()) {
        return Err(ShimError::Collision(name.to_string()));
    }

    let exe_tmp = ctx.shim_dir.join(format!("{}.exe.tmp", name));
    let sidecar_tmp = ctx.shim_dir.join(format!("{}.shrt.tmp", name));

    let body = config::serialize_sidecar(cfg).map_err(ShimError::Io)?;

    if let Err(e) = fs::write(&sidecar_tmp, body.as_bytes()) {
        return Err(ShimError::Io(anyhow::anyhow!(
            "writing {}: {}",
            sidecar_tmp.display(),
            e
        )));
    }

    if let Err(e) = fs::write(&exe_tmp, ctx.runner_bytes) {
        let _ = fs::remove_file(&sidecar_tmp);
        return Err(ShimError::Io(anyhow::anyhow!(
            "writing {}: {}",
            exe_tmp.display(),
            e
        )));
    }

    if let Err(e) = fs::rename(&sidecar_tmp, &sidecar) {
        let _ = fs::remove_file(&sidecar_tmp);
        let _ = fs::remove_file(&exe_tmp);
        return Err(ShimError::Io(anyhow::anyhow!(
            "renaming sidecar to {}: {}",
            sidecar.display(),
            e
        )));
    }

    if let Err(e) = fs::rename(&exe_tmp, &exe) {
        let _ = fs::remove_file(&exe_tmp);
        let _ = fs::remove_file(&sidecar);
        return Err(ShimError::Io(anyhow::anyhow!(
            "renaming exe to {}: {}",
            exe.display(),
            e
        )));
    }

    Ok(())
}

pub fn list(ctx: &Ctx) -> Result<Vec<Entry>, ShimError> {
    let mut entries: Vec<Entry> = Vec::new();
    if !ctx.shim_dir.is_dir() {
        return Ok(entries);
    }

    let read_dir = fs::read_dir(&ctx.shim_dir).map_err(|e| {
        ShimError::Io(anyhow::anyhow!(
            "reading {}: {}",
            ctx.shim_dir.display(),
            e
        ))
    })?;

    for dir_entry in read_dir {
        let dir_entry = dir_entry
            .map_err(|e| ShimError::Io(anyhow::anyhow!("reading dir entry: {}", e)))?;
        let path = dir_entry.path();
        let is_shrt = path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.eq_ignore_ascii_case("shrt"))
            .unwrap_or(false);
        if !is_shrt {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| {
                ShimError::Io(anyhow::anyhow!("invalid filename: {}", path.display()))
            })?
            .to_string();
        let cfg = config::read_sidecar(&path)
            .map_err(|e| ShimError::ParseError(path.clone(), e))?;
        entries.push(Entry { name, config: cfg });
    }

    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}

pub fn show(
    ctx: &Ctx,
    name: &str,
) -> Result<(PathBuf, String, Entry), ShimError> {
    let sidecar = ctx.shim_dir.join(format!("{}.shrt", name));
    if !sidecar.exists() {
        return Err(ShimError::Missing(name.to_string()));
    }

    let raw = fs::read_to_string(&sidecar).map_err(|e| {
        ShimError::Io(anyhow::anyhow!("reading {}: {}", sidecar.display(), e))
    })?;
    let cfg: SidecarConfig = toml::from_str(&raw).map_err(|e| {
        ShimError::ParseError(sidecar.clone(), anyhow::anyhow!("{}", e))
    })?;
    let entry = Entry {
        name: name.to_string(),
        config: cfg,
    };
    Ok((sidecar, raw, entry))
}

pub fn sync(ctx: &Ctx) -> Result<SyncReport, ShimError> {
    let mut updated = 0usize;
    let mut total = 0usize;
    let mut errors: Vec<(String, String)> = Vec::new();

    if !ctx.shim_dir.is_dir() {
        return Ok(SyncReport {
            updated,
            total,
            errors,
        });
    }

    let read_dir = fs::read_dir(&ctx.shim_dir).map_err(|e| {
        ShimError::Io(anyhow::anyhow!(
            "reading {}: {}",
            ctx.shim_dir.display(),
            e
        ))
    })?;

    for dir_entry in read_dir {
        let dir_entry = dir_entry
            .map_err(|e| ShimError::Io(anyhow::anyhow!("reading dir entry: {}", e)))?;
        let path = dir_entry.path();
        let is_shrt = path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.eq_ignore_ascii_case("shrt"))
            .unwrap_or(false);
        if !is_shrt {
            continue;
        }
        let name = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };

        total += 1;
        let exe = ctx.shim_dir.join(format!("{}.exe", name));

        if !exe.exists() {
            errors.push((name, "missing .exe".to_string()));
            continue;
        }

        let current = match fs::read(&exe) {
            Ok(b) => b,
            Err(e) => {
                errors.push((name, format!("reading {}: {}", exe.display(), e)));
                continue;
            }
        };

        if current == ctx.runner_bytes {
            continue;
        }

        let exe_tmp = ctx.shim_dir.join(format!("{}.exe.tmp", name));
        if let Err(e) = fs::write(&exe_tmp, ctx.runner_bytes) {
            errors.push((name, format!("writing {}: {}", exe_tmp.display(), e)));
            continue;
        }
        if let Err(e) = fs::rename(&exe_tmp, &exe) {
            let _ = fs::remove_file(&exe_tmp);
            errors.push((name, format!("renaming exe: {}", e)));
            continue;
        }
        updated += 1;
    }

    Ok(SyncReport {
        updated,
        total,
        errors,
    })
}

pub fn doctor(ctx: &Ctx) -> anyhow::Result<DoctorReport> {
    let mut checks: Vec<Check> = Vec::new();

    let on_path = paths::is_on_path(&ctx.shim_dir);
    checks.push(Check {
        name: "path".to_string(),
        status: if on_path { Status::Ok } else { Status::Warn },
        message: if on_path {
            format!("{} is on PATH", ctx.shim_dir.display())
        } else {
            format!(
                "{} is not on PATH; run `shrt init` for instructions",
                ctx.shim_dir.display()
            )
        },
    });

    if ctx.shim_dir.is_dir() {
        let read_dir = fs::read_dir(&ctx.shim_dir).map_err(|e| {
            anyhow::anyhow!("reading {}: {}", ctx.shim_dir.display(), e)
        })?;
        let mut names: Vec<String> = Vec::new();
        for entry in read_dir {
            let entry = entry
                .map_err(|e| anyhow::anyhow!("reading dir entry: {}", e))?;
            let path = entry.path();
            let is_shrt = path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case("shrt"))
                .unwrap_or(false);
            if is_shrt {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    names.push(stem.to_string());
                }
            }
        }
        names.sort();

        for name in &names {
            let sidecar = ctx.shim_dir.join(format!("{}.shrt", name));
            let parse_result = config::read_sidecar(&sidecar);

            match &parse_result {
                Ok(_) => checks.push(Check {
                    name: format!("{}: parse", name),
                    status: Status::Ok,
                    message: "sidecar parses".to_string(),
                }),
                Err(e) => checks.push(Check {
                    name: format!("{}: parse", name),
                    status: Status::Fail,
                    message: format!("{:#}", e),
                }),
            }

            let exe = ctx.shim_dir.join(format!("{}.exe", name));
            let (bytes_status, bytes_msg) = if !exe.exists() {
                (Status::Fail, "missing .exe".to_string())
            } else {
                match fs::read(&exe) {
                    Ok(b) if b == ctx.runner_bytes => {
                        (Status::Ok, "byte-equal to embedded runner".to_string())
                    }
                    Ok(_) => (
                        Status::Fail,
                        "byte mismatch with embedded runner; run `shrt sync`"
                            .to_string(),
                    ),
                    Err(e) => (Status::Fail, format!("reading {}: {}", exe.display(), e)),
                }
            };
            checks.push(Check {
                name: format!("{}: bytes", name),
                status: bytes_status,
                message: bytes_msg,
            });

            if let Ok(cfg) = parse_result {
                let (target_status, target_msg) = match which::which(&cfg.target) {
                    Ok(p) => (Status::Ok, format!("resolves to {}", p.display())),
                    Err(e) => (
                        Status::Fail,
                        format!("'{}' not found: {}", cfg.target, e),
                    ),
                };
                checks.push(Check {
                    name: format!("{}: target", name),
                    status: target_status,
                    message: target_msg,
                });
            }
        }
    }

    checks.push(Check {
        name: "acls".to_string(),
        status: Status::Warn,
        message: "Windows user-only ACLs deferred to v0.2".to_string(),
    });

    let any_fail = checks.iter().any(|c| matches!(c.status, Status::Fail));
    let any_warn = checks.iter().any(|c| matches!(c.status, Status::Warn));
    let summary = if any_fail {
        Status::Fail
    } else if any_warn {
        Status::Warn
    } else {
        Status::Ok
    };

    Ok(DoctorReport { summary, checks })
}

pub fn remove(ctx: &Ctx, name: &str) -> Result<(), ShimError> {
    let exe = ctx.shim_dir.join(format!("{}.exe", name));
    let sidecar = ctx.shim_dir.join(format!("{}.shrt", name));

    if !exe.exists() && !sidecar.exists() {
        return Err(ShimError::Missing(name.to_string()));
    }

    if exe.exists() {
        if let Err(e) = fs::remove_file(&exe) {
            return Err(ShimError::Io(anyhow::anyhow!(
                "removing {}: {}",
                exe.display(),
                e
            )));
        }
    }
    if sidecar.exists() {
        if let Err(e) = fs::remove_file(&sidecar) {
            return Err(ShimError::Io(anyhow::anyhow!(
                "removing {}: {}",
                sidecar.display(),
                e
            )));
        }
    }
    Ok(())
}
