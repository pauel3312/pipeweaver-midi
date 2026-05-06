use crate::behaviours::ToggleBtn;
use crate::midi_pattern::{is_event_valid, MidiMsgCallbackTree};
use crate::pwv_controllers::{bool_controller, AxisCommand, AxisProvider, BoolCommand, BooleanProvider};
use midi_msg::{Channel, ChannelVoiceMsg, MidiMsg};
use midir::{MidiInput, MidiInputPort};
use pipeweaver_ipc::commands::{DaemonRequest, DaemonStatus};
use pipeweaver_shared::MuteTarget;
use pipeweaver_websocket_client::{spawn_pipeweaver_handler, BroadcastMessage};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::signal;
use tokio::sync::mpsc::Sender;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;
use ulid::Ulid;

#[derive(Clone)]
pub struct SharedState {
    pub status: Option<DaemonStatus>,
    pub tx: Option<Arc<Mutex<Sender<DaemonRequest>>>>,
    pub current_port: MidiInputPort,
    pub last_midi_event: Option<MidiMsg>,
    pub midi_tree: MidiMsgCallbackTree,
    pub axes: HashMap<AxisCommand, Arc<Mutex<dyn AxisProvider + Send + Sync>>>,
    pub buttons: HashMap<BoolCommand, Arc<Mutex<dyn BooleanProvider + Send + Sync>>>,
    pub learn_msg: Option<MidiMsg>,
    pub learn_mode: bool,
    pub learning: bool,
}

impl Debug for SharedState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedState").finish()
    }
}


impl SharedState {
    pub fn new(port: Option<MidiInputPort>) -> Self {
        let midi = MidiInput::new("temp").unwrap();
        Self {
            status: None,
            tx: None,
            current_port: port.unwrap_or(midi.ports()[0].clone()),
            last_midi_event: None,
            midi_tree: MidiMsgCallbackTree::new(),
            axes: HashMap::new(),
            buttons: HashMap::new(),
            learn_msg: None,
            learn_mode: true,
            learning: false,
        }
    }
}

pub async fn main(state: Arc<Mutex<SharedState>>) {
    // Create a channel for broadcasting changes
    let (broadcast, _) = broadcast::channel(10);

    // Create a subscription for the broadcast channel
    let mut subscription = broadcast.subscribe();

    // Create a channel for sending commands
    let (tx, rx) = mpsc::channel(10);

    // Spawn up the Pipeweaver handler, which will return a way to stop it.
    let stopper = spawn_pipeweaver_handler(rx, broadcast.clone()).await;

    // let mut can_send: bool = false;


    let tx = Arc::new(Mutex::new(tx));
    // let cmd = AxisCommand::TargetVolume {id: Ulid::new()};
    // let controller = axis_controller(cmd,
    // Arc::new(Mutex::new(AbsoluteAxis::new(0, 127, 0, 100))),
    // tx.clone());

    state.lock().unwrap().tx = Some(tx.clone());

    // let cmd = BoolCommand::SourceMute {
    //     id: Ulid::new(),
    //     target: MuteTarget::TargetA
    // };
    // let controller = bool_controller(cmd,
    // Arc::new(Mutex::new(ToggleBtn::new(64, None))),
    // tx.clone());
    // let controller = Arc::new(Mutex::new(controller));
    //
    // state.lock().unwrap().midi_tree.insert_callback(&MidiMsg::ChannelVoice {
    //     channel: Channel::Ch1,
    //     msg: ChannelVoiceMsg::NoteOff {
    //         note: 12,
    //         velocity: 0,
    //     }
    // }, controller.clone()).unwrap();
    //
    // state.lock().unwrap().buttons.insert(cmd, controller.clone());




    tokio::spawn(learn_thread(state.clone()));

    loop {
        tokio::select! {
            Ok(message) = subscription.recv() => {
                match message {
                    BroadcastMessage::Online => {
                        println!("Connected to Pipeweaver");
                        // can_send = true;
                    }
                    BroadcastMessage::Offline => {
                        println!("Connection to Pipeweaver lost, reconnecting in 5 seconds...");
                        // can_send = false;
                        state.lock().unwrap().status = None
                    }
                    BroadcastMessage::Status(new_status) => {
                        // Is this the first time we've seen the status since connecting?
                        if state.lock().unwrap().status.is_none() {
                        }

                        // Send received data back to the axis behaviours.
                        for (id, pvd) in &mut state.lock().unwrap().axes {
                            let val = id.get_value(new_status.as_ref());
                            match val {
                                Some(d) => { pvd.lock().unwrap().set(d); }
                                None => {}
                            }
                        }

                        // Send received data back to the bool behaviours
                        for (id, pvd) in &mut state.lock().unwrap().buttons {
                            let val = id.get_value(new_status.as_ref());
                            match val {
                                Some(d) => { pvd.lock().unwrap().set(d); }
                                None => {}
                            }
                        }
                        state.lock().unwrap().status = Some(*new_status);
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

async fn learn_thread(state: Arc<Mutex<SharedState>>) -> JoinHandle<()> {
    println!("starting learn thread");
    loop {
        if state.lock().unwrap().learning {
            state.lock().unwrap().learn_msg = Some(wait_on_midi_event_changed(state.clone()).await);
            state.lock().unwrap().learning = false;
            // println!("{:?}", state.lock().unwrap().learn_msg);
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn wait_on_midi_event_changed(state: Arc<Mutex<SharedState>>) -> MidiMsg {
    let start_msg = state.lock().unwrap().last_midi_event.clone();
    loop {
        let event = state.lock().unwrap().last_midi_event.clone();
        // println!("{:?}", event);
        if start_msg != event {break;}
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    state.lock().unwrap().last_midi_event.clone().unwrap()
}
