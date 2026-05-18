use crate::config::ConfigState;
use crate::midi::callback_tree::MidiMsgCallbackTree;
use crate::pipeweaver_controllers::axis::axis_controller;
use crate::pipeweaver_controllers::bool::bool_controller;
use crate::pipeweaver_controllers::commands::{AxisCommand, BoolCommand};
use crate::pipeweaver_controllers::core::{AxisProvider, BooleanProvider};
use midi_msg::MidiMsg;
use midir::{MidiInput, MidiInputPort};
use pipeweaver_ipc::commands::{DaemonRequest, DaemonStatus};
use std::collections::HashMap;
use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Sender;

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
            config: ConfigState::from(path),
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
                    self.midi_tree.insert_callback(&msg, controller.clone()).unwrap();
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
                    self.midi_tree.insert_callback(&msg, controller.clone()).unwrap();
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
