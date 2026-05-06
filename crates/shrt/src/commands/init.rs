use crate::cli::Ctx;
use crate::shim;

pub fn run(ctx: &Ctx) -> anyhow::Result<i32> {
    let report = shim::init(ctx)?;

    if ctx.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(0);
    }

    if ctx.quiet {
        return Ok(0);
    }

    println!("shim dir: {}", report.shim_dir.display());
    println!("created: {}", report.created);
    println!("on PATH: {}", report.on_path);

    let dir = report.shim_dir.display();

    if let Some(err) = &report.path_error {
        eprintln!();
        eprintln!("warning: could not auto-add {} to your user PATH: {}", dir, err);
        eprintln!("Add it manually:");
        eprintln!("  PowerShell: $env:PATH = \"$env:PATH;{}\"", dir);
        eprintln!("  cmd.exe:    set PATH=%PATH%;{}", dir);
        eprintln!("  Git Bash:   export PATH=\"$PATH:{}\"", dir);
    } else if report.path_added {
        println!();
        println!(
            "Added {} to your user PATH. Open a new shell for it to take effect.",
            dir
        );
    } else if report.path_already_present && !report.on_path {
        println!();
        println!(
            "{} is on your user PATH but not yet visible in this shell. Open a new shell to pick it up.",
            dir
        );
    }

    Ok(0)
}
