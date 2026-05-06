use std::path::{Path, PathBuf};

pub fn shim_dir(override_: Option<&Path>) -> anyhow::Result<PathBuf> {
    if let Some(p) = override_ {
        return Ok(p.to_path_buf());
    }
    let user_dirs = directories::UserDirs::new()
        .ok_or_else(|| anyhow::anyhow!("could not determine user home directory"))?;
    Ok(user_dirs.home_dir().join(".shrt").join("bin"))
}
