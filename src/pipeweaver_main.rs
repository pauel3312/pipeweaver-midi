use crate::midi_callbacks::MidiMsgCallbackTree;
use crate::pwv_controllers::{
    AxisCommand, AxisProvider, BoolCommand, BooleanProvider, axis_controller, bool_controller,
};
use midi_msg::MidiMsg;
use midir::{MidiInput, MidiInputPort};
use pipeweaver_ipc::commands::{DaemonRequest, DaemonStatus};
use pipeweaver_websocket_client::{BroadcastMessage, spawn_pipeweaver_handler};
use std::collections::HashMap;
use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::signal;
use tokio::sync::mpsc::Sender;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;

use crate::config::ConfigState;
use crate::midi_mgr::MidiMgr;

#[derive(Clone)]
pub struct SharedState {
    pub(crate) config: ConfigState,
    pub(crate) status: Option<DaemonStatus>,
    pub(crate) tx: Option<Sender<DaemonRequest>>,
    pub(crate) current_port: MidiInputPort,
    pub(crate) last_midi_event: Option<MidiMsg>,
    pub(crate) midi_tree: MidiMsgCallbackTree,
    pub(crate) axes: HashMap<AxisCommand, Arc<Mutex<dyn AxisProvider + Send + Sync>>>,
    pub(crate) buttons: HashMap<BoolCommand, Arc<Mutex<dyn BooleanProvider + Send + Sync>>>,
    pub(crate) learn_msg: Option<MidiMsg>,
    pub(crate) learn_mode: bool,
    pub(crate) learning: bool,
}

impl Debug for SharedState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedState").finish()
    }
}

impl SharedState {
    pub fn new(port: Option<MidiInputPort>, path: PathBuf) -> Self {
        let midi = MidiInput::new("temp").unwrap();
        Self {
            config: ConfigState::load(path),
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

    pub fn initialize(&mut self) {
        let tx = self.tx.clone();
        match tx {
            Some(tx) => {
                for (cmd, (behaviour, midi_data)) in self.config.axes.clone() {
                    let behaviour = Arc::new(Mutex::new(behaviour));
                    let controller = axis_controller(cmd, behaviour, tx.clone());
                    match &self.status {
                        None => {}
                        Some(status) => match cmd.get_value(status) {
                            None => {}
                            Some(d) => controller.set(d),
                        },
                    }
                    let controller = Arc::new(Mutex::new(controller));

                    let msg = midi_data;
                    self.midi_tree
                        .insert_callback(&msg, controller.clone())
                        .unwrap();
                    self.axes.insert(cmd, controller);
                }
                for (cmd, (behaviour, midi_data)) in self.config.buttons.clone() {
                    let behaviour = Arc::new(Mutex::new(behaviour));
                    let controller = bool_controller(cmd, behaviour, tx.clone());
                    match &self.status {
                        None => {}
                        Some(status) => match cmd.get_value(status) {
                            None => {}
                            Some(d) => controller.set(d),
                        },
                    }
                    let controller = Arc::new(Mutex::new(controller));

                    let msg = midi_data;
                    self.midi_tree
                        .insert_callback(&msg, controller.clone())
                        .unwrap();
                    self.buttons.insert(cmd, controller);
                }
            }
            None => {}
        }
        let midi = MidiInput::new("temp").unwrap();
        for port in midi.ports() {
            if midi.port_name(&port).unwrap() == self.config.midi_device {
                self.current_port = port;
                break;
            }
        }
    }
}

pub async fn main(state: Arc<Mutex<SharedState>>, midi_mgr: Arc<Mutex<MidiMgr>>) {
    // Create a channel for broadcasting changes
    let (broadcast, _) = broadcast::channel(10);

    // Create a subscription for the broadcast channel
    let mut subscription = broadcast.subscribe();

    // Create a channel for sending commands
    let (tx, rx) = mpsc::channel(10);

    // Spawn up the Pipeweaver handler, which will return a way to stop it.
    let stopper = spawn_pipeweaver_handler(rx, broadcast.clone()).await;

    // let mut can_send: bool = false;

    {
        let mut state = state.lock().unwrap();
        state.tx = Some(tx.clone());
        state.initialize();
    }

    midi_mgr.lock().unwrap().connect(state.clone()).unwrap();

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
    loop {
        if state.lock().unwrap().learning {
            state.lock().unwrap().learn_msg = Some(wait_on_midi_event_changed(state.clone()).await);
            state.lock().unwrap().learning = false;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn wait_on_midi_event_changed(state: Arc<Mutex<SharedState>>) -> MidiMsg {
    let start_msg = state.lock().unwrap().last_midi_event.clone();
    loop {
        let event = state.lock().unwrap().last_midi_event.clone();
        if start_msg != event {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    state.lock().unwrap().last_midi_event.clone().unwrap()
}
