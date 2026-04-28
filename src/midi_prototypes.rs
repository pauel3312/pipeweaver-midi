use std::error::Error;
use std::io::ErrorKind;
use tokio::time::Duration;
use midir::{MidiInput, MidiInputPort};
use midi_msg::MidiMsg;

fn print_midi_callback(timestamp_micros: u64, data: &[u8], _t: &mut ()){
    println!("{}: {:?}", timestamp_micros, MidiMsg::from_midi(data));
}

pub(crate) async fn main() -> Result<(), Box<dyn Error>> {
    println!("Starting MIDI...");
    let midi = MidiInput::new("test-midi")?;
    let ports = midi.ports();
    let mut midi_device_opt: Option<MidiInputPort> = None;
    for (i, p) in ports.iter().enumerate() {
        let name = midi.port_name(p)?;
        println!("{}: {}", i, name);
        if i == 1 {
            midi_device_opt = Some(p.clone());
            break;
        }
    }

    if midi_device_opt.is_none() {
        return Err(std::io::Error::new(ErrorKind::Other, "MIDI device not found").into());
    }

    let midi_device = midi_device_opt.unwrap();
    let name = midi.port_name(&midi_device)?;

    println!("{}", midi_device.id());
    let _connection = midi.connect(&midi_device, name.as_str(), print_midi_callback, ())?;


    loop{
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}
