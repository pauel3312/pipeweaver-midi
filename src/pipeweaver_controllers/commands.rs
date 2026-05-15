use pipeweaver_ipc::commands::{APICommand, DaemonRequest, DaemonStatus};
use pipeweaver_shared::{Mix, MuteState, MuteTarget};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

#[derive(Eq, Clone, Hash, PartialEq, Copy, Debug, Serialize, Deserialize)]
pub(crate) enum AxisCommand {
    SourceVolume { id: Ulid, mix: Mix },
    TargetVolume { id: Ulid },
}

#[derive(Eq, Hash, PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub(crate) enum BoolCommand {
    Route { in_id: Ulid, out_id: Ulid },
    SourceMute { id: Ulid, target: MuteTarget },
    TargetMute { id: Ulid },
    TargetMix { id: Ulid },
}

impl AxisCommand {
    pub(crate) fn to_request(&self, data: u8) -> DaemonRequest {
        match self {
            AxisCommand::SourceVolume { id, mix } => {
                DaemonRequest::Pipewire(APICommand::SetSourceVolume(*id, *mix, data))
            }

            AxisCommand::TargetVolume { id } => DaemonRequest::Pipewire(APICommand::SetTargetVolume(*id, data)),
        }
    }

    pub fn get_value(&self, status: &DaemonStatus) -> Option<u8> {
        let mut val: Option<u8> = None;
        match *self {
            AxisCommand::SourceVolume { id, mix } => {
                let phy_devices = &status.audio.profile.devices.sources.physical_devices;
                let virt_devices = &status.audio.profile.devices.sources.virtual_devices;
                for pd in phy_devices {
                    if pd.description.id != id {
                        continue;
                    }
                    val = Some(pd.volumes.volume[mix]);
                    break;
                }
                if val != None {
                    return val;
                }
                for vd in virt_devices {
                    if vd.description.id != id {
                        continue;
                    }
                    val = Some(vd.volumes.volume[mix]);
                    break;
                }
            }
            AxisCommand::TargetVolume { id } => {
                let phy_devices = &status.audio.profile.devices.targets.physical_devices;
                let virt_devices = &status.audio.profile.devices.targets.virtual_devices;
                for pd in phy_devices {
                    if pd.description.id != id {
                        continue;
                    }
                    val = Some(pd.volume);
                    break;
                }
                if val != None {
                    return val;
                }
                for vd in virt_devices {
                    if vd.description.id != id {
                        continue;
                    }
                    val = Some(vd.volume);
                    break;
                }
            }
        }
        val
    }
}

impl BoolCommand {
    pub(crate) fn to_request(&self, data: bool) -> DaemonRequest {
        match self {
            BoolCommand::Route { in_id, out_id } => {
                DaemonRequest::Pipewire(APICommand::SetRoute(*in_id, *out_id, data))
            }
            BoolCommand::SourceMute { id, target } => {
                let command: APICommand;
                if data {
                    command = APICommand::AddSourceMuteTarget(*id, *target);
                } else {
                    command = APICommand::DelSourceMuteTarget(*id, *target);
                }
                DaemonRequest::Pipewire(command)
            }
            BoolCommand::TargetMute { id } => {
                let state: MuteState = if data { MuteState::Muted } else { MuteState::Unmuted };
                DaemonRequest::Pipewire(APICommand::SetTargetMuteState(*id, state))
            }
            BoolCommand::TargetMix { id } => {
                let mix = if data { Mix::A } else { Mix::B };
                DaemonRequest::Pipewire(APICommand::SetTargetMix(*id, mix))
            }
        }
    }
    pub fn get_value(&self, status: &DaemonStatus) -> Option<bool> {
        match self {
            BoolCommand::Route { in_id, out_id } => Some(status.audio.profile.routes[in_id].contains(out_id)),
            BoolCommand::SourceMute { id, target } => {
                let phy_devices = &status.audio.profile.devices.sources.physical_devices;
                let virt_devices = &status.audio.profile.devices.sources.virtual_devices;
                let mut val: Option<bool> = None;
                for pd in phy_devices {
                    if pd.description.id != *id {
                        continue;
                    }
                    val = Some(pd.mute_states.mute_state.contains(target));
                    break;
                }
                if val != None {
                    return val;
                }
                for vd in virt_devices {
                    if vd.description.id != *id {
                        continue;
                    }
                    val = Some(vd.mute_states.mute_state.contains(target));
                    break;
                }
                val
            }
            BoolCommand::TargetMute { id } => {
                let phy_devices = &status.audio.profile.devices.targets.physical_devices;
                let virt_devices = &status.audio.profile.devices.targets.virtual_devices;
                let mut val: Option<bool> = None;
                for pd in phy_devices {
                    if pd.description.id != *id {
                        continue;
                    }
                    val = Some(pd.mute_state == MuteState::Muted);
                    break;
                }
                if val != None {
                    return val;
                }
                for vd in virt_devices {
                    if vd.description.id != *id {
                        continue;
                    }
                    val = Some(vd.mute_state == MuteState::Muted);
                    break;
                }
                val
            }
            BoolCommand::TargetMix { id } => {
                let phy_devices = &status.audio.profile.devices.targets.physical_devices;
                let virt_devices = &status.audio.profile.devices.targets.virtual_devices;
                let mut val: Option<bool> = None;
                for pd in phy_devices {
                    if pd.description.id != *id {
                        continue;
                    }
                    val = Some(pd.mix == Mix::A);
                    break;
                }
                if val != None {
                    return val;
                }
                for vd in virt_devices {
                    if vd.description.id != *id {
                        continue;
                    }
                    val = Some(vd.mix == Mix::A);
                    break;
                }
                val
            }
        }
    }
}
