use crate::cli::Ctx;
use crate::config::{self, SidecarConfig};
use crate::paths;
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
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct PathReport {
    pub path: PathBuf,
    pub on_path: bool,
}

#[derive(Debug)]
pub enum ShimError {
    Collision(String),
    Missing(String),
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
    Ok(InitReport {
        shim_dir: ctx.shim_dir.clone(),
        created,
        on_path: paths::is_on_path(&ctx.shim_dir),
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
