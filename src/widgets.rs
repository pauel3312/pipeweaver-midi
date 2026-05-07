use crate::behaviours::{AbsoluteAxis, AxisBehaviour, AxisBehaviourTrait, BinaryOffsetRelativeAxis, BooleanBehaviour, BooleanBehaviourTrait, CustomRelativeAxis, PushBtn, SignMagnitudeRelativeAxis, ToggleBtn, TwosComplimentRelativeAxis};
use crate::pipeweaver_main::SharedState;
use crate::pwv_controllers::BooleanProvider;
use crate::pwv_controllers::{AxisCommand, BoolCommand, axis_controller};
use crate::pwv_controllers::{AxisProvider, bool_controller};
use crate::widgets::AxisBehaviourState::{
    BinaryOffsetRelative, CustomAbsolute, CustomRelative, MidiAbsolute, SignMagnitudeRelative,
    TwosComplimentRelative,
};
use eframe::emath::Align;
use egui::{Color32, CornerRadius, DragValue, Frame, Label, Response, Stroke, Ui, Widget};
use midi_msg::{Channel, ChannelVoiceMsg, MidiMsg};
use pipeweaver_shared::{Mix, MuteTarget};
use std::collections::HashMap;
use std::ops::RangeInclusive;
use std::sync::{Arc, Mutex};
use ulid::Ulid;

#[derive(Debug, Copy, Clone)]
pub enum BoolBehaviourState {
    Toggle { threshold: u8, invert: bool },
    Push { threshold: u8, invert: bool },
}

#[derive(Debug, Copy, Clone)]
pub enum AxisBehaviourState {
    BinaryOffsetRelative {},
    MidiAbsolute {},
    TwosComplimentRelative {},
    SignMagnitudeRelative {},
    CustomRelative { threshold: u8, invert: bool },
    CustomAbsolute { min_in: u8, max_in: u8 },
}

impl BoolBehaviourState {
    fn make_behaviour(self) -> Arc<Mutex<BooleanBehaviour>> {
        let behaviour = match self {
            BoolBehaviourState::Toggle { threshold, invert } => {
                BooleanBehaviour::Toggle(ToggleBtn::new(threshold, Some(invert)))
            }
            BoolBehaviourState::Push { threshold, invert } => {
                BooleanBehaviour::Push(PushBtn::new(threshold, Some(invert)))
            }
        };
        Arc::new(Mutex::new(behaviour))
    }
}

impl AxisBehaviourState {
    fn make_behaviour(self) -> Arc<Mutex<AxisBehaviour>> {
        let behaviour = match self {
            BinaryOffsetRelative {} => {
                AxisBehaviour::BinaryOffsetRelative(BinaryOffsetRelativeAxis::new(None))
            }
            MidiAbsolute {} => AxisBehaviour::Absolute(AbsoluteAxis::new(0, 127, 0, 100)),
            TwosComplimentRelative {} => {
                AxisBehaviour::TwosComplimentRelative(TwosComplimentRelativeAxis::new(None))
            }
            SignMagnitudeRelative {} => {
                AxisBehaviour::SignMagnitudeRelative(SignMagnitudeRelativeAxis::new(None))
            }
            CustomRelative { threshold, invert } => AxisBehaviour::CustomRelative(
                CustomRelativeAxis::new(threshold, Some(invert), None),
            ),
            CustomAbsolute { min_in, max_in } => {
                AxisBehaviour::Absolute(AbsoluteAxis::new(min_in, max_in, 0, 100))
            }
        };
        Arc::new(Mutex::new(behaviour))
    }
}

pub struct ButtonWidget<'a> {
    pub shared_state: Arc<Mutex<SharedState>>,
    pub state: &'a mut BoolBehaviourState,
    pub cmd: BoolCommand,
    pub name: Option<&'a str>,
}

impl<'a> Widget for ButtonWidget<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let ButtonWidget {
            shared_state,
            state,
            cmd,
            name,
        } = self;

        ui.vertical(|ui| {
            ui.set_min_width(100f32);
            match name {
                Some(name) => {
                    ui.add(Label::new(name).halign(Align::Center));
                }
                None => {}
            }

            egui::ComboBox::from_id_salt(cmd)
                .selected_text(match state {
                    BoolBehaviourState::Toggle { .. } => "Toggle",
                    BoolBehaviourState::Push { .. } => "Push",
                })
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(
                            matches!(state, BoolBehaviourState::Toggle { .. }),
                            "Toggle",
                        )
                        .clicked()
                    {
                        if !matches!(state, BoolBehaviourState::Toggle { .. }) {
                            *state = BoolBehaviourState::Toggle {
                                threshold: 64,
                                invert: false,
                            };
                        }
                    }

                    if ui
                        .selectable_label(matches!(state, BoolBehaviourState::Push { .. }), "Push")
                        .clicked()
                    {
                        if !matches!(state, BoolBehaviourState::Push { .. }) {
                            *state = BoolBehaviourState::Push {
                                threshold: 64,
                                invert: false,
                            };
                        }
                    }
                });

            let (threshold, invert) = match state {
                BoolBehaviourState::Toggle { threshold, invert }
                | BoolBehaviourState::Push { threshold, invert } => (threshold, invert),
            };

            ui.horizontal(|ui| {
                ui.add(DragValue::new(threshold).range(0..=127));
                ui.label("threshold");
            });

            ui.checkbox(invert, "Invert");
            ui.horizontal(|ui| {
                let mut state_guard = shared_state.lock().unwrap();

                if ui.button("Delete").clicked() {
                    if let Some(msg) = state_guard.learn_msg.clone() {
                        state_guard.midi_tree.rm_callback(&msg);
                    }
                    state_guard.buttons.remove(&cmd);
                }

                if ui.button("Save").clicked() {
                    let tx = state_guard.tx.clone().unwrap();
                    let controller = bool_controller(cmd, state.make_behaviour(), tx);
                    match &state_guard.status {
                        None => {}
                        Some(status) => match cmd.get_value(status) {
                            None => {}
                            Some(d) => controller.set(d),
                        },
                    }

                    let controller = Arc::new(Mutex::new(controller));

                    let msg = state_guard
                        .learn_msg
                        .clone()
                        .unwrap_or(MidiMsg::ChannelVoice {
                            channel: Channel::Ch1,
                            msg: ChannelVoiceMsg::NoteOn {
                                note: 0,
                                velocity: 0,
                            },
                        });
                    println!("{:?}", msg);

                    state_guard
                        .midi_tree
                        .insert_callback(&msg, controller.clone())
                        .unwrap();
                    state_guard.buttons.insert(cmd, controller);
                }
            })
            .response
        })
        .response
    }
}

pub struct VolumeWidget<'a> {
    pub shared_state: Arc<Mutex<SharedState>>,
    pub state: &'a mut AxisBehaviourState,
    pub cmd: AxisCommand,
}

impl<'a> Widget for VolumeWidget<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let VolumeWidget {
            shared_state,
            state,
            cmd,
        } = self;
        ui.vertical(|ui| {
            ui.set_min_width(100f32);
            ui.label("Volume");
            egui::ComboBox::from_id_salt(cmd)
                .selected_text(match state {
                    BinaryOffsetRelative { .. } => "Binary Offset Relative",
                    MidiAbsolute { .. } => "Absolute",
                    TwosComplimentRelative { .. } => "Two's Compliment Relative",
                    SignMagnitudeRelative { .. } => "Sign Magnitude Relative",
                    CustomRelative { .. } => "Custom Relative",
                    CustomAbsolute { .. } => "Custom Absolute",
                })
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(
                            matches!(state, BinaryOffsetRelative { .. }),
                            "Binary Offset Relative",
                        )
                        .clicked()
                    {
                        if !matches! {state, BinaryOffsetRelative { .. }} {
                            *state = BinaryOffsetRelative {};
                        }
                    }
                    if ui
                        .selectable_label(matches!(state, MidiAbsolute { .. }), "Absolute")
                        .clicked()
                    {
                        if !matches!(state, MidiAbsolute { .. }) {
                            *state = MidiAbsolute {};
                        }
                    }
                    if ui
                        .selectable_label(
                            matches!(state, TwosComplimentRelative { .. }),
                            "Two's Compliment Relative",
                        )
                        .clicked()
                    {
                        if !matches!(state, TwosComplimentRelative { .. }) {
                            *state = TwosComplimentRelative {}
                        }
                    }
                    if ui
                        .selectable_label(
                            matches!(state, SignMagnitudeRelative { .. }),
                            "Sign Magnitude Relative",
                        )
                        .clicked()
                    {
                        if !matches!(state, SignMagnitudeRelative { .. }) {
                            *state = SignMagnitudeRelative {};
                        }
                    }
                    if ui
                        .selectable_label(matches!(state, CustomRelative { .. }), "Custom Relative")
                        .clicked()
                    {
                        if !matches!(state, CustomRelative { .. }) {
                            *state = CustomRelative {
                                invert: false,
                                threshold: 64,
                            }
                        }
                    }
                    if ui
                        .selectable_label(matches!(state, CustomAbsolute { .. }), "Custom Absolute")
                        .clicked()
                    {
                        if !matches!(state, CustomAbsolute { .. }) {
                            *state = CustomAbsolute {
                                min_in: 0,
                                max_in: 127,
                            }
                        }
                    }
                });

            match &mut *state {
                CustomRelative { threshold, invert } => {
                    ui.horizontal(|ui| {
                        ui.add(
                            DragValue::new(&mut *threshold).range(RangeInclusive::new(0u8, 127u8)),
                        );
                        ui.label("threshold");
                    });
                    ui.checkbox(&mut *invert, "Invert");
                }
                CustomAbsolute { min_in, max_in } => {
                    ui.horizontal(|ui| {
                        ui.add(DragValue::new(&mut *min_in).range(RangeInclusive::new(0u8, 127u8)));
                        ui.label("minimum input");
                    });
                    ui.horizontal(|ui| {
                        ui.add(DragValue::new(&mut *max_in).range(RangeInclusive::new(0u8, 127u8)));
                        ui.label("maximum input");
                    });
                }
                _ => {}
            }
            ui.horizontal(|ui| {
                let mut state_guard = shared_state.lock().unwrap();

                if ui.button("Delete").clicked() {
                    if let Some(msg) = state_guard.learn_msg.clone() {
                        state_guard.midi_tree.rm_callback(&msg);
                    }
                    state_guard.axes.remove(&self.cmd);
                }

                if ui.button("Save").clicked() {
                    let tx = state_guard.tx.clone().unwrap();
                    let controller = axis_controller(cmd, state.make_behaviour(), tx);
                    match &state_guard.status {
                        None => {}
                        Some(status) => match cmd.get_value(status) {
                            None => {}
                            Some(d) => controller.set(d),
                        },
                    }

                    // todo savestates are cmd + Behaviour + MidiMsg
                    let controller = Arc::new(Mutex::new(controller));

                    let msg = state_guard
                        .learn_msg
                        .clone()
                        .unwrap_or(MidiMsg::ChannelVoice {
                            channel: Channel::Ch1,
                            msg: ChannelVoiceMsg::NoteOn {
                                note: 0,
                                velocity: 0,
                            },
                        });
                    println!("{:?}", msg);

                    state_guard
                        .midi_tree
                        .insert_callback(&msg, controller.clone())
                        .unwrap();
                    state_guard.axes.insert(cmd, controller);
                }
            })
            .response
        })
        .inner
    }
}

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
                            let mut bool_state = bool_states.get(&cmd).cloned().unwrap_or(
                                BoolBehaviourState::Toggle {
                                    threshold: 64,
                                    invert: false,
                                },
                            );
                            let mut axis_state =
                                axis_states.get(&vol).cloned().unwrap_or(MidiAbsolute {});

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
                    let mut bool_state =
                        bool_states
                            .get(&mute_cmd)
                            .cloned()
                            .unwrap_or(BoolBehaviourState::Toggle {
                                threshold: 64,
                                invert: false,
                            });
                    let mut axis_state = axis_states
                        .get(&vol_cmd)
                        .cloned()
                        .unwrap_or(MidiAbsolute {});

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

                                let mut route_state = bool_states.get(&cmd).cloned().unwrap_or(
                                    BoolBehaviourState::Toggle {
                                        threshold: 64,
                                        invert: false,
                                    },
                                );

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
