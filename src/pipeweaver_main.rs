use crate::common::SharedState;
use crate::midi::learn_event::learn_thread;
use crate::midi::manager::MidiMgr;
use pipeweaver_websocket_client::{BroadcastMessage, spawn_pipeweaver_handler};
use std::sync::{Arc, Mutex};
use tokio::signal;
use tokio::sync::{broadcast, mpsc};

pub async fn main(state: Arc<Mutex<SharedState>>, midi_mgr: Arc<Mutex<MidiMgr>>) {
    // Create a channel for broadcasting changes
    let (broadcast, _) = broadcast::channel(10);

    // Create a subscription for the broadcast channel
    let mut subscription = broadcast.subscribe();

    // Create a channel for sending commands
    let (tx, rx) = mpsc::channel(10);

    // Spawn up the Pipeweaver handler, which will return a way to stop it.
    let stopper = spawn_pipeweaver_handler(rx, broadcast.clone()).await;

    // let mut can_send: bool = false;

    {
        let mut state = state.lock().unwrap();
        state.tx = Some(tx.clone());
        state.initialize();
    }

    midi_mgr.lock().unwrap().connect(state.clone()).unwrap();

    tokio::spawn(learn_thread(state.clone()));

    loop {
        tokio::select! {
            Ok(message) = subscription.recv() => {
                match message {
                    BroadcastMessage::Online => {
                        println!("Connected to Pipeweaver");
                        // can_send = true;
                    }
                    BroadcastMessage::Offline => {
                        println!("Connection to Pipeweaver lost, reconnecting in 5 seconds...");
                        // can_send = false;
                        state.lock().unwrap().status = None
                    }
                    BroadcastMessage::Status(new_status) => {
                        // Is this the first time we've seen the status since connecting?
                        if state.lock().unwrap().status.is_none() {
                        }

                        // Send received data back to the axis behaviours.
                        for (id, pvd) in &mut state.lock().unwrap().axes {
                            let val = id.get_value(new_status.as_ref());
                            match val {
                                Some(d) => { pvd.lock().unwrap().set(d); }
                                None => {}
                            }
                        }

                        // Send received data back to the bool behaviours
                        for (id, pvd) in &mut state.lock().unwrap().buttons {
                            let val = id.get_value(new_status.as_ref());
                            match val {
                                Some(d) => { pvd.lock().unwrap().set(d); }
                                None => {}
                            }
                        }
                        state.lock().unwrap().status = Some(*new_status);
                    }
                }
            }
            _ = signal::ctrl_c() => {
                println!("Stopping Pipeweaver Manager");
                stopper.trigger();
                break;
            }
        }
    }
}
