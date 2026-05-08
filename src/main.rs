mod behaviours;
mod pipeweaver_main;
mod pwv_controllers;

mod midi_pattern;
mod ui;
mod widgets;

use crate::pipeweaver_main::SharedState;
use clap::Parser;
use std::io::Result;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Path to config file
    #[arg(
        short,
        long,
        default_value_os_t = default_config_path()
    )]
    config: PathBuf,

    /// Disables the config UI.
    #[arg(short, long, default_value_t = false)]
    quiet: bool,
}

fn default_config_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|home| PathBuf::from(home).join(".config"))
        })
        .unwrap_or_else(|| PathBuf::from("."));

    base.join("pipeweaver/pipeweaver-midi.json")
}
#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let state = Arc::new(Mutex::new(SharedState::new(None, args.config)));

    let pwv_join_handle = tokio::spawn(pipeweaver_main::main(state.clone()));

    if !args.quiet {
        ui::run(state.clone()).await?;
    }

    tokio::join!(pwv_join_handle).0?;

    Ok(())
}
