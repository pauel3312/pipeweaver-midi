use crate::behaviours::{
    AbsoluteAxis, AxisBehaviour, BinaryOffsetRelativeAxis, CustomRelativeAxis,
    SignMagnitudeRelativeAxis, TwosComplimentRelativeAxis,
};
use crate::pipeweaver_main::SharedState;
use crate::pwv_controllers::AxisProvider;
use crate::pwv_controllers::{AxisCommand, BoolCommand, axis_controller};
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

impl AxisBehaviourState {
    fn make_behaviour(self) -> Arc<Mutex<dyn AxisBehaviour + Send + Sync>> {
        match self {
            BinaryOffsetRelative {} => Arc::new(Mutex::new(BinaryOffsetRelativeAxis::new(None))),
            MidiAbsolute {} => Arc::new(Mutex::new(AbsoluteAxis::new(0, 127, 0, 100))),
            TwosComplimentRelative {} => {
                Arc::new(Mutex::new(TwosComplimentRelativeAxis::new(None)))
            }
            SignMagnitudeRelative {} => Arc::new(Mutex::new(SignMagnitudeRelativeAxis::new(None))),
            CustomRelative { threshold, invert } => Arc::new(Mutex::new(CustomRelativeAxis::new(
                threshold,
                Some(invert),
                None,
            ))),
            CustomAbsolute { min_in, max_in } => {
                Arc::new(Mutex::new(AbsoluteAxis::new(min_in, max_in, 0, 100)))
            }
        }
    }
}

pub struct MuteWidget<'a> {
    pub state: &'a mut BoolBehaviourState,
    pub cmd: BoolCommand,
}

impl<'a> Widget for MuteWidget<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let MuteWidget { state, cmd } = self;

        ui.vertical(|ui| {
            ui.set_min_width(100f32);
            ui.add(Label::new("Mute").halign(Align::Center));

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
        })
        .response
    }
}

pub struct VolumeWidget<'a> {
    pub state: &'a mut AxisBehaviourState,
    pub cmd: AxisCommand,
}

impl<'a> Widget for VolumeWidget<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let VolumeWidget { state, cmd } = self;
        ui.vertical(|ui| {
            ui.set_min_width(100f32);
            ui.label("Volume");
            egui::ComboBox::from_id_salt(cmd)
                .selected_text(match state {
                    AxisBehaviourState::BinaryOffsetRelative { .. } => "Binary Offset Relative",
                    AxisBehaviourState::MidiAbsolute { .. } => "Absolute",
                    AxisBehaviourState::TwosComplimentRelative { .. } => {
                        "Two's Compliment Relative"
                    }
                    AxisBehaviourState::SignMagnitudeRelative { .. } => "Sign Magnitude Relative",
                    AxisBehaviourState::CustomRelative { .. } => "Custom Relative",
                    AxisBehaviourState::CustomAbsolute { .. } => "Custom Absolute",
                })
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(
                            matches!(state, AxisBehaviourState::BinaryOffsetRelative { .. }),
                            "Binary Offset Relative",
                        )
                        .clicked()
                    {
                        if !matches! {state, AxisBehaviourState::BinaryOffsetRelative { .. }} {
                            *state = AxisBehaviourState::BinaryOffsetRelative {};
                        }
                    }
                    if ui
                        .selectable_label(
                            matches!(state, AxisBehaviourState::MidiAbsolute { .. }),
                            "Absolute",
                        )
                        .clicked()
                    {
                        if !matches!(state, AxisBehaviourState::MidiAbsolute { .. }) {
                            *state = AxisBehaviourState::MidiAbsolute {};
                        }
                    }
                    if ui
                        .selectable_label(
                            matches!(state, AxisBehaviourState::TwosComplimentRelative { .. }),
                            "Two's Compliment Relative",
                        )
                        .clicked()
                    {
                        if !matches!(state, AxisBehaviourState::TwosComplimentRelative { .. }) {
                            *state = AxisBehaviourState::TwosComplimentRelative {}
                        }
                    }
                    if ui
                        .selectable_label(
                            matches!(state, AxisBehaviourState::SignMagnitudeRelative { .. }),
                            "Sign Magnitude Relative",
                        )
                        .clicked()
                    {
                        if !matches!(state, AxisBehaviourState::SignMagnitudeRelative { .. }) {
                            *state = AxisBehaviourState::SignMagnitudeRelative {};
                        }
                    }
                    if ui
                        .selectable_label(
                            matches!(state, AxisBehaviourState::CustomRelative { .. }),
                            "Custom Relative",
                        )
                        .clicked()
                    {
                        if !matches!(state, AxisBehaviourState::CustomRelative { .. }) {
                            *state = AxisBehaviourState::CustomRelative {
                                invert: false,
                                threshold: 64,
                            }
                        }
                    }
                    if ui
                        .selectable_label(
                            matches!(state, AxisBehaviourState::CustomAbsolute { .. }),
                            "Custom Absolute",
                        )
                        .clicked()
                    {
                        if !matches!(state, AxisBehaviourState::CustomAbsolute { .. }) {
                            *state = AxisBehaviourState::CustomAbsolute {
                                min_in: 0,
                                max_in: 127,
                            }
                        }
                    }
                });

            match &mut *state {
                AxisBehaviourState::CustomRelative { threshold, invert } => {
                    ui.horizontal(|ui| {
                        ui.add(
                            DragValue::new(&mut *threshold).range(RangeInclusive::new(0u8, 127u8)),
                        );
                        ui.label("threshold");
                    });
                    ui.checkbox(&mut *invert, "Invert");
                }
                AxisBehaviourState::CustomAbsolute { min_in, max_in } => {
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut *min_in)
                                .range(RangeInclusive::new(0u8, 127u8)),
                        );
                        ui.label("minimum input");
                    });
                    ui.horizontal(|ui| {
                        ui.add(DragValue::new(&mut *max_in).range(RangeInclusive::new(0u8, 127u8)));
                        ui.label("maximum input");
                    });
                }
                _ => {}
            }
        })
        .response
    }
}

pub struct SourceDeviceWidget<'a> {
    pub bool_states: &'a mut HashMap<BoolCommand, BoolBehaviourState>,
    pub axis_states: &'a mut HashMap<AxisCommand, AxisBehaviourState>,
    pub id: Ulid,
    pub name: String,
}

impl<'a> Widget for SourceDeviceWidget<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let SourceDeviceWidget {
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
                            let mut axis_state = axis_states
                                .get(&vol)
                                .cloned()
                                .unwrap_or(AxisBehaviourState::MidiAbsolute {});
                            // TODO save btns

                            ui.vertical(|ui| {
                                ui.add(VolumeWidget {
                                    state: &mut axis_state,
                                    cmd: vol,
                                });
                                ui.add(MuteWidget {
                                    state: &mut bool_state,
                                    cmd,
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
                        .unwrap_or(AxisBehaviourState::MidiAbsolute {});

                    ui.vertical(|ui| {
                        ui.add(VolumeWidget {
                            state: &mut axis_state,
                            cmd: vol_cmd,
                        });
                        // TODO move save btns to Volume and Mute widgets
                        ui.horizontal(|ui| {
                            let mut state_guard = state.lock().unwrap();

                            if ui.button("Delete").clicked() {
                                if let Some(msg) = state_guard.learn_msg.clone() {
                                    state_guard.midi_tree.rm_callback(&msg);
                                }
                                state_guard.axes.remove(&vol_cmd);
                            }

                            if ui.button("Save").clicked() {
                                let tx = state_guard.tx.clone().unwrap();
                                let controller =
                                    axis_controller(vol_cmd, axis_state.make_behaviour(), tx);
                                match &state_guard.status {
                                    None => {}
                                    Some(status) => match vol_cmd.get_value(status) {
                                        None => {}
                                        Some(d) => controller.set(d),
                                    },
                                }

                                let controller = Arc::new(Mutex::new(controller));

                                let msg = state_guard.learn_msg.clone().unwrap_or(
                                    MidiMsg::ChannelVoice {
                                        channel: Channel::Ch1,
                                        msg: ChannelVoiceMsg::NoteOn {
                                            note: 0,
                                            velocity: 0,
                                        },
                                    },
                                );
                                println!("{:?}", msg);

                                state_guard
                                    .midi_tree
                                    .insert_callback(&msg, controller.clone())
                                    .unwrap();
                                state_guard.axes.insert(vol_cmd, controller);
                            }
                        });

                        ui.add(MuteWidget {
                            state: &mut bool_state,
                            cmd: mute_cmd,
                        })
                    });

                    axis_states.insert(vol_cmd, axis_state.clone());
                    bool_states.insert(mute_cmd, bool_state.clone());
                })
                .response
            })
            .inner
    }
}
