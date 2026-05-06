mod behaviours;
mod pipeweaver_main;
mod pwv_controllers;

mod midi_pattern;
mod ui;
mod widgets;

use std::io::Result;
use std::sync::{Arc, Mutex};
use crate::pipeweaver_main::SharedState;
// TODO save/load

#[tokio::main]
async fn main() -> Result<()> {

    let state = Arc::new(Mutex::new(SharedState::new(None)));

    let pwv_join_handle = tokio::spawn(pipeweaver_main::main(state.clone()));

    ui::run(state.clone()).await?;

    tokio::join!(pwv_join_handle).0?;

    Ok(())
}
