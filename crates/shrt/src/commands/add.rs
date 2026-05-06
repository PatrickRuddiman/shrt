use crate::cli::{self, AddArgs, Ctx};
use crate::config::SidecarConfig;
use crate::shim;

pub fn run(ctx: &Ctx, args: &AddArgs) -> anyhow::Result<i32> {
    if let Err(e) = cli::validate_name(&args.name) {
        eprintln!("shrt: {}", e);
        return Ok(64);
    }

    let (target, body) =
        cli::parse_template_and_target(&args.template, args.target.as_deref());

    let cfg = SidecarConfig {
        target,
        template: body,
        shell: args.shell,
        cwd: args.cwd.clone().unwrap_or_default(),
        description: args.description.clone().unwrap_or_default(),
        created: Some(
            chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        ),
        version: 1,
    };

    if !cfg.shell && which::which(&cfg.target).is_err() {
        eprintln!(
            "shrt: warning: target '{}' not found on PATH. If it's a shell builtin \
             (e.g. echo, dir, cd, set), re-run with --shell. Otherwise pass a full \
             path via --target.",
            cfg.target
        );
    }

    match shim::add(ctx, &args.name, &cfg, args.force) {
        Ok(()) => Ok(0),
        Err(e) => {
            eprintln!("shrt: {}", e);
            Ok(e.exit_code())
        }
    }
}
