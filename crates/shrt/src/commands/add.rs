use crate::cli::{self, AddArgs, Ctx};
use crate::config::SidecarConfig;
use crate::paths;
use crate::shim;

pub fn run(ctx: &Ctx, args: &AddArgs) -> anyhow::Result<i32> {
    if let Err(e) = cli::validate_name(&args.name) {
        eprintln!("shrt: {}", e);
        return Ok(64);
    }

    let expected_shim = ctx.shim_dir.join(format!("{}.exe", args.name));
    if let Ok(found) = which::which(&args.name) {
        if paths::normalize_for_compare(&found)
            != paths::normalize_for_compare(&expected_shim)
        {
            eprintln!(
                "shrt: error: shim name '{}' is shadowed by an existing binary at {}.\n\
                 PATH lookup of '{}' resolves to that path before reaching {};\n\
                 your shim would never be invoked. Pick a different shim name, or\n\
                 remove the shadowing binary first.\n\
                 \n\
                 If '{}' is a Windows App Execution Alias (e.g. wt, python, winget),\n\
                 disable it via Settings -> Apps -> Advanced app settings -> App\n\
                 execution aliases.",
                args.name,
                found.display(),
                args.name,
                expected_shim.display(),
                args.name,
            );
            return Ok(64);
        }
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
