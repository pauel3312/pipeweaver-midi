mod pipeweaver_main;

pub mod behaviours;
mod common;
mod config;
pub mod midi;
mod pipeweaver_controllers;
mod tray;
mod ui;

use crate::common::SharedState;
use crate::config::Args;
use crate::tray::{spawn_tray, TrayState};
use anyhow::Result;
use clap::Parser;
use midi::manager::MidiMgr;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

const APP_NAME: &str = "Pipeweaver-MIDI";

const ICON: &[u8] = include_bytes!("../../pipeweaver/daemon/resources/icons/pipeweaver-large.png");



#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let state = Arc::new(Mutex::new(SharedState::new(None, args.config)));

    let midi_mgr = MidiMgr::new();
    let midi_mgr = Arc::new(Mutex::new(midi_mgr));

    let stopped = Arc::new(AtomicBool::new(false));
    let tray_state = Arc::new(Mutex::new(TrayState {
        ui_on: Arc::new(AtomicBool::new(!args.quiet)),
        ctx: None,
    }));

    let pwv_join_handle = tokio::spawn(pipeweaver_main::main(state.clone(), midi_mgr.clone(), stopped.clone()));

    let tray_join_handle = tokio::spawn(spawn_tray(stopped.clone(), tray_state.clone()));


    ui::main::run(state.clone(), midi_mgr.clone(), stopped, tray_state).await?; // NEEDS TO RUN ON MAIN THREAD (bc egui ig)

    let (pwv_result, tray_result) = tokio::join!(pwv_join_handle, tray_join_handle);
    pwv_result??;
    tray_result?
}
