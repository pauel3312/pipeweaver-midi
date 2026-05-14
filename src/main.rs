mod behaviours;
mod pipeweaver_main;
mod pwv_controllers;

mod midi_callbacks;
mod ui;
mod widgets;
mod config;
mod midi_mgr;

use crate::pipeweaver_main::SharedState;
use clap::Parser;
use std::io::Result;
use std::sync::{Arc, Mutex};
use crate::config::Args;
use crate::midi_mgr::MidiMgr;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let state = Arc::new(Mutex::new(SharedState::new(None, args.config)));
    
    let midi_mgr = MidiMgr::new();
    let midi_mgr = Arc::new(Mutex::new(midi_mgr));

    let pwv_join_handle = tokio::spawn(pipeweaver_main::main(state.clone(), midi_mgr.clone()));

    if !args.quiet {
        ui::run(state.clone(), midi_mgr.clone()).await?;
    }

    tokio::join!(pwv_join_handle).0?;

    Ok(())
}
