use crate::common::SharedState;
use crate::midi::callback_tree::is_event_valid;
use midi_msg::MidiMsg;
use midir::{MidiInput, MidiInputConnection};
use std::sync::{Arc, Mutex};

pub(crate) struct MidiMgr {
    pub(crate) conn: Option<MidiInputConnection<()>>,
}

impl MidiMgr {
    pub(crate) fn new() -> Self {
        Self { conn: None }
    }

    pub(crate) fn connect(&mut self, state: Arc<Mutex<SharedState>>) -> anyhow::Result<()> {
        let midi = MidiInput::new("pipeweaver-midi")?;
        let port = {
            let mut state = state.lock().unwrap();
            let port = state.current_port.clone();
            state.config.set_midi_device(midi.port_name(&port)?);
            port
        };
        let name = midi.port_name(&port)?;
        let state = state.clone();
        let conn = midi
            .connect(
                &port,
                name.clone().as_str(),
                move |_tm, data, _t| {
                    let rst = MidiMsg::from_midi(data);
                    match rst {
                        Err(e) => {
                            println!("Failed to connect to {}: {}", &name, e)
                        }
                        Ok((msg, _)) => {
                            if is_event_valid(msg.clone()) {
                                let mut state = state.lock().unwrap();
                                state.midi_tree.exec(&msg).unwrap();
                                state.last_midi_event = Some(msg);
                            }
                        }
                    }
                },
                (),
            )
            .unwrap();
        self.conn = Some(conn);
        Ok(())
    }

    pub(crate) fn disconnect(&mut self) {
        if let Some(conn) = self.conn.take() {
            conn.close();
        }
    }
}
