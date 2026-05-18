use crate::common::SharedState;
use crate::config::ConfigState;
use crate::midi::manager::MidiMgr;
use eframe::epaint::{Color32, CornerRadius, Stroke};
use egui::{Button, ComboBox, DragValue, Frame, Response, Ui, Widget};
use midi_msg::ControlChange::CC;
use midi_msg::{Channel, ChannelVoiceMsg, MidiMsg};
use midir::MidiInput;
use std::sync::{Arc, Mutex};

pub(super) struct MidiDeviceWidget<'a> {
    pub(super) state: Arc<Mutex<SharedState>>,
    pub(super) midi_dsc: &'a MidiInput,
    pub(super) midi_mgr: Arc<Mutex<MidiMgr>>,
}

impl<'a> Widget for MidiDeviceWidget<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let mut conn_ok = true;


        let inner_response = Frame::default()
            .inner_margin(4)
            .outer_margin(5)
            .stroke(Stroke::new(3.0, Color32::DARK_GRAY))
            .corner_radius(CornerRadius::same(10))
            .show(ui, |ui| {
                let mut state = self.state.lock().unwrap();
                let before_port = state.current_port.clone();
                let mut text = String::from("Port no longer valid!");
                match self.midi_dsc.port_name(&state.current_port) {
                    Ok(txt) => {
                        text = txt;
                    }
                    Err(_) => {
                        conn_ok = false;
                    }
                }

                ui.vertical(|ui| {
                    ComboBox::from_label("MIDI Device")
                        .selected_text(text)
                        .show_ui(ui, |ui| {
                            let ports = self.midi_dsc.ports();
                            let current_port = &mut state.current_port;

                            for port in ports {
                                ui.selectable_value(
                                    current_port,
                                    port.clone(),
                                    self.midi_dsc.port_name(&port).unwrap(),
                                );
                            }
                        });
                    let after_port = state.current_port.clone();
                    state.config.midi_device = self.midi_dsc.port_name(&after_port).unwrap();

                    let mut current_learn_mode: bool = state.learn_mode;
                    ComboBox::from_label("MIDI event source")
                        .selected_text(if state.learn_mode { "Learn" } else { "Custom" })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut current_learn_mode, true, "Learn");
                            ui.selectable_value(&mut current_learn_mode, false, "Custom")
                        });
                    state.learn_mode = current_learn_mode;

                    let (mut sel_channel, mut is_cc, mut sel_value) = match state.clone().learn_msg {
                        None => (Channel::Ch1, true, 0u8),
                        Some(msg) => match msg {
                            MidiMsg::ChannelVoice { channel, msg } => {
                                let (is_cc, value) = match msg {
                                    ChannelVoiceMsg::ControlChange { control } => {
                                        let value = match control {
                                            CC { control, value: _ } => control,
                                            _ => 0u8,
                                        };
                                        (true, value)
                                    }
                                    ChannelVoiceMsg::NoteOff { note, velocity: _ }
                                    | ChannelVoiceMsg::NoteOn { note, velocity: _ } => (false, note),
                                    _ => (false, 0),
                                };
                                (channel, is_cc, value)
                            }
                            _ => (Channel::Ch1, true, 0u8),
                        },
                    };

                    if state.learn_mode {
                        ui.label(format!("Channel: {}", sel_channel));
                        if is_cc {
                            ui.label(format!("CC: {}", sel_value));
                        } else {
                            ui.label(format!("Note: {}", sel_value));
                        }
                        if ui
                            .add(Button::new(if state.learning { "Learning..." } else { "Learn" }))
                            .clicked()
                        {
                            state.learning = true;
                        }
                    } else {
                        ComboBox::from_label("Channel")
                            .selected_text(sel_channel.to_string())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut sel_channel, Channel::Ch1, "Ch1");
                                ui.selectable_value(&mut sel_channel, Channel::Ch2, "Ch2");
                                ui.selectable_value(&mut sel_channel, Channel::Ch3, "Ch3");
                                ui.selectable_value(&mut sel_channel, Channel::Ch4, "Ch4");
                                ui.selectable_value(&mut sel_channel, Channel::Ch5, "Ch5");
                                ui.selectable_value(&mut sel_channel, Channel::Ch6, "Ch6");
                                ui.selectable_value(&mut sel_channel, Channel::Ch7, "Ch7");
                                ui.selectable_value(&mut sel_channel, Channel::Ch8, "Ch8");
                                ui.selectable_value(&mut sel_channel, Channel::Ch9, "Ch9");
                                ui.selectable_value(&mut sel_channel, Channel::Ch10, "Ch10");
                                ui.selectable_value(&mut sel_channel, Channel::Ch11, "Ch11");
                                ui.selectable_value(&mut sel_channel, Channel::Ch12, "Ch12");
                                ui.selectable_value(&mut sel_channel, Channel::Ch13, "Ch13");
                                ui.selectable_value(&mut sel_channel, Channel::Ch14, "Ch14");
                                ui.selectable_value(&mut sel_channel, Channel::Ch15, "Ch15");
                                ui.selectable_value(&mut sel_channel, Channel::Ch16, "Ch16")
                            });
                        ComboBox::from_label("Type")
                            .selected_text(if is_cc { "CC" } else { "Note" })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut is_cc, true, "CC");
                                ui.selectable_value(&mut is_cc, false, "Note");
                            });

                        ui.horizontal(|ui| {
                            ui.add(DragValue::new(&mut sel_value));
                            if is_cc {
                                ui.label("CC number");
                            } else {
                                ui.label("Note number");
                            }
                        });
                        state.learn_msg = Some(MidiMsg::ChannelVoice {
                            channel: sel_channel,
                            msg: if is_cc {
                                ChannelVoiceMsg::ControlChange {
                                    control: CC {
                                        control: sel_value,
                                        value: 0,
                                    },
                                }
                            } else {
                                ChannelVoiceMsg::NoteOn {
                                    note: sel_value,
                                    velocity: 0,
                                }
                            },
                        });
                    }
                    (before_port, after_port)
                })
                .inner
            });

        let (before_port, after_port) = inner_response.inner;

        let mut midi_mgr = self.midi_mgr.lock().unwrap();
        if before_port != after_port {
            midi_mgr.disconnect();
            midi_mgr.connect(self.state.clone()).unwrap();
        } else if !conn_ok {
            midi_mgr.disconnect();
        } else if conn_ok && midi_mgr.conn.is_none() {
            midi_mgr.connect(self.state.clone()).unwrap();
        }

        inner_response.response
    }
}

pub(super) struct ConfigWidget<'a> {
    state: &'a mut ConfigState,
}

impl<'a> ConfigWidget<'a> {
    pub(super) fn new(state: &'a mut ConfigState) -> Self {
        Self { state }
    }
}

impl<'a> Widget for ConfigWidget<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        Frame::default()
            .inner_margin(4)
            .outer_margin(5)
            .stroke(Stroke::new(3.0, Color32::DARK_GRAY))
            .corner_radius(CornerRadius::same(10))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.label("config");

                    ui.checkbox(&mut self.state.auto_save, "Auto-save");

                    ui.label(self.state.path.lock().unwrap().as_path().display().to_string());

                    let tx = self.state.file_dialog_tx.clone();
                    let current_dir = self
                        .state
                        .path.lock().unwrap()
                        .parent()
                        .unwrap_or_else(|| std::path::Path::new("/"))
                        .to_path_buf();


                    if ui.button("Pick File").clicked() {
                        std::thread::spawn(move || {
                            let file = futures::executor::block_on(
                                rfd::AsyncFileDialog::new()
                                    .add_filter("config files", &["json"])
                                    .set_directory(&current_dir)
                                    .pick_file(),
                            );

                            let path = file.map(|f| f.path().to_path_buf());
                            if tx.is_some() {
                                let _ = tx.unwrap().send(path);
                            }
                        });
                    }

                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            self.state.save()
                        }
                        if ui.button("Load").clicked() {
                            self.state.load()
                        }
                    })
                    .response
                })
                .inner
            })
            .inner
    }
}
