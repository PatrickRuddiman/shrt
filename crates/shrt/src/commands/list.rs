use crate::cli::{Ctx, ListArgs};
use crate::shim;

pub fn run(ctx: &Ctx, args: &ListArgs) -> anyhow::Result<i32> {
    let entries = match shim::list(ctx) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("shrt: {}", e);
            return Ok(e.exit_code());
        }
    };

    if ctx.json {
        println!("{}", serde_json::to_string_pretty(&entries)?);
        return Ok(0);
    }

    if entries.is_empty() {
        return Ok(0);
    }

    let max_name = entries.iter().map(|e| e.name.len()).max().unwrap_or(0);

    for entry in &entries {
        if args.verbose {
            println!(
                "{:<width$}  target:      {}",
                entry.name,
                entry.config.target,
                width = max_name
            );
            println!(
                "{:<width$}  template:    {}",
                "",
                entry.config.template,
                width = max_name
            );
            if !entry.config.cwd.is_empty() {
                println!(
                    "{:<width$}  cwd:         {}",
                    "",
                    entry.config.cwd,
                    width = max_name
                );
            }
            if !entry.config.description.is_empty() {
                println!(
                    "{:<width$}  description: {}",
                    "",
                    entry.config.description,
                    width = max_name
                );
            }
            if let Some(c) = &entry.config.created {
                println!(
                    "{:<width$}  created:     {}",
                    "",
                    c,
                    width = max_name
                );
            }
            println!();
        } else {
            println!(
                "{:<width$}  {}",
                entry.name,
                entry.config.target,
                width = max_name
            );
        }
    }

    Ok(0)
}
