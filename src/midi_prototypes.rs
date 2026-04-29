use midi_msg::{Channel, ChannelVoiceMsg, MidiMsg};
use midir::{MidiInput, MidiInputConnection};
use std::error::Error;
use std::io::ErrorKind;
use midi_msg::ControlChange::CC;
use tokio::signal;
use crate::midi_pattern::{Callback, MidiMsgCallbackTree};

fn print_midi_callback(timestamp_micros: u64, data: &[u8], _t: &mut ()) {
    println!(
        "{}: {:?}",
        timestamp_micros,
        MidiMsg::from_midi(data).unwrap().0
    );
}

pub(crate) async fn main() -> Result<(), Box<dyn Error>> {
    println!("Starting MIDI...");
    let midi = MidiInput::new("midi-discover")?;
    let ports = midi.ports();
    let callback: Callback = |c: u8| {println!("{}", c)};


    let mut connections: Vec<MidiInputConnection<()>> = Vec::new();

    for (i, p) in ports.iter().enumerate() {
        let name = midi.port_name(p)?;
        println!("{}: {}", i, name);
        let mut current_tree = MidiMsgCallbackTree::new();
        current_tree.insert_callback(
            &MidiMsg::ChannelVoice {
                channel: Channel::Ch1,
                msg: ChannelVoiceMsg::ControlChange {
                    control: CC{
                        control: 20,
                        value: 0,
                    }
                }},
        callback)?;
        current_tree.insert_callback(
            &MidiMsg::ChannelVoice {
                channel: Channel::Ch1,
                msg: ChannelVoiceMsg::NoteOn {
                    note: 48,
                    velocity: 0,
                }},
            callback)?;

        let tmp_midi = MidiInput::new(format!("midi-{}", name).as_str())?;
        connections.push(tmp_midi.connect(
            &p,
            name.as_str(),
            move |_t, data, _t2| {
                // print_midi_callback(_t, data, _t2);
                let msg = &MidiMsg::from_midi(data).unwrap().0;
                match current_tree.exec(msg) {
                    Ok(_) => {},
                    Err(e) => {
                        if e.kind() != ErrorKind::InvalidData {
                            eprintln!("{}", e);
                        }
                    }
                }
            },
            (),
        )?)
    }

    signal::ctrl_c().await?;
    while let Some(conn) = connections.pop() {
        conn.close();
        println!("Stopping a connection. Remains {}", connections.len());
    }
    Ok(())
}
