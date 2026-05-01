use crate::behaviours::AbsoluteAxis;
use crate::midi_pattern::MidiMsgCallbackTree;
use crate::pwv_controllers::{CallbackProvider, source_volume_controller};
use midi_msg::ControlChange::CC;
use midi_msg::{Channel, ChannelVoiceMsg, MidiMsg};
use pipeweaver_ipc::commands::DaemonStatus;
use pipeweaver_shared::Mix;
use pipeweaver_websocket_client::{BroadcastMessage, spawn_pipeweaver_handler};
use std::sync::{Arc, Mutex};
use tokio::signal;
use tokio::sync::{broadcast, mpsc};

pub async fn main() {
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

    let mut tree = MidiMsgCallbackTree::new();

    loop {
        tokio::select! {
            Ok(message) = subscription.recv() => {
                match message {
                    BroadcastMessage::Online => {
                        println!("Connected to Pipeweaver");
                        can_send = true;
                    },
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
                            let src = &status.audio.profile.devices.sources.physical_devices[0];
                            let controller = source_volume_controller(
                                src.description.id,
                                Mix::A,
                                Arc::new(Mutex::new(
                                    AbsoluteAxis::new(0, 127, 0, 100)
                                )),
                                tx.clone());

                            tree.insert_callback(&MidiMsg::ChannelVoice {
                                channel: Channel::Ch1,
                                msg: ChannelVoiceMsg::ControlChange {
                                    control: CC {
                                        control: 60,
                                        value: 0,
                                    }
                                }
                            },
                                Arc::new(Mutex::new(move |data: u8| {controller.callback(data);}))
                            ).unwrap();
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
}
