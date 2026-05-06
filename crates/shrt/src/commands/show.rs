use crate::cli::{Ctx, ShowArgs};
use crate::config::SidecarConfig;
use crate::shim;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize)]
struct ShowJson<'a> {
    path: &'a PathBuf,
    config: &'a SidecarConfig,
}

pub fn run(ctx: &Ctx, args: &ShowArgs) -> anyhow::Result<i32> {
    match shim::show(ctx, &args.name) {
        Ok((path, raw, entry)) => {
            if ctx.json {
                let out = ShowJson {
                    path: &path,
                    config: &entry.config,
                };
                println!("{}", serde_json::to_string_pretty(&out)?);
            } else {
                print!("{}", raw);
            }
            Ok(0)
        }
        Err(e) => {
            eprintln!("shrt: {}", e);
            Ok(e.exit_code())
        }
    }
}
