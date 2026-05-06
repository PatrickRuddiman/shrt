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

    if !report.on_path {
        let dir = report.shim_dir.display();
        println!();
        println!("Add the shim directory to PATH so its shims become invocable:");
        println!("  PowerShell: $env:PATH = \"$env:PATH;{}\"", dir);
        println!("  cmd.exe:    set PATH=%PATH%;{}", dir);
        println!("  Git Bash:   export PATH=\"$PATH:{}\"", dir);
    }

    Ok(0)
}
