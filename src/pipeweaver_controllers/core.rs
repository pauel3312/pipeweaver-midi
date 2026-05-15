use pipeweaver_ipc::commands::DaemonRequest;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Sender;

pub trait AxisProvider: CallbackProvider {
    fn set(&self, data: u8);
}

pub trait BooleanProvider: CallbackProvider {
    fn set(&self, data: bool);
}

pub trait CallbackProvider: Debug {
    fn callback(&self, data: u8);
}

#[derive(Debug)]
pub struct ControllerCore<B: ?Sized> {
    behaviour: Arc<Mutex<B>>,
    tx: Sender<DaemonRequest>,
}

impl<B: ?Sized> ControllerCore<B> {
    pub fn new(behaviour: Arc<Mutex<B>>, tx: Sender<DaemonRequest>) -> Self {
        Self { behaviour, tx }
    }

    pub fn with_behaviour<R>(&self, f: impl FnOnce(&mut B) -> R) -> R {
        let mut b = self.behaviour.lock().unwrap();
        f(&mut b)
    }

    pub fn send(&self, req: DaemonRequest) {
        match self.tx.try_send(req) {
            Ok(_) => (),
            Err(_) => {}
        }
    }
}
