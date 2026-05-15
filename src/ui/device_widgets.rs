use crate::common::SharedState;
use crate::pipeweaver_controllers::commands::{AxisCommand, BoolCommand};
use crate::ui::behaviour_selectors::AxisBehaviourState::MidiAbsolute;
use crate::ui::behaviour_selectors::{AxisBehaviourState, BoolBehaviourState};
use crate::ui::components::{ButtonWidget, VolumeWidget};
use egui::{Color32, CornerRadius, Frame, Response, Stroke, Ui, Widget};
use pipeweaver_shared::{Mix, MuteTarget};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use ulid::Ulid;

pub struct SourceDeviceWidget<'a> {
    pub state: Arc<Mutex<SharedState>>,
    pub bool_states: &'a mut HashMap<BoolCommand, BoolBehaviourState>,
    pub axis_states: &'a mut HashMap<AxisCommand, AxisBehaviourState>,
    pub id: Ulid,
    pub name: String,
}

impl<'a> Widget for SourceDeviceWidget<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let SourceDeviceWidget {
            state,
            bool_states,
            axis_states,
            id,
            name,
        } = self;

        let mute_a_cmd = BoolCommand::SourceMute {
            id,
            target: MuteTarget::TargetA,
        };
        let mute_b_cmd = BoolCommand::SourceMute {
            id,
            target: MuteTarget::TargetB,
        };
        let vol_a_cmd = AxisCommand::SourceVolume { id, mix: Mix::A };
        let vol_b_cmd = AxisCommand::SourceVolume { id, mix: Mix::B };
        Frame::default()
            .inner_margin(4)
            .outer_margin(5)
            .stroke(Stroke::new(3.0, Color32::DARK_GRAY))
            .corner_radius(CornerRadius::same(10))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.set_min_width(100f32);
                    ui.label(name);
                    ui.horizontal(|ui| {
                        for (cmd, vol) in [(mute_a_cmd, vol_a_cmd), (mute_b_cmd, vol_b_cmd)] {
                            let mut bool_state = bool_states.get(&cmd).cloned().unwrap_or(BoolBehaviourState::Toggle {
                                threshold: 64,
                                invert: false,
                            });
                            let mut axis_state = axis_states.get(&vol).cloned().unwrap_or(MidiAbsolute {});

                            ui.vertical(|ui| {
                                ui.add(VolumeWidget {
                                    shared_state: state.clone(),
                                    state: &mut axis_state,
                                    cmd: vol,
                                });
                                ui.add(ButtonWidget {
                                    shared_state: state.clone(),
                                    state: &mut bool_state,
                                    cmd,
                                    name: Some("Mute"),
                                })
                            });

                            axis_states.insert(vol, axis_state.clone());
                            bool_states.insert(cmd, bool_state.clone());
                        }
                    })
                    .response
                })
                .inner
            })
            .inner
    }
}

pub struct TargetDeviceWidget<'a> {
    pub state: Arc<Mutex<SharedState>>,
    pub bool_states: &'a mut HashMap<BoolCommand, BoolBehaviourState>,
    pub axis_states: &'a mut HashMap<AxisCommand, AxisBehaviourState>,
    pub id: Ulid,
    pub name: String,
}
impl<'a> Widget for TargetDeviceWidget<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let TargetDeviceWidget {
            state,
            bool_states,
            axis_states,
            id,
            name,
        } = self;

        let mute_cmd = BoolCommand::TargetMute { id };
        let vol_cmd = AxisCommand::TargetVolume { id };
        Frame::default()
            .inner_margin(4)
            .outer_margin(5)
            .stroke(Stroke::new(3.0, Color32::DARK_GRAY))
            .corner_radius(CornerRadius::same(10))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.set_min_width(100f32);
                    ui.label(format!("{}", name));
                    let mut bool_state = bool_states
                        .get(&mute_cmd)
                        .cloned()
                        .unwrap_or(BoolBehaviourState::Toggle {
                            threshold: 64,
                            invert: false,
                        });
                    let mut axis_state = axis_states.get(&vol_cmd).cloned().unwrap_or(MidiAbsolute {});

                    ui.add(VolumeWidget {
                        shared_state: state.clone(),
                        state: &mut axis_state,
                        cmd: vol_cmd,
                    });
                    ui.add(ButtonWidget {
                        shared_state: state.clone(),
                        state: &mut bool_state,
                        cmd: mute_cmd,
                        name: Some("Mute"),
                    });

                    axis_states.insert(vol_cmd, axis_state.clone());
                    bool_states.insert(mute_cmd, bool_state.clone());
                })
                .response
            })
            .inner
    }
}

pub struct RoutingTableWidget<'a> {
    pub state: Arc<Mutex<SharedState>>,
    pub bool_states: &'a mut HashMap<BoolCommand, BoolBehaviourState>,
}

impl<'a> Widget for RoutingTableWidget<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let RoutingTableWidget { state, bool_states } = self;
        let status = state.lock().unwrap().status.clone().unwrap();
        let mut targets: Vec<(Ulid, String)> = Vec::new();
        for ptd in status.audio.profile.devices.targets.physical_devices {
            targets.push((ptd.description.id, ptd.description.name));
        }
        for vtd in status.audio.profile.devices.targets.virtual_devices {
            targets.push((vtd.description.id, vtd.description.name));
        }
        let mut sources: HashMap<Ulid, String> = HashMap::new();
        for psd in status.audio.profile.devices.sources.physical_devices {
            sources.insert(psd.description.id, psd.description.name);
        }
        for vsd in status.audio.profile.devices.sources.virtual_devices {
            sources.insert(vsd.description.id, vsd.description.name);
        }

        Frame::default()
            .inner_margin(8)
            .outer_margin(5)
            .stroke(Stroke::new(3.0, Color32::DARK_GRAY))
            .corner_radius(CornerRadius::same(10))
            .show(ui, |ui| {
                egui::Grid::new("routing_grid")
                    .striped(true)
                    .spacing([12.0, 10.0])
                    .min_col_width(100.0)
                    .show(ui, |ui| {
                        ui.label("");

                        for route in &status.audio.profile.routes {
                            ui.label(format!("{}", sources[&route.0]));
                        }

                        ui.end_row();

                        for tgt in targets.iter().cloned() {
                            ui.label(format!("{}", tgt.1));

                            for route in &status.audio.profile.routes {
                                let cmd = BoolCommand::Route {
                                    in_id: *route.0,
                                    out_id: tgt.0,
                                };

                                let mut route_state =
                                    bool_states.get(&cmd).cloned().unwrap_or(BoolBehaviourState::Toggle {
                                        threshold: 64,
                                        invert: false,
                                    });

                                ui.centered_and_justified(|ui| {
                                    ui.add(ButtonWidget {
                                        shared_state: state.clone(),
                                        state: &mut route_state,
                                        cmd,
                                        name: None,
                                    });
                                });
                                bool_states.insert(cmd, route_state.clone());
                            }

                            ui.end_row();
                        }
                    });
            })
            .response
    }
}
