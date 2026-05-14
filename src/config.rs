use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use clap::Parser;
use midi_msg::MidiMsg;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use crate::behaviours::{AxisBehaviour, BooleanBehaviour};
use crate::pwv_controllers::{AxisCommand, BoolCommand};

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
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|home| PathBuf::from(home).join(".config"))
        })
        .unwrap_or_else(|| PathBuf::from("."));

    base.join("pipeweaver/pipeweaver-midi.json")
}

#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct ConfigState {
    // Vec<u8>'s are MIDI messages converted back to bytes, because MidiMsg isn't Serializable.
    #[serde_as(as = "Vec<(_, _)>")]
    pub(crate) axes: HashMap<AxisCommand, (AxisBehaviour, Vec<u8>)>,
    #[serde_as(as = "Vec<(_, _)>")]
    pub(crate) buttons: HashMap<BoolCommand, (BooleanBehaviour, Vec<u8>)>,
    pub(crate) midi_device: String,

    #[serde(skip)]
    pub path: PathBuf,
}


impl ConfigState {
    pub(crate) fn new() -> Self {
        Self {
            axes: HashMap::new(),
            buttons: HashMap::new(),
            midi_device: "".to_string(),
            path: default_config_path(),
        }
    }

    pub(crate) fn save(&self) {
        let json_string = serde_json::to_string_pretty(self).unwrap();
        fs::write(self.path.clone(), json_string).unwrap();
    }

    pub(crate) fn load(path: PathBuf) -> Self {
        let mut config = match fs::read_to_string(path.clone()) {
            Ok(data) => {
                serde_json::from_str::<ConfigState>(&data).unwrap_or_else(|_| ConfigState::new())
            }
            Err(_) => ConfigState::new(),
        };
        config.path = path;
        config
    }
    
    pub(crate) fn insert_axis(&mut self, cmd: AxisCommand, behaviour: AxisBehaviour, msg: MidiMsg) {
        self.axes.insert(cmd, (behaviour, msg.to_midi().into()));
        self.save()
    }

    pub(crate) fn rm_axis(&mut self, cmd: AxisCommand) {
        self.axes.remove(&cmd);
        self.save()
    }

    pub(crate) fn insert_btn(&mut self, cmd: BoolCommand, behaviour: BooleanBehaviour, msg: MidiMsg) {
        self.buttons.insert(cmd, (behaviour, msg.to_midi().into()));
        self.save()
    }

    pub(crate) fn rm_btn(&mut self, cmd: BoolCommand) {
        self.buttons.remove(&cmd);
        self.save()
    }
    
    pub(crate) fn set_midi_device(&mut self, device: String) {
        self.midi_device = device;
        self.save()
    }
    
}

