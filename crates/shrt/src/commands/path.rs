use crate::cli::Ctx;
use crate::shim;

pub fn run(ctx: &Ctx) -> anyhow::Result<i32> {
    let report = shim::path_report(ctx);

    if ctx.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", report.path.display());
    }

    Ok(0)
}
