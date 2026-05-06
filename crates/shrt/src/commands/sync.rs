use crate::cli::Ctx;
use crate::shim;

pub fn run(ctx: &Ctx) -> anyhow::Result<i32> {
    let report = match shim::sync(ctx) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("shrt: {}", e);
            return Ok(e.exit_code());
        }
    };

    if ctx.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        if !ctx.quiet {
            println!("updated: {} / total: {}", report.updated, report.total);
        }
        for (name, reason) in &report.errors {
            eprintln!("shrt: {}: {}", name, reason);
        }
    }

    if report.total > 0 && report.errors.len() == report.total {
        return Ok(1);
    }
    Ok(0)
}
