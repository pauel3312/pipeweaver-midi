mod behaviours;
mod pipeweaver_tests;
mod pwv_controllers;

mod midi_pattern;
mod midi_prototypes;

use crate::pwv_controllers::{AxisCommand, AxisProvider, BoolCommand, BooleanProvider};
use crate::midi_pattern::MidiMsgCallbackTree;
use anyhow::Result;
use pipeweaver_ipc::commands::DaemonStatus;
use pipeweaver_websocket_client::{BroadcastMessage, spawn_pipeweaver_handler};
use std::collections::HashMap;
use std::os::fd::AsRawFd;
use std::sync::{Arc, Mutex};
use midir::{MidiInput, MidiInputConnection};
use tokio::signal;
use tokio::sync::{broadcast, mpsc};

#[tokio::main]
async fn main() -> Result<()> {
    // let t1 = tokio::spawn( midi_prototypes::main_wrap());
    // let t2 = tokio::spawn( pipeweaver_tests::main() );

    // tokio::try_join!(t2)?;

    // Create a channel for broadcasting changes
    let (broadcast, _) = broadcast::channel(10);

    // Create a subscription for the broadcast channel
    let mut subscription = broadcast.subscribe();

    // Create a channel for sending commands
    let (tx, rx) = mpsc::channel(10);

    // Spawn up the Pipeweaver handler, which will return a way to stop it.
    let stopper = spawn_pipeweaver_handler(rx, broadcast.clone()).await;

    let mut daemon_status: Option<DaemonStatus> = None;
    let mut can_send: bool = false;
    let tx = Arc::new(Mutex::new(tx));

    // Tree that will store Midi message -> pipeweaver callback
    let mut tree = MidiMsgCallbackTree::new();

    let midi = MidiInput::new("midi-discover")?;
    let ports = midi.ports();
    let mut connection: Option<MidiInputConnection<()>> = None;


    // Maps of pipeweaver commands to their behaviours for the pipeweaver-side setters.
    let mut axes: HashMap<AxisCommand, Arc<Mutex<dyn AxisProvider + Send + Sync>>> = HashMap::new();
    let mut buttons: HashMap<BoolCommand, Arc<Mutex<dyn BooleanProvider + Send + Sync>>> =
        HashMap::new();


    loop {
        tokio::select! {
            Ok(message) = subscription.recv() => {
                match message {
                    BroadcastMessage::Online => {
                        println!("Connected to Pipeweaver");
                        can_send = true;
                    }
                    BroadcastMessage::Offline => {
                        println!("Connection to Pipeweaver lost, reconnecting in 5 seconds...");
                        can_send = false;
                        daemon_status = None;
                    }
                    BroadcastMessage::Status(status) => {
                        // Is this the first time we've seen the status since connecting?
                        if daemon_status.is_none() {
                            println!("Devices: {:?}", status.audio.profile.devices);
                            println!("Apps: {:?}", status.audio.applications);
                        }

                        // Send received data back to the axis behaviours.
                        for (id, pvd) in &mut axes {
                            let val = id.get_value(status.as_ref());
                            match val {
                                Some(d) => { pvd.lock().unwrap().set(d); }
                                None => {}
                            }
                        }

                        // Send received data back to the bool behaviours
                        for (id, pvd) in &mut buttons {
                            let val = id.get_value(status.as_ref());
                            match val {
                                Some(d) => { pvd.lock().unwrap().set(d); }
                                None => {}
                            }
                        }
                        daemon_status = Some(*status);
                    }
                }
            }

            _ = signal::ctrl_c() => {
                println!("Stopping Pipeweaver Manager");
                stopper.trigger();
                break;
            }

        }
    }



    Ok(())
}
