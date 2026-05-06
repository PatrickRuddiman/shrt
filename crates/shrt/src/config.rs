use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SidecarConfig {
    pub target: String,
    pub template: String,
    #[serde(default)]
    pub shell: bool,
    #[serde(default)]
    pub cwd: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub created: Option<String>,
    #[serde(default = "default_version")]
    pub version: u32,
}

fn default_version() -> u32 {
    1
}

impl Default for SidecarConfig {
    fn default() -> Self {
        Self {
            target: String::new(),
            template: String::new(),
            shell: false,
            cwd: String::new(),
            description: String::new(),
            created: None,
            version: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub name: String,
    #[serde(flatten)]
    pub config: SidecarConfig,
}

pub fn read_sidecar(path: &Path) -> Result<SidecarConfig> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("reading sidecar {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("parsing sidecar {}", path.display()))
}

pub fn write_sidecar(path: &Path, cfg: &SidecarConfig) -> Result<()> {
    let body = serialize_sidecar(cfg)?;

    let mut tmp_name = path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("invalid sidecar path: {}", path.display()))?
        .to_os_string();
    tmp_name.push(".tmp");
    let tmp = path.with_file_name(&tmp_name);

    fs::write(&tmp, body.as_bytes())
        .with_context(|| format!("writing temp sidecar {}", tmp.display()))?;

    if let Err(e) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(e).with_context(|| {
            format!("renaming {} to {}", tmp.display(), path.display())
        });
    }
    Ok(())
}

fn serialize_sidecar(cfg: &SidecarConfig) -> Result<String> {
    let mut out = String::new();
    out.push_str("target = ");
    out.push_str(&escape_basic(&cfg.target)?);
    out.push('\n');

    out.push_str("template = ");
    out.push_str(&escape_basic(&cfg.template)?);
    out.push('\n');

    if cfg.shell {
        out.push_str("shell = true\n");
    }
    if !cfg.cwd.is_empty() {
        out.push_str("cwd = ");
        out.push_str(&escape_basic(&cfg.cwd)?);
        out.push('\n');
    }
    if !cfg.description.is_empty() {
        out.push_str("description = ");
        out.push_str(&escape_basic(&cfg.description)?);
        out.push('\n');
    }
    if let Some(created) = &cfg.created {
        out.push_str("created = ");
        out.push_str(&escape_basic(created)?);
        out.push('\n');
    }

    out.push_str("version = ");
    out.push_str(&cfg.version.to_string());
    out.push('\n');

    Ok(out)
}

fn escape_basic(s: &str) -> Result<String> {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => anyhow::bail!("carriage return ('\\r') is not allowed in sidecar strings"),
            c if (c as u32) < 0x20 => {
                anyhow::bail!(
                    "control character U+{:04X} is not allowed in sidecar strings",
                    c as u32
                );
            }
            c => out.push(c),
        }
    }
    out.push('"');
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_basic_string_form() {
        let cfg = SidecarConfig {
            target: "copilot".into(),
            template: "hello".into(),
            ..Default::default()
        };
        let out = serialize_sidecar(&cfg).unwrap();
        assert!(out.contains("target = \"copilot\""));
        assert!(out.contains("template = \"hello\""));
        assert!(out.contains("version = 1"));
    }

    #[test]
    fn omits_default_optional_fields() {
        let cfg = SidecarConfig {
            target: "x".into(),
            template: "y".into(),
            ..Default::default()
        };
        let out = serialize_sidecar(&cfg).unwrap();
        assert!(!out.contains("shell"));
        assert!(!out.contains("cwd"));
        assert!(!out.contains("description"));
        assert!(!out.contains("created"));
    }

    #[test]
    fn emits_shell_when_true() {
        let cfg = SidecarConfig {
            target: "x".into(),
            template: "y".into(),
            shell: true,
            ..Default::default()
        };
        let out = serialize_sidecar(&cfg).unwrap();
        assert!(out.contains("shell = true"));
    }

    #[test]
    fn emits_optional_fields_when_set() {
        let cfg = SidecarConfig {
            target: "x".into(),
            template: "y".into(),
            cwd: "~".into(),
            description: "test".into(),
            created: Some("2026-05-06T10:00:00Z".into()),
            ..Default::default()
        };
        let out = serialize_sidecar(&cfg).unwrap();
        assert!(out.contains("cwd = \"~\""));
        assert!(out.contains("description = \"test\""));
        assert!(out.contains("created = \"2026-05-06T10:00:00Z\""));
    }

    #[test]
    fn escape_basic_quote_and_backslash() {
        assert_eq!(escape_basic("\"").unwrap(), "\"\\\"\"");
        assert_eq!(escape_basic("\\").unwrap(), "\"\\\\\"");
    }

    #[test]
    fn escape_basic_newline_and_tab() {
        assert_eq!(escape_basic("\n").unwrap(), "\"\\n\"");
        assert_eq!(escape_basic("\t").unwrap(), "\"\\t\"");
    }

    #[test]
    fn escape_basic_rejects_carriage_return() {
        assert!(escape_basic("a\rb").is_err());
    }

    #[test]
    fn escape_basic_rejects_control_chars() {
        assert!(escape_basic("\u{0001}").is_err());
        assert!(escape_basic("\u{0007}").is_err());
        assert!(escape_basic("\u{0008}").is_err());
        assert!(escape_basic("\u{001F}").is_err());
    }

    #[test]
    fn escape_basic_unicode_passthrough() {
        assert_eq!(escape_basic("café").unwrap(), "\"café\"");
        assert_eq!(escape_basic("🎉").unwrap(), "\"🎉\"");
    }

    #[test]
    fn round_trip_full_config() {
        let cfg = SidecarConfig {
            target: "copilot".into(),
            template: "-p \"hello {1}\" --yolo".into(),
            shell: false,
            cwd: "~/projects".into(),
            description: "create worktree".into(),
            created: Some("2026-05-06T10:00:00Z".into()),
            version: 1,
        };
        let body = serialize_sidecar(&cfg).unwrap();
        let parsed: SidecarConfig = toml::from_str(&body).unwrap();
        assert_eq!(parsed, cfg);
    }

    #[test]
    fn round_trip_minimal_config() {
        let cfg = SidecarConfig {
            target: "x".into(),
            template: "y".into(),
            ..Default::default()
        };
        let body = serialize_sidecar(&cfg).unwrap();
        let parsed: SidecarConfig = toml::from_str(&body).unwrap();
        assert_eq!(parsed, cfg);
    }

    #[test]
    fn round_trip_with_escape_sequences() {
        let cfg = SidecarConfig {
            target: "x".into(),
            template: "tab\there\nnewline\"quote\\backslash".into(),
            ..Default::default()
        };
        let body = serialize_sidecar(&cfg).unwrap();
        let parsed: SidecarConfig = toml::from_str(&body).unwrap();
        assert_eq!(parsed, cfg);
    }

    #[test]
    fn defaults_applied_on_minimal_read() {
        let body = "target = \"x\"\ntemplate = \"y\"\n";
        let cfg: SidecarConfig = toml::from_str(body).unwrap();
        assert!(!cfg.shell);
        assert_eq!(cfg.cwd, "");
        assert_eq!(cfg.description, "");
        assert_eq!(cfg.created, None);
        assert_eq!(cfg.version, 1);
    }

    #[test]
    fn write_sidecar_round_trip_via_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.shrt");
        let cfg = SidecarConfig {
            target: "copilot".into(),
            template: "go {1}".into(),
            shell: true,
            cwd: "/tmp".into(),
            description: "round-trip".into(),
            created: Some("2026-05-06T10:00:00Z".into()),
            version: 1,
        };
        write_sidecar(&path, &cfg).unwrap();
        let parsed = read_sidecar(&path).unwrap();
        assert_eq!(parsed, cfg);
    }

    #[test]
    fn write_sidecar_leaves_no_temp_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("atomic.shrt");
        let cfg = SidecarConfig {
            target: "x".into(),
            template: "y".into(),
            ..Default::default()
        };
        write_sidecar(&path, &cfg).unwrap();
        let temp_file = path.with_file_name("atomic.shrt.tmp");
        assert!(!temp_file.exists());
        assert!(path.exists());
    }

    #[test]
    fn write_sidecar_uses_lf_line_endings() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("eol.shrt");
        let cfg = SidecarConfig {
            target: "x".into(),
            template: "y".into(),
            ..Default::default()
        };
        write_sidecar(&path, &cfg).unwrap();
        let bytes = fs::read(&path).unwrap();
        assert!(!bytes.contains(&b'\r'));
    }

    #[test]
    fn read_sidecar_propagates_io_error() {
        let missing = Path::new("C:/definitely_not_a_real_file_xyz_42.shrt");
        assert!(read_sidecar(missing).is_err());
    }

    #[test]
    fn entry_serializes_with_flattened_config() {
        let entry = Entry {
            name: "wt".into(),
            config: SidecarConfig {
                target: "copilot".into(),
                template: "y".into(),
                ..Default::default()
            },
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"name\":\"wt\""));
        assert!(json.contains("\"target\":\"copilot\""));
        assert!(json.contains("\"template\":\"y\""));
    }
}
