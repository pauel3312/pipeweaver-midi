use crate::common::SharedState;
use crate::pipeweaver_controllers::axis::axis_controller;
use crate::pipeweaver_controllers::bool::bool_controller;
use crate::pipeweaver_controllers::commands::{AxisCommand, BoolCommand};
use crate::pipeweaver_controllers::core::{AxisProvider, BooleanProvider};
use crate::ui::behaviour_selectors::AxisBehaviourState::{
    BinaryOffsetRelative, CustomAbsolute, CustomRelative, MidiAbsolute, SignMagnitudeRelative, TwosComplimentRelative,
};
use crate::ui::behaviour_selectors::{AxisBehaviourState, BoolBehaviourState};
use eframe::emath::Align;
use egui::{DragValue, Label, Response, Ui, Widget};
use midi_msg::{Channel, ChannelVoiceMsg, MidiMsg};
use std::ops::RangeInclusive;
use std::sync::{Arc, Mutex};

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
                        .selectable_label(matches!(state, BoolBehaviourState::Toggle { .. }), "Toggle")
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
                BoolBehaviourState::Toggle { threshold, invert } | BoolBehaviourState::Push { threshold, invert } => {
                    (threshold, invert)
                }
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
                    state_guard.config.rm_btn(cmd);
                }

                if ui.button("Save").clicked() {
                    let tx = state_guard.tx.clone().unwrap();
                    let behaviour = state.make_behaviour();

                    let msg = state_guard.learn_msg.clone().unwrap_or(MidiMsg::ChannelVoice {
                        channel: Channel::Ch1,
                        msg: ChannelVoiceMsg::NoteOn { note: 0, velocity: 0 },
                    });
                    state_guard.config.insert_btn(cmd, behaviour, msg.clone());

                    let behaviour = Arc::new(Mutex::new(behaviour));

                    let controller = bool_controller(cmd, behaviour, tx);
                    match &state_guard.status {
                        None => {}
                        Some(status) => match cmd.get_value(status) {
                            None => {}
                            Some(d) => controller.set(d),
                        },
                    }

                    let controller = Arc::new(Mutex::new(controller));

                    state_guard.midi_tree.insert_callback(&msg, controller.clone()).unwrap();
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
                        .selectable_label(matches!(state, BinaryOffsetRelative { .. }), "Binary Offset Relative")
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
                        .selectable_label(matches!(state, SignMagnitudeRelative { .. }), "Sign Magnitude Relative")
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
                            *state = CustomAbsolute { min_in: 0, max_in: 127 }
                        }
                    }
                });

            match &mut *state {
                CustomRelative { threshold, invert } => {
                    ui.horizontal(|ui| {
                        ui.add(DragValue::new(&mut *threshold).range(RangeInclusive::new(0u8, 127u8)));
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
                    state_guard.config.rm_axis(self.cmd);
                }

                if ui.button("Save").clicked() {
                    let tx = state_guard.tx.clone().unwrap();
                    let behaviour = state.make_behaviour();

                    let msg = state_guard.learn_msg.clone().unwrap_or(MidiMsg::ChannelVoice {
                        channel: Channel::Ch1,
                        msg: ChannelVoiceMsg::NoteOn { note: 0, velocity: 0 },
                    });

                    state_guard.config.insert_axis(cmd, behaviour, msg.clone());

                    let behaviour = Arc::new(Mutex::new(behaviour));

                    let controller = axis_controller(cmd, behaviour, tx);
                    match &state_guard.status {
                        None => {}
                        Some(status) => match cmd.get_value(status) {
                            None => {}
                            Some(d) => controller.set(d),
                        },
                    }

                    let controller = Arc::new(Mutex::new(controller));

                    state_guard.midi_tree.insert_callback(&msg, controller.clone()).unwrap();
                    state_guard.axes.insert(cmd, controller);
                }
            })
            .response
        })
        .inner
    }
}
