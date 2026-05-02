use crate::midi_pattern::{Callback, MidiMsgCallbackTree};
use midi_msg::ControlChange::CC;
use midi_msg::{Channel, ChannelVoiceMsg, MidiMsg};
use midir::{MidiInput, MidiInputConnection};
use std::error::Error;
use std::io::ErrorKind;
use std::sync::{Arc, Mutex};
use tokio::signal;
use crate::pwv_controllers::PrinterController;

#[allow(unused)] // This is a testing function
fn print_midi_callback(timestamp_micros: u64, data: &[u8], _t: &mut ()) {
    println!(
        "{}: {:?}",
        timestamp_micros,
        MidiMsg::from_midi(data).unwrap().0
    );
}
pub(crate) async fn main_wrap() {
    main().await.unwrap();
}

async fn main() -> Result<(), Box<dyn Error>> {
    println!("Starting MIDI...");
    let midi = MidiInput::new("midi-discover")?;
    let ports = midi.ports();
    let callback: Callback = Arc::new(Mutex::new(PrinterController{name:"TEST".to_string()}));

    let mut connections: Vec<MidiInputConnection<()>> = Vec::new();

    for (i, p) in ports.iter().enumerate() {
        let name = midi.port_name(p)?;
        println!("{}: {}", i, name);
        let callback = callback.clone();
        let mut current_tree = MidiMsgCallbackTree::new();
        current_tree.insert_callback(
            &MidiMsg::ChannelVoice {
                channel: Channel::Ch1,
                msg: ChannelVoiceMsg::ControlChange {
                    control: CC {
                        control: 20,
                        value: 0,
                    },
                },
            },
            callback.clone(),
        )?;
        current_tree.insert_callback(
            &MidiMsg::ChannelVoice {
                channel: Channel::Ch1,
                msg: ChannelVoiceMsg::NoteOn {
                    note: 48,
                    velocity: 0,
                },
            },
            callback.clone(),
        )?;

        let current_tree = Arc::new(Mutex::new(current_tree));
        let tree_handle = Arc::clone(&current_tree);

        let tmp_midi = MidiInput::new(format!("midi-{}", name).as_str())?;
        connections.push(tmp_midi.connect(
            &p,
            name.as_str(),
            move |_t, data, _t2| {
                // print_midi_callback(_t, data, _t2);
                let msg = &MidiMsg::from_midi(data).unwrap().0;
                match tree_handle.lock().unwrap().exec(msg) {
                    Ok(_) => {}
                    Err(e) => {
                        if e.kind() != ErrorKind::InvalidData {
                            eprintln!("{}", e);
                        }
                    }
                }
            },
            (),
        )?);
        current_tree.lock().unwrap().insert_callback(
            &MidiMsg::ChannelVoice {
                channel: Channel::Ch1,
                msg: ChannelVoiceMsg::ControlChange {
                    control: CC {
                        control: 21,
                        value: 0,
                    },
                },
            },
            callback,
        )?;
    }

    signal::ctrl_c().await?;
    while let Some(conn) = connections.pop() {
        conn.close();
        println!("Stopping a connection. Remains {}", connections.len());
    }
    Ok(())
}
