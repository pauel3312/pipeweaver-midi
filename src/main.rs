use std::{env, fs};
use interprocess::local_socket::{GenericFilePath, ToFsName};
use pipeweaver_ipc::client::Client;
use pipeweaver_ipc::clients::ipc::ipc_socket::Socket;
use pipeweaver_ipc::commands::{APICommand, DaemonRequest, DaemonResponse};
use interprocess::local_socket::tokio::prelude::LocalSocketStream;
use directories::BaseDirs;
use anyhow::{Error, Result};
use std::path::PathBuf;
use std::time::Duration;
use interprocess::local_socket::traits::tokio::Stream;
use pipeweaver_ipc::clients::ipc::ipc_client::IPCClient;
use pipeweaver_shared::Mix;

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
    println!("Hello, world!");
    let path = get_socket_path()?;
    println!("Using IPC Path: {:?}", path);

    let connection = LocalSocketStream::connect(path.to_fs_name::<GenericFilePath>()?).await?;
    let socket: Socket<DaemonResponse, DaemonRequest> = Socket::new(connection);
    let mut client = IPCClient::new(socket);
    let status = client.get_status().await?;
    let mut i = 0u8;
    let mut up = true;

    loop{
        tokio::time::sleep(Duration::from_millis(10)).await;
        client.send(
            &DaemonRequest::Pipewire(
                APICommand::SetSourceVolume(status.audio.profile.devices.sources.physical_devices[0].description.id,Mix::B, i)
            )
        ).await?;
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

}
