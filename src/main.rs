mod behaviours;
mod pipeweaver_main;
mod pwv_controllers;

mod midi_pattern;
mod ui;
mod widgets;

use crate::pipeweaver_main::SharedState;
use clap::Parser;
use std::io::Result;
use std::sync::{Arc, Mutex};

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Path to config file
    #[arg(short, long, default_value = "./test.json")]
    config: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let state = Arc::new(Mutex::new(SharedState::new(None, args.config)));

    let pwv_join_handle = tokio::spawn(pipeweaver_main::main(state.clone()));

    ui::run(state.clone()).await?;

    tokio::join!(pwv_join_handle).0?;

    Ok(())
}
