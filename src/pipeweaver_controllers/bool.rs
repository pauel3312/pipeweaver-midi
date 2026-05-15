use crate::behaviours::button_behaviours::{BooleanBehaviour, BooleanBehaviourTrait};
use crate::pipeweaver_controllers::commands::BoolCommand;
use crate::pipeweaver_controllers::core::{BooleanProvider, CallbackProvider, ControllerCore};
use pipeweaver_ipc::commands::DaemonRequest;
use std::fmt::{Debug, Formatter};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Sender;

pub struct BooleanController<F>
where
    F: Fn(bool) -> DaemonRequest + Send + Sync + 'static,
{
    core: ControllerCore<BooleanBehaviour>,
    drq_map: F,
}

impl<F> BooleanController<F>
where
    F: Fn(bool) -> DaemonRequest + Send + Sync + 'static,
{
    pub fn new(behaviour: Arc<Mutex<BooleanBehaviour>>, tx: Sender<DaemonRequest>, drq_map: F) -> Self {
        Self {
            core: ControllerCore::new(behaviour, tx),
            drq_map,
        }
    }
}

impl<F> Debug for BooleanController<F>
where
    F: 'static + Fn(bool) -> DaemonRequest + Send + Sync,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("BooleanController")
    }
}

impl<F> CallbackProvider for BooleanController<F>
where
    F: Fn(bool) -> DaemonRequest + Send + Sync + 'static,
{
    fn callback(&self, data: u8) {
        let value = self.core.with_behaviour(|b| b.get(data));
        let req = (self.drq_map)(value);
        self.core.send(req);
    }
}

impl<F> BooleanProvider for BooleanController<F>
where
    F: Fn(bool) -> DaemonRequest + Send + Sync + 'static,
{
    fn set(&self, data: bool) {
        self.core.with_behaviour(|b| b.set(data));
    }
}

pub fn bool_controller(
    command: BoolCommand,
    behaviour: Arc<Mutex<BooleanBehaviour>>,
    tx: Sender<DaemonRequest>,
) -> impl BooleanProvider + Send + Sync {
    BooleanController::new(behaviour, tx, move |data: bool| command.to_request(data))
}
