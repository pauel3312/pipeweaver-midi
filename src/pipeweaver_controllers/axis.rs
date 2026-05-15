use crate::behaviours::axis_behaviours::{AxisBehaviour, AxisBehaviourTrait};
use crate::pipeweaver_controllers::commands::AxisCommand;
use crate::pipeweaver_controllers::core::{AxisProvider, CallbackProvider, ControllerCore};
use pipeweaver_ipc::commands::DaemonRequest;
use std::fmt::{Debug, Formatter};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Sender;

pub struct AxisController<F>
where
    F: Fn(u8) -> DaemonRequest + Send + Sync + 'static,
{
    core: ControllerCore<AxisBehaviour>,
    drq_map: F,
}

impl<F> AxisController<F>
where
    F: Fn(u8) -> DaemonRequest + Send + Sync + 'static,
{
    pub fn new(behaviour: Arc<Mutex<AxisBehaviour>>, tx: Sender<DaemonRequest>, drq_map: F) -> Self {
        Self {
            core: ControllerCore::new(behaviour, tx),
            drq_map,
        }
    }
}

impl<F> Debug for AxisController<F>
where
    F: 'static + Fn(u8) -> DaemonRequest + Send + Sync,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("AxisController")
    }
}

impl<F> CallbackProvider for AxisController<F>
where
    F: Fn(u8) -> DaemonRequest + Send + Sync + 'static,
{
    fn callback(&self, data: u8) {
        let value = self.core.with_behaviour(|b| b.get(data));
        let req = (self.drq_map)(value);
        self.core.send(req);
    }
}

impl<F> AxisProvider for AxisController<F>
where
    F: Fn(u8) -> DaemonRequest + Send + Sync + 'static,
{
    fn set(&self, data: u8) {
        self.core.with_behaviour(|b| b.set(data));
    }
}

pub fn axis_controller(
    command: AxisCommand,
    behaviour: Arc<Mutex<AxisBehaviour>>,
    tx: Sender<DaemonRequest>,
) -> impl AxisProvider + Send + Sync {
    AxisController::new(behaviour, tx, move |data: u8| command.to_request(data))
}
