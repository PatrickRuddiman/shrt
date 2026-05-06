use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "shrt",
    version,
    about = "Parameterized command shortcuts for Windows."
)]
pub struct Cli {
    /// Override default shim directory; env: SHRT_DIR
    #[arg(long = "shim-dir", env = "SHRT_DIR", global = true)]
    pub shim_dir: Option<PathBuf>,

    /// Suppress non-error stdout
    #[arg(long, global = true)]
    pub quiet: bool,

    /// Machine-readable output where applicable
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create shim dir; print PATH status
    Init,
    /// Create a new shim
    Add(AddArgs),
    /// Delete a shim
    Remove(RemoveArgs),
    /// Print all shims
    List(ListArgs),
    /// Print sidecar contents
    Show(ShowArgs),
    /// Refresh embedded runner in every shim
    Sync,
    /// Print shim directory path
    Path,
    /// Run diagnostic checks
    Doctor,
}

#[derive(clap::Args)]
pub struct AddArgs {
    /// Shim name (e.g. wt)
    pub name: String,
    /// Argument template; first whitespace-delimited token is the target unless --target overrides
    pub template: String,
    /// Override target binary
    #[arg(long)]
    pub target: Option<String>,
    /// Run via `cmd /c` (allows pipes / redirects)
    #[arg(long)]
    pub shell: bool,
    /// Working directory; supports ~ and ${VAR} expansion at runtime
    #[arg(long)]
    pub cwd: Option<String>,
    /// Description shown in `shrt list`
    #[arg(long = "desc")]
    pub description: Option<String>,
    /// Overwrite an existing shim
    #[arg(long)]
    pub force: bool,
}

#[derive(clap::Args)]
pub struct RemoveArgs {
    pub name: String,
}

#[derive(clap::Args)]
pub struct ListArgs {
    /// Show full template, cwd, description, created
    #[arg(long, short)]
    pub verbose: bool,
}

#[derive(clap::Args)]
pub struct ShowArgs {
    pub name: String,
}

pub struct Ctx {
    pub shim_dir: PathBuf,
    pub quiet: bool,
    pub json: bool,
    pub runner_bytes: &'static [u8],
}

pub fn validate_name(name: &str) -> anyhow::Result<()> {
    if name.is_empty() {
        anyhow::bail!("invalid shim name: empty");
    }
    if name.len() > 64 {
        anyhow::bail!("invalid shim name '{}': exceeds 64 characters", name);
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
    {
        anyhow::bail!(
            "invalid shim name '{}': must match [A-Za-z0-9._-]",
            name
        );
    }
    if name.contains("..") {
        anyhow::bail!("invalid shim name '{}': '..' is not allowed", name);
    }
    let lower = name.to_ascii_lowercase();
    if matches!(lower.as_str(), "con" | "prn" | "aux" | "nul") {
        anyhow::bail!("invalid shim name '{}': reserved device name", name);
    }
    if lower.len() == 4
        && (lower.starts_with("com") || lower.starts_with("lpt"))
        && matches!(
            lower.as_bytes()[3],
            b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9'
        )
    {
        anyhow::bail!("invalid shim name '{}': reserved device name", name);
    }
    Ok(())
}

pub fn parse_template_and_target(
    template: &str,
    override_target: Option<&str>,
) -> (String, String) {
    if let Some(t) = override_target {
        return (t.to_string(), template.to_string());
    }
    let mut parts = template.splitn(2, char::is_whitespace);
    let target = parts.next().unwrap_or("").to_string();
    let body = parts.next().unwrap_or("").trim_start().to_string();
    (target, body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_name_accepts_alphanumeric() {
        assert!(validate_name("wt").is_ok());
        assert!(validate_name("wt0_0").is_ok());
        assert!(validate_name("foo-bar").is_ok());
        assert!(validate_name("a.b.c").is_ok());
    }

    #[test]
    fn validate_name_rejects_path_separator() {
        assert!(validate_name("foo/bar").is_err());
        assert!(validate_name("foo\\bar").is_err());
    }

    #[test]
    fn validate_name_rejects_double_dot() {
        assert!(validate_name("a..b").is_err());
        assert!(validate_name("..").is_err());
    }

    #[test]
    fn validate_name_rejects_empty() {
        assert!(validate_name("").is_err());
    }

    #[test]
    fn validate_name_rejects_too_long() {
        let long = "a".repeat(65);
        assert!(validate_name(&long).is_err());
        let exact = "a".repeat(64);
        assert!(validate_name(&exact).is_ok());
    }

    #[test]
    fn validate_name_rejects_reserved_devices() {
        for n in &[
            "con", "CON", "prn", "Prn", "aux", "nul", "com1", "COM9", "lpt1", "LPT9",
        ] {
            assert!(
                validate_name(n).is_err(),
                "expected '{}' to be rejected",
                n
            );
        }
    }

    #[test]
    fn validate_name_accepts_com10_and_other_non_reserved() {
        // com10 is 5 chars, com0 has digit 0
        assert!(validate_name("com10").is_ok());
        assert!(validate_name("com0").is_ok());
        assert!(validate_name("comx").is_ok());
    }

    #[test]
    fn validate_name_rejects_metacharacters() {
        for n in &["foo bar", "foo\"bar", "foo|bar", "foo;bar", "foo$bar", "foo*bar"] {
            assert!(
                validate_name(n).is_err(),
                "expected '{}' to be rejected",
                n
            );
        }
    }

    #[test]
    fn parse_template_extracts_first_token() {
        let (target, body) =
            parse_template_and_target("copilot -p '/worktree create' --yolo", None);
        assert_eq!(target, "copilot");
        assert_eq!(body, "-p '/worktree create' --yolo");
    }

    #[test]
    fn parse_template_with_override() {
        let (target, body) =
            parse_template_and_target("-p '/worktree create' --yolo", Some("copilot"));
        assert_eq!(target, "copilot");
        assert_eq!(body, "-p '/worktree create' --yolo");
    }

    #[test]
    fn parse_template_single_word() {
        let (target, body) = parse_template_and_target("copilot", None);
        assert_eq!(target, "copilot");
        assert_eq!(body, "");
    }

    #[test]
    fn parse_template_strips_leading_whitespace_after_target() {
        let (target, body) = parse_template_and_target("copilot   --foo", None);
        assert_eq!(target, "copilot");
        assert_eq!(body, "--foo");
    }
}
