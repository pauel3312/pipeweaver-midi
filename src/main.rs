mod behaviours;
mod pipeweaver_main;
mod pwv_controllers;

mod midi_callbacks;
mod ui;
mod widgets;
mod config;

use crate::pipeweaver_main::SharedState;
use clap::Parser;
use std::io::Result;
use std::sync::{Arc, Mutex};
use crate::config::Args;

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
