mod cli;
mod commands;
mod paths;
mod shim;

use clap::Parser;
use cli::{Cli, Commands, Ctx};

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let parsed = Cli::parse();

    let shim_dir = match paths::shim_dir(parsed.shim_dir.as_deref()) {
        Ok(d) => d,
        Err(e) => {
            print_error(&e, parsed.quiet);
            return 1;
        }
    };

    let ctx = Ctx {
        shim_dir,
        quiet: parsed.quiet,
        json: parsed.json,
        runner_bytes: shim::RUNNER_BYTES,
    };

    let result: anyhow::Result<i32> = match parsed.command {
        Commands::Init => commands::init::run(&ctx),
        Commands::Add(args) => commands::add::run(&ctx, &args),
        Commands::Remove(args) => commands::remove::run(&ctx, &args),
        Commands::List(args) => commands::list::run(&ctx, &args),
        Commands::Show(args) => commands::show::run(&ctx, &args),
        Commands::Sync => commands::sync::run(&ctx),
        Commands::Path => commands::path::run(&ctx),
        Commands::Doctor => commands::doctor::run(&ctx),
    };

    match result {
        Ok(code) => code,
        Err(e) => {
            print_error(&e, ctx.quiet);
            1
        }
    }
}

fn print_error(e: &anyhow::Error, quiet: bool) {
    if quiet {
        eprintln!("shrt: {}", e);
    } else {
        eprintln!("shrt: {:#}", e);
    }
}
