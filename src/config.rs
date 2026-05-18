use crate::behaviours::axis_behaviours::AxisBehaviour;
use crate::behaviours::button_behaviours::BooleanBehaviour;
use crate::pipeweaver_controllers::commands::{AxisCommand, BoolCommand};
use clap::Parser;
use midi_msg::MidiMsg;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::{fs, thread};

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub(crate) struct Args {
    /// Path to config file
    #[arg(
        short,
        long,
        default_value_os_t = default_config_path()
    )]
    pub(crate) config: PathBuf,

    /// Disables the config UI.
    #[arg(short, long, default_value_t = false)]
    pub(crate) quiet: bool,
}

pub(crate) fn default_config_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));

    base.join("pipeweaver/pipeweaver-midi.json")
}

#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct ConfigState {
    #[serde_as(as = "Vec<(_, _)>")]
    pub(crate) axes: HashMap<AxisCommand, (AxisBehaviour, MidiMsg)>,
    #[serde_as(as = "Vec<(_, _)>")]
    pub(crate) buttons: HashMap<BoolCommand, (BooleanBehaviour, MidiMsg)>,
    pub(crate) midi_device: String,

    pub(crate) auto_save: bool,

    #[serde(skip)]
    pub(crate) path: Arc<Mutex<PathBuf>>,

    #[serde(skip)]
    pub(super) file_dialog_tx: Option<Sender<Option<PathBuf>>>,
}

impl From<PathBuf> for ConfigState {
    fn from(path: PathBuf) -> Self {
        let (file_dialog_tx, file_dialog_rx) = channel::<Option<PathBuf>>();

        let mut config = match fs::read_to_string(path.clone()) {
            Ok(data) => serde_json::from_str::<ConfigState>(&data).unwrap_or_else(|_| ConfigState::new()),
            Err(_) => ConfigState::new(),
        };
        let path = Arc::new(Mutex::new(path));
        config.path = path;
        config.launch_rx_thread(file_dialog_rx);
        config.file_dialog_tx = Some(file_dialog_tx);
        config
    }
}

impl ConfigState {
    pub(crate) fn new() -> Self {
        let path = Arc::new(Mutex::new(default_config_path()));

        let (file_dialog_tx, file_dialog_rx) = channel::<Option<PathBuf>>();

        let conf = Self {
            axes: HashMap::new(),
            buttons: HashMap::new(),
            midi_device: "".to_string(),
            auto_save: false,
            path,
            file_dialog_tx: Some(file_dialog_tx),
        };
        conf.launch_rx_thread(file_dialog_rx);
        conf
    }

    fn launch_rx_thread(&self, file_dialog_rx: Receiver<Option<PathBuf>>) {
        let path = self.path.clone();
        thread::spawn(move || {
            while let Ok(Some(new_path)) = file_dialog_rx.recv() {
                let mut p = path.lock().unwrap();
                *p = new_path;
            }
        });
    }

    pub(crate) fn save(&self) {
        let json_string = serde_json::to_string(self).unwrap();
        fs::write(self.path.lock().unwrap().clone(), json_string).unwrap();
    }

    pub(crate) fn load(&mut self) {
        let loaded = match fs::read_to_string(self.path.lock().unwrap().clone()) {
            Ok(data) => serde_json::from_str::<ConfigState>(&data).unwrap_or_else(|_| ConfigState::new()),
            Err(_) => ConfigState::new(),
        };
        self.axes = loaded.axes;
        self.buttons = loaded.buttons;
        self.midi_device = loaded.midi_device;
        self.auto_save = loaded.auto_save;
    }

    pub(crate) fn insert_axis(&mut self, cmd: AxisCommand, behaviour: AxisBehaviour, msg: MidiMsg) {
        self.axes.insert(cmd, (behaviour, msg));
        self.autosave()
    }

    pub(crate) fn rm_axis(&mut self, cmd: AxisCommand) {
        self.axes.remove(&cmd);
        self.autosave()
    }

    pub(crate) fn insert_btn(&mut self, cmd: BoolCommand, behaviour: BooleanBehaviour, msg: MidiMsg) {
        self.buttons.insert(cmd, (behaviour, msg));
        self.autosave()
    }

    pub(crate) fn rm_btn(&mut self, cmd: BoolCommand) {
        self.buttons.remove(&cmd);
        self.autosave()
    }

    pub(crate) fn set_midi_device(&mut self, device: String) {
        self.midi_device = device;
        self.autosave()
    }

    fn autosave(&self) {
        if self.auto_save {
            self.save()
        }
    }
}
