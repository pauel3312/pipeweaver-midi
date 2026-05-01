use crate::behaviours::{AxisProvider, BooleanProvider};
use pipeweaver_ipc::commands::{APICommand, DaemonRequest};
use pipeweaver_shared::{Mix, MuteState, MuteTarget};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Sender;
use ulid::Ulid;

pub trait AxisStateSetterProvider {
    fn set(&self, data: u8);
}

pub trait BooleanStateSetterProvider {
    fn set(&self, data: bool);
}

pub trait CallbackProvider {
    fn callback(&self, data: u8);
}

pub struct ControllerCore<B: ?Sized> {
    behaviour: Arc<Mutex<B>>,
    tx: Arc<Mutex<Sender<DaemonRequest>>>,
}

impl<B: ?Sized> ControllerCore<B> {
    pub fn new(behaviour: Arc<Mutex<B>>, tx: Arc<Mutex<Sender<DaemonRequest>>>) -> Self {
        Self { behaviour, tx }
    }

    pub fn with_behaviour<R>(&self, f: impl FnOnce(&mut B) -> R) -> R {
        let mut b = self.behaviour.lock().unwrap();
        f(&mut b)
    }

    pub fn send(&self, req: DaemonRequest) {
        let _ = self.tx.lock().unwrap().send(req);
    }
}

pub struct AxisController<F>
where
    F: Fn(u8) -> DaemonRequest + Send + Sync + 'static,
{
    core: ControllerCore<dyn AxisProvider + Send + Sync>,
    drq_map: F,
}

impl<F> AxisController<F>
where
    F: Fn(u8) -> DaemonRequest + Send + Sync + 'static,
{
    pub fn new(
        behaviour: Arc<Mutex<dyn AxisProvider + Send + Sync>>,
        tx: Arc<Mutex<Sender<DaemonRequest>>>,
        drq_map: F,
    ) -> Self {
        Self {
            core: ControllerCore::new(behaviour, tx),
            drq_map,
        }
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

impl<F> AxisStateSetterProvider for AxisController<F>
where
    F: Fn(u8) -> DaemonRequest + Send + Sync + 'static,
{
    fn set(&self, data: u8) {
        self.core.with_behaviour(|b| b.set(data));
    }
}

pub struct BooleanController<F>
where
    F: Fn(bool) -> DaemonRequest + Send + Sync + 'static,
{
    core: ControllerCore<dyn BooleanProvider + Send + Sync>,
    drq_map: F,
}

impl<F> BooleanController<F>
where
    F: Fn(bool) -> DaemonRequest + Send + Sync + 'static,
{
    pub fn new(
        behaviour: Arc<Mutex<dyn BooleanProvider + Send + Sync>>,
        tx: Arc<Mutex<Sender<DaemonRequest>>>,
        drq_map: F,
    ) -> Self {
        Self {
            core: ControllerCore::new(behaviour, tx),
            drq_map,
        }
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

impl<F> BooleanStateSetterProvider for BooleanController<F>
where
    F: Fn(bool) -> DaemonRequest + Send + Sync + 'static,
{
    fn set(&self, data: bool) {
        self.core.with_behaviour(|b| b.set(data));
    }
}

pub fn source_volume_controller(
    id: Ulid,
    mix: Mix,
    behaviour: Arc<Mutex<dyn AxisProvider + Send + Sync>>,
    tx: Arc<Mutex<Sender<DaemonRequest>>>,
) -> impl AxisStateSetterProvider + CallbackProvider {
    AxisController::new(behaviour, tx, move |data: u8| -> DaemonRequest {
        DaemonRequest::Pipewire(APICommand::SetSourceVolume(id, mix, data))
    })
}

pub fn target_volume_controller(
    id: Ulid,
    behaviour: Arc<Mutex<dyn AxisProvider + Send + Sync>>,
    tx: Arc<Mutex<Sender<DaemonRequest>>>,
) -> impl AxisStateSetterProvider + CallbackProvider {
    AxisController::new(behaviour, tx, move |data: u8| -> DaemonRequest {
        DaemonRequest::Pipewire(APICommand::SetTargetVolume(id, data))
    })
}

pub fn route_controller(
    in_id: Ulid,
    out_id: Ulid,
    behaviour: Arc<Mutex<dyn BooleanProvider + Send + Sync>>,
    tx: Arc<Mutex<Sender<DaemonRequest>>>,
) -> impl BooleanStateSetterProvider + CallbackProvider {
    BooleanController::new(behaviour, tx, move |data: bool| -> DaemonRequest {
        DaemonRequest::Pipewire(APICommand::SetRoute(in_id, out_id, data))
    })
}

pub fn source_mute_controller(
    id: Ulid,
    target: MuteTarget,
    behaviour: Arc<Mutex<dyn BooleanProvider + Send + Sync>>,
    tx: Arc<Mutex<Sender<DaemonRequest>>>,
) -> impl BooleanStateSetterProvider + CallbackProvider {
    BooleanController::new(behaviour, tx, move |data: bool| -> DaemonRequest {
        let command: APICommand;
        if data {
            command = APICommand::AddSourceMuteTarget(id, target);
        } else {
            command = APICommand::DelSourceMuteTarget(id, target);
        }
        DaemonRequest::Pipewire(command)
    })
}

pub fn target_mute_controller(
    id: Ulid,
    behaviour: Arc<Mutex<dyn BooleanProvider + Send + Sync>>,
    tx: Arc<Mutex<Sender<DaemonRequest>>>,
) -> impl BooleanStateSetterProvider + CallbackProvider {
    BooleanController::new(behaviour, tx, move |data: bool| -> DaemonRequest {
        let state: MuteState = if data {
            MuteState::Muted
        } else {
            MuteState::Unmuted
        };
        DaemonRequest::Pipewire(APICommand::SetTargetMuteState(id, state))
    })
}

pub fn target_mix_controller(
    id: Ulid,
    behaviour: Arc<Mutex<dyn BooleanProvider + Send + Sync>>,
    tx: Arc<Mutex<Sender<DaemonRequest>>>
) -> impl BooleanStateSetterProvider + CallbackProvider {
    BooleanController::new(behaviour, tx, move |data: bool| -> DaemonRequest {
        let mix = if data { Mix::A } else { Mix::B };
        DaemonRequest::Pipewire(APICommand::SetTargetMix(id, mix))
    })
}