mod pipeweaver_main;

pub mod behaviours;
mod common;
mod config;
pub mod midi;
mod pipeweaver_controllers;
mod ui;

use crate::common::SharedState;
use crate::config::Args;
use clap::Parser;
use midi::manager::MidiMgr;
use std::io::Result;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let state = Arc::new(Mutex::new(SharedState::new(None, args.config)));

    let midi_mgr = MidiMgr::new();
    let midi_mgr = Arc::new(Mutex::new(midi_mgr));

    let pwv_join_handle = tokio::spawn(pipeweaver_main::main(state.clone(), midi_mgr.clone()));

    if !args.quiet {
        ui::main::run(state.clone(), midi_mgr.clone()).await?;
    }

    tokio::join!(pwv_join_handle).0?;

    Ok(())
}
