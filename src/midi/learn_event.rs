use crate::common::SharedState;
use midi_msg::MidiMsg;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::task::JoinHandle;

pub(crate) async fn learn_thread(state: Arc<Mutex<SharedState>>) -> JoinHandle<()> {
    loop {
        if state.lock().unwrap().learning {
            state.lock().unwrap().learn_msg = Some(wait_on_midi_event_changed(state.clone()).await);
            state.lock().unwrap().learning = false;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn wait_on_midi_event_changed(state: Arc<Mutex<SharedState>>) -> MidiMsg {
    let start_msg = state.lock().unwrap().last_midi_event.clone();
    loop {
        let event = state.lock().unwrap().last_midi_event.clone();
        if start_msg != event {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    state.lock().unwrap().last_midi_event.clone().unwrap()
}
