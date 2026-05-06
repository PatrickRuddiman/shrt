use crate::cli::Ctx;
use crate::paths;
use serde::Serialize;
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

pub fn init(ctx: &Ctx) -> anyhow::Result<InitReport> {
    let created = if ctx.shim_dir.is_dir() {
        false
    } else {
        std::fs::create_dir_all(&ctx.shim_dir).map_err(|e| {
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
