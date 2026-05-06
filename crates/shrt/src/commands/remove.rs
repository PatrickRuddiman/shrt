use crate::cli::{self, Ctx, RemoveArgs};
use crate::shim;

pub fn run(ctx: &Ctx, args: &RemoveArgs) -> anyhow::Result<i32> {
    if let Err(e) = cli::validate_name(&args.name) {
        eprintln!("shrt: {}", e);
        return Ok(64);
    }

    match shim::remove(ctx, &args.name) {
        Ok(()) => Ok(0),
        Err(e) => {
            eprintln!("shrt: {}", e);
            Ok(e.exit_code())
        }
    }
}
