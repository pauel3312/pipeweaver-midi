mod midi_prototypes;

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

    let _ = midi_prototypes::main().await;
    Ok(())

/*
    let mut client = WebClient::new(String::from("http://localhost:14565/api/command"));
    println!("{:?}", client.get_status().await?);

    let client: Arc<Mutex<WebClient>> = Arc::new(Mutex::new(client));
    let status: Arc<Mutex<DaemonStatus>> = Arc::new(Mutex::new(client.lock().await.get_status().await?));
    let mut i = 0u8;
    let mut up = true;

    loop{
        tokio::time::sleep(Duration::from_millis(100)).await;

        let status_clone = Arc::clone(&status);
        let client_clone = Arc::clone(&client);
        tokio::spawn(async move {
            let _ = client_clone.lock().await.send(
                &DaemonRequest::Pipewire(
                    APICommand::SetSourceVolume(status_clone.lock().await.audio.profile.devices.sources.physical_devices[0].description.id, Mix::B, i)
                )
            ).await;
        });
        if up {
            if i < 100{
                i += 1;
                continue;
            }
            up = false;
            continue;
        }
        if i > 0 {
            i -= 1;
            continue;
        }
        up = true;
    }
 */
}
