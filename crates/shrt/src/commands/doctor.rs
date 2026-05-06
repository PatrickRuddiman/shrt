use crate::cli::Ctx;
use crate::shim::{self, Status};

pub fn run(ctx: &Ctx) -> anyhow::Result<i32> {
    let report = shim::doctor(ctx)?;

    if ctx.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else if !ctx.quiet {
        for check in &report.checks {
            let tag = match check.status {
                Status::Ok => "[OK]  ",
                Status::Warn => "[WARN]",
                Status::Fail => "[FAIL]",
            };
            println!("{} {}: {}", tag, check.name, check.message);
        }
    }

    let exit = match report.summary {
        Status::Fail => 1,
        Status::Warn | Status::Ok => 0,
    };
    Ok(exit)
}
