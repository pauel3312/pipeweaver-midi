use crate::behaviours::{AxisBehaviour, BooleanBehaviour};
use pipeweaver_ipc::commands::{APICommand, DaemonRequest, DaemonStatus};
use pipeweaver_shared::{Mix, MuteState, MuteTarget};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Sender;
use ulid::Ulid;

pub trait AxisProvider: CallbackProvider {
    fn set(&self, data: u8);
}

pub trait BooleanProvider: CallbackProvider {
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
    core: ControllerCore<dyn AxisBehaviour + Send + Sync>,
    drq_map: F,
}

impl<F> AxisController<F>
where
    F: Fn(u8) -> DaemonRequest + Send + Sync + 'static,
{
    pub fn new(
        behaviour: Arc<Mutex<dyn AxisBehaviour + Send + Sync>>,
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

impl<F> AxisProvider for AxisController<F>
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
    core: ControllerCore<dyn BooleanBehaviour + Send + Sync>,
    drq_map: F,
}

impl<F> BooleanController<F>
where
    F: Fn(bool) -> DaemonRequest + Send + Sync + 'static,
{
    pub fn new(
        behaviour: Arc<Mutex<dyn BooleanBehaviour + Send + Sync>>,
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

impl<F> BooleanProvider for BooleanController<F>
where
    F: Fn(bool) -> DaemonRequest + Send + Sync + 'static,
{
    fn set(&self, data: bool) {
        self.core.with_behaviour(|b| b.set(data));
    }
}


pub enum AxisCommand {
    SourceVolume{id: Ulid, mix: Mix},
    TargetVolume{id: Ulid},
}

pub enum BoolCommand {
    Route { in_id: Ulid, out_id: Ulid },
    SourceMute { id: Ulid, target: MuteTarget },
    TargetMute { id: Ulid },
    TargetMix { id: Ulid },
}


impl AxisCommand {
    fn to_request(&self, data: u8) -> DaemonRequest {
        match self {
            AxisCommand::SourceVolume { id, mix } =>
                DaemonRequest::Pipewire(APICommand::SetSourceVolume(*id, *mix, data)),

            AxisCommand::TargetVolume { id } =>
                DaemonRequest::Pipewire(APICommand::SetTargetVolume(*id, data)),
        }
    }

    pub fn get_value(&self, status: &DaemonStatus) -> Option<u8> {
        let mut val: Option<u8> = None;
        match *self {
            AxisCommand::SourceVolume{id, mix} => {
                let phy_devices = &status.audio.profile.devices.sources.physical_devices;
                let virt_devices = &status.audio.profile.devices.sources.virtual_devices;
                for pd in phy_devices {
                    if pd.description.id != id {continue;}
                    val = Some(pd.volumes.volume[mix]);
                    break
                }
                if val != None {return val;}
                for vd in virt_devices {
                    if vd.description.id != id {continue;}
                    val = Some(vd.volumes.volume[mix]);
                    break
                }
            },
            AxisCommand::TargetVolume { id } => {
                let phy_devices = &status.audio.profile.devices.targets.physical_devices;
                let virt_devices = &status.audio.profile.devices.targets.virtual_devices;
                for pd in phy_devices {
                    if pd.description.id != id {continue;}
                    val = Some(pd.volume);
                    break
                }
                if val != None {return val;}
                for vd in virt_devices {
                    if vd.description.id != id {continue;}
                    val = Some(vd.volume);
                    break
                }
            }
        }
        val
    }

}

impl BoolCommand {
    fn to_request(&self, data: bool) -> DaemonRequest {
        match self {
            BoolCommand::Route {in_id, out_id} => {
                DaemonRequest::Pipewire(APICommand::SetRoute(*in_id, *out_id, data))
            }
            BoolCommand::SourceMute {id, target} => {
                let command: APICommand;
                if data {
                    command = APICommand::AddSourceMuteTarget(*id, *target);
                } else {
                    command = APICommand::DelSourceMuteTarget(*id, *target);
                }
                DaemonRequest::Pipewire(command)
            }
            BoolCommand::TargetMute {id} => {
                let state: MuteState = if data {
                    MuteState::Muted
                } else {
                    MuteState::Unmuted
                };
                DaemonRequest::Pipewire(APICommand::SetTargetMuteState(*id, state))
            }
            BoolCommand::TargetMix {id} => {
                let mix = if data { Mix::A } else { Mix::B };
                DaemonRequest::Pipewire(APICommand::SetTargetMix(*id, mix))
            }
        }
    }
    pub fn get_value(&self, status: &DaemonStatus) -> Option<bool> {
        match self {
            BoolCommand::Route { in_id, out_id } => {
                Some(status.audio.profile.routes[in_id].contains(out_id))
            }
            BoolCommand::SourceMute { id, target } => {
                let phy_devices = &status.audio.profile.devices.sources.physical_devices;
                let virt_devices = &status.audio.profile.devices.sources.virtual_devices;
                let mut val: Option<bool> = None;
                for pd in phy_devices {
                    if pd.description.id != *id {continue;}
                    val = Some(pd.mute_states.mute_state.contains(target));
                    break
                }
                if val != None {return val}
                for vd in virt_devices {
                    if vd.description.id != *id {continue;}
                    val = Some(vd.mute_states.mute_state.contains(target));
                    break
                }
                val
            }
            BoolCommand::TargetMute { id } => {
                let phy_devices = &status.audio.profile.devices.targets.physical_devices;
                let virt_devices = &status.audio.profile.devices.targets.virtual_devices;
                let mut val: Option<bool> = None;
                for pd in phy_devices {
                    if pd.description.id != *id {continue;}
                    val = Some(pd.mute_state == MuteState::Muted);
                    break
                }
                if val != None {return val}
                for vd in virt_devices {
                    if vd.description.id != *id {continue;}
                    val = Some(vd.mute_state == MuteState::Muted);
                    break
                }
                val
            }
            BoolCommand::TargetMix { id } => {
                let phy_devices = &status.audio.profile.devices.targets.physical_devices;
                let virt_devices = &status.audio.profile.devices.targets.virtual_devices;
                let mut val: Option<bool> = None;
                for pd in phy_devices {
                    if pd.description.id != *id {continue;}
                    val = Some(pd.mix == Mix::A);
                    break
                }
                if val != None {return val}
                for vd in virt_devices {
                    if vd.description.id != *id {continue;}
                    val = Some(vd.mix == Mix::A);
                    break
                }
                val
            }
        }
    }
}
pub fn axis_controller(
    command: AxisCommand,
    behaviour: Arc<Mutex<dyn AxisBehaviour + Send + Sync>>,
    tx: Arc<Mutex<Sender<DaemonRequest>>>
) -> impl AxisProvider + Send + Sync {
    AxisController::new(behaviour, tx, move |data: u8| {command.to_request(data)})
}

pub fn bool_controller(
    command: BoolCommand,
    behaviour: Arc<Mutex<dyn BooleanBehaviour + Send + Sync>>,
    tx: Arc<Mutex<Sender<DaemonRequest>>>
) -> impl BooleanProvider + Send + Sync {
    BooleanController::new(behaviour, tx, move |data: bool| {command.to_request(data)})
}


pub struct PrinterController {
    pub name: String
}

impl CallbackProvider for PrinterController {
    fn callback(&self, data: u8) {
        println!("Callback for {} with data {}", self.name, data);
    }
}

impl AxisProvider for PrinterController {
    fn set(&self, data: u8) {
        println!("Axis set for {} with data {}", self.name, data);
    }
}

impl BooleanProvider for PrinterController {
    fn set(&self, data: bool) {
        println!("Bool set for {} with data {}", self.name, data);
    }
}