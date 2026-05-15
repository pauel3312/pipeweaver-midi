use crate::midi::util::{Callback, ChannelVoiceKind, HashChannel};
use midi_msg::ChannelVoiceMsg::{ControlChange, NoteOff, NoteOn};
use midi_msg::MidiMsg;
use midi_msg::MidiMsg::ChannelVoice;
use std::collections::HashMap;
use std::fmt::Debug;
use std::io::{Error, ErrorKind};

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
pub struct ChannelVoiceMsgCallbackTree {
    types: HashMap<ChannelVoiceKind, CallbackTreeLeaves>,
}

impl ChannelVoiceMsgCallbackTree {
    pub fn new() -> Self {
        Self { types: HashMap::new() }
    }

    pub(crate) fn insert_callback(
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

    pub(crate) fn get_if_exists(&self, channel: ChannelVoiceKind, place: u8) -> Option<Callback> {
        match self.types.get(&channel) {
            Some(next) => next.get_if_exists(place),
            None => None,
        }
    }

    pub(crate) fn rm_callback(&mut self, channel: ChannelVoiceKind, place: u8) {
        match self.types.get_mut(&channel) {
            Some(next) => next.remove_callback(place),
            None => (),
        }
    }
}

#[derive(Clone, Debug)]
struct CallbackTreeLeaves {
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

pub fn is_event_valid(event: MidiMsg) -> bool {
    match event {
        ChannelVoice { channel: _, msg } => match msg {
            NoteOn { .. } | NoteOff { .. } | ControlChange { .. } => true,
            _ => false,
        },
        _ => false,
    }
}
