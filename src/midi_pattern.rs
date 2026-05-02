use crate::pwv_controllers::CallbackProvider;
use midi_msg::ChannelVoiceMsg::{ControlChange, NoteOff, NoteOn};
use midi_msg::MidiMsg::ChannelVoice;
use midi_msg::{Channel, ChannelVoiceMsg, MidiMsg};
use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::sync::{Arc, Mutex};

#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
pub enum ChannelVoiceKind {
    Note,
    ControlChange,
    Other,
}

impl From<&ChannelVoiceMsg> for ChannelVoiceKind {
    fn from(msg: &ChannelVoiceMsg) -> Self {
        match msg {
            NoteOn {
                note: _,
                velocity: _,
            } => ChannelVoiceKind::Note,
            NoteOff {
                note: _,
                velocity: _,
            } => ChannelVoiceKind::Note,
            ControlChange { control: _ } => ChannelVoiceKind::ControlChange,
            _ => ChannelVoiceKind::Other,
        }
    }
}
impl ChannelVoiceKind {
    fn get_place(self, msg: ChannelVoiceMsg) -> Result<u8, Error> {
        match self {
            ChannelVoiceKind::Note => match msg {
                NoteOn {
                    note: n,
                    velocity: _,
                } => Ok(n),
                NoteOff {
                    note: n,
                    velocity: _,
                } => Ok(n),
                _ => Err(Error::new(
                    ErrorKind::InvalidData,
                    "Invalid message for Note CVK: is not NoteOn or NoteOff",
                )),
            },
            ChannelVoiceKind::ControlChange => match msg {
                ControlChange { control: c } => Ok(c.to_simple().control()),
                _ => Err(Error::new(
                    ErrorKind::InvalidData,
                    "Invalid message for CC CVK: is not CC",
                )),
            },
            ChannelVoiceKind::Other => Err(Error::new(
                ErrorKind::InvalidData,
                "Cant get place for CVK other than CC or Note",
            )),
        }
    }

    fn get_val(self, msg: ChannelVoiceMsg) -> Result<u8, Error> {
        match self {
            ChannelVoiceKind::Note => match msg {
                NoteOn {
                    note: _,
                    velocity: _,
                } => Ok(127),
                NoteOff {
                    note: _,
                    velocity: _,
                } => Ok(0),
                _ => Err(Error::new(
                    ErrorKind::InvalidData,
                    "Invalid message for Note CVK: is not NoteOn or NoteOff",
                )),
            },
            ChannelVoiceKind::ControlChange => match msg {
                ControlChange { control: c } => Ok(c.to_simple().value()),
                _ => Err(Error::new(
                    ErrorKind::InvalidData,
                    "Invalid message for CC CVK: is not CC",
                )),
            },
            ChannelVoiceKind::Other => Err(Error::new(
                ErrorKind::InvalidData,
                "Cant get value for CVK other than CC or Note",
            )),
        }
    }
}

#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
enum HashChannel {
    Ch1,
    Ch2,
    Ch3,
    Ch4,
    Ch5,
    Ch6,
    Ch7,
    Ch8,
    Ch9,
    Ch10,
    Ch11,
    Ch12,
    Ch13,
    Ch14,
    Ch15,
    Ch16,
}

impl From<&Channel> for HashChannel {
    fn from(channel: &Channel) -> Self {
        match channel {
            Channel::Ch1 => HashChannel::Ch1,
            Channel::Ch2 => HashChannel::Ch2,
            Channel::Ch3 => HashChannel::Ch3,
            Channel::Ch4 => HashChannel::Ch4,
            Channel::Ch5 => HashChannel::Ch5,
            Channel::Ch6 => HashChannel::Ch6,
            Channel::Ch7 => HashChannel::Ch7,
            Channel::Ch8 => HashChannel::Ch8,
            Channel::Ch9 => HashChannel::Ch9,
            Channel::Ch10 => HashChannel::Ch10,
            Channel::Ch11 => HashChannel::Ch11,
            Channel::Ch12 => HashChannel::Ch12,
            Channel::Ch13 => HashChannel::Ch13,
            Channel::Ch14 => HashChannel::Ch14,
            Channel::Ch15 => HashChannel::Ch15,
            Channel::Ch16 => HashChannel::Ch16,
        }
    }
}

pub type Callback = Arc<Mutex<dyn CallbackProvider + Send>>;
#[derive(Clone)]
pub struct MidiMsgCallbackTree {
    channels: HashMap<HashChannel, ChannelVoiceMsgCallbackTree>,
}

impl MidiMsgCallbackTree {
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
        }
    }

    pub fn insert_callback(&mut self, event: &MidiMsg, callback: Callback) -> Result<(), Error> {
        match event {
            ChannelVoice { channel, msg } => {
                let hash_channel = HashChannel::from(channel);
                let cvk = ChannelVoiceKind::from(msg);
                let place = cvk.get_place(msg.clone())?;
                match self.channels.get_mut(&hash_channel) {
                    Some(next) => next.insert_callback(cvk, place, callback),
                    None => {
                        let mut next = ChannelVoiceMsgCallbackTree::new();
                        next.insert_callback(cvk, place, callback)?;
                        self.channels.insert(hash_channel, next);
                        Ok(())
                    }
                }
            }
            _ => Err(Error::new(
                ErrorKind::InvalidInput,
                "Tried to insert callback for non-CV event.",
            )),
        }
    }

    pub fn get_if_exists(&self, event: &MidiMsg) -> Option<Callback> {
        match event {
            ChannelVoice { channel, msg } => {
                let hash_channel = HashChannel::from(channel);
                let cvk = ChannelVoiceKind::from(msg);
                let place = cvk.get_place(msg.clone()).unwrap();
                match self.channels.get(&hash_channel) {
                    Some(next) => next.get_if_exists(cvk, place),
                    None => None,
                }
            }
            _ => None,
        }
    }

    pub fn rm_callback(&mut self, event: &MidiMsg) {
        match event {
            ChannelVoice { channel, msg } => {
                let hash_channel = HashChannel::from(channel);
                let cvk = ChannelVoiceKind::from(msg);
                let place = cvk.get_place(msg.clone()).unwrap();
                match self.channels.get_mut(&hash_channel) {
                    Some(next) => next.rm_callback(cvk, place),
                    None => {}
                }
            }
            _ => {}
        }
    }

    pub fn exec(&self, event: &MidiMsg) -> Result<bool, Error> {
        match event {
            ChannelVoice { channel, msg } => {
                let hash_channel = HashChannel::from(channel);
                let cvk = ChannelVoiceKind::from(msg);
                let place = cvk.get_place(msg.clone())?;
                match self.channels.get(&hash_channel) {
                    Some(next) => match next.get_if_exists(cvk, place) {
                        Some(cb) => {
                            cb.lock().unwrap().callback(cvk.get_val(msg.clone())?);
                            Ok(true)
                        }
                        None => Ok(false),
                    },
                    None => Ok(false),
                }
            }
            _ => Ok(false),
        }
    }
}

#[derive(Clone)]
pub struct ChannelVoiceMsgCallbackTree {
    types: HashMap<ChannelVoiceKind, CallbackTreeLeaves>,
}

impl ChannelVoiceMsgCallbackTree {
    pub fn new() -> Self {
        Self {
            types: HashMap::new(),
        }
    }

    pub fn insert_callback(
        &mut self,
        channel: ChannelVoiceKind,
        place: u8,
        callback: Callback,
    ) -> Result<(), Error> {
        match self.types.get_mut(&channel) {
            Some(next) => next.insert_callback(place, callback),
            None => {
                let mut next = CallbackTreeLeaves::new();
                next.insert_callback(place, callback)?;
                self.types.insert(channel, next);
                Ok(())
            }
        }
    }

    pub fn get_if_exists(&self, channel: ChannelVoiceKind, place: u8) -> Option<Callback> {
        match self.types.get(&channel) {
            Some(next) => next.get_if_exists(place),
            None => None,
        }
    }

    pub fn rm_callback(&mut self, channel: ChannelVoiceKind, place: u8) {
        match self.types.get_mut(&channel) {
            Some(next) => next.remove_callback(place),
            None => (),
        }
    }
}

#[derive(Clone)]
pub struct CallbackTreeLeaves {
    callbacks: HashMap<u8, Callback>,
}

impl CallbackTreeLeaves {
    pub fn new() -> Self {
        Self {
            callbacks: HashMap::new(),
        }
    }

    pub fn insert_callback(&mut self, place: u8, callback: Callback) -> Result<(), Error> {
        self.callbacks.insert(place, callback);
        Ok(())
    }

    pub fn get_if_exists(&self, place: u8) -> Option<Callback> {
        self.callbacks.get(&place).cloned()
    }

    pub fn remove_callback(&mut self, place: u8) {
        self.callbacks.remove(&place);
    }
}
