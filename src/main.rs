mod midi_prototypes;
mod midi_pattern;
mod pipeweaver_tests;
mod behaviours;
mod pwv_controllers;

use pipeweaver_ipc::commands::{APICommand, DaemonRequest, DaemonStatus};
use directories::BaseDirs;
use anyhow::{Error, Result};
use std::path::PathBuf;
use std::{env, fs};
use std::sync::Arc;
use pipeweaver_ipc::clients::web::web_client::WebClient;
use pipeweaver_shared::Mix;
use tokio::sync::Mutex;
use tokio::time::Duration;


const APP_NAME: &str = "PipeWeaver";
const APP_NAME_ID: &str = "pipeweaver";


pub fn get_socket_path() -> Result<PathBuf> {
    let path = BaseDirs::new()
        .and_then(|base| base.runtime_dir().map(|p| p.to_path_buf()))
        .map(Ok::<PathBuf, Error>)
        .unwrap_or_else(|| {
            let tmp_dir = env::temp_dir().join(APP_NAME);
            if !tmp_dir.exists() {
                fs::create_dir_all(&tmp_dir)?;
            }
            Ok(tmp_dir)
        })?;

    let socket_path = path.join(format!("{}.socket", APP_NAME_ID));
    Ok(socket_path)
}


#[tokio::main]
async fn main() -> Result<()> {

    // let t1 = tokio::spawn( midi_prototypes::main_wrap());
    let t2 = tokio::spawn( pipeweaver_tests::main() );

    tokio::try_join!(t2)?;
    Ok(())
}
