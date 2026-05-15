use crate::pipeweaver_controllers::core::CallbackProvider;
use midi_msg::ChannelVoiceMsg::{ControlChange, NoteOff, NoteOn};
use midi_msg::{Channel, ChannelVoiceMsg};
use std::io::{Error, ErrorKind};
use std::sync::{Arc, Mutex};

#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
pub(crate) enum ChannelVoiceKind {
    Note,
    ControlChange,
    Other,
}

impl From<&ChannelVoiceMsg> for ChannelVoiceKind {
    fn from(msg: &ChannelVoiceMsg) -> Self {
        match msg {
            NoteOn { note: _, velocity: _ } => ChannelVoiceKind::Note,
            NoteOff { note: _, velocity: _ } => ChannelVoiceKind::Note,
            ControlChange { control: _ } => ChannelVoiceKind::ControlChange,
            _ => ChannelVoiceKind::Other,
        }
    }
}
impl ChannelVoiceKind {
    pub(super) fn get_place(self, msg: ChannelVoiceMsg) -> Result<u8, Error> {
        match self {
            ChannelVoiceKind::Note => match msg {
                NoteOn { note: n, velocity: _ } => Ok(n),
                NoteOff { note: n, velocity: _ } => Ok(n),
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

    pub(super) fn get_val(self, msg: ChannelVoiceMsg) -> Result<u8, Error> {
        match self {
            ChannelVoiceKind::Note => match msg {
                NoteOn { note: _, velocity: _ } => Ok(127),
                NoteOff { note: _, velocity: _ } => Ok(0),
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
pub(super) enum HashChannel {
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

pub(crate) type Callback = Arc<Mutex<dyn CallbackProvider + Send>>;
