use crate::common::SharedState;
use crate::midi::manager::MidiMgr;
use crate::pipeweaver_controllers::commands::{AxisCommand, BoolCommand};
use crate::tray::TrayState;
use crate::ui::behaviour_selectors::{AxisBehaviourState, BoolBehaviourState};
use crate::ui::device_widgets::{RoutingTableWidget, SourceDeviceWidget, TargetDeviceWidget};
use eframe::epaint::{Color32, CornerRadius, Stroke};
use egui::{Button, CentralPanel, ComboBox, DragValue, Frame, ViewportCommand};
use midi_msg::ControlChange::CC;
use midi_msg::{Channel, ChannelVoiceMsg, MidiMsg};
use midir::MidiInput;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::time::sleep;

pub(crate) async fn run(
    state: Arc<Mutex<SharedState>>,
    midi_mgr: Arc<Mutex<MidiMgr>>,
    stop: Arc<AtomicBool>,
    tray: Arc<Mutex<TrayState>>
) -> anyhow::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default(),
        ..Default::default()
    };

    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        let ui_on = tray.lock().unwrap().ui_on.clone();

        if ui_on.load(Ordering::Relaxed) {
            let state = state.clone();
            let midi_mgr = midi_mgr.clone();
            let stop = stop.clone();
            let ui_on = ui_on.clone();
            let tray = tray.clone();
            eframe::run_native(
                "Pipeweaver-MIDI",
                options.clone(),
                Box::new(move |cc| {
                     tray.lock().unwrap().ctx = Some(cc.egui_ctx.clone());
                    Ok(Box::new(PwvMidiGUI::new(state, midi_mgr, stop, ui_on)))
                }),
            )?;
        } else {
            sleep(std::time::Duration::from_millis(100)).await;
        }
    }
    Ok(())
}

pub(crate) struct PwvMidiGUI {
    stop: Arc<AtomicBool>,
    ui_on: Arc<AtomicBool>,
    state: Arc<Mutex<SharedState>>,
    midi_dsc: MidiInput,
    midi_mgr: Arc<Mutex<MidiMgr>>,
    bool_states: HashMap<BoolCommand, BoolBehaviourState>,
    axis_states: HashMap<AxisCommand, AxisBehaviourState>,
}

impl PwvMidiGUI {
    fn new(
        state: Arc<Mutex<SharedState>>,
        midi_mgr: Arc<Mutex<MidiMgr>>,
        stop: Arc<AtomicBool>,
        ui_on: Arc<AtomicBool>,
    ) -> Self {
        Self {
            stop,
            ui_on,
            state,
            midi_dsc: MidiInput::new("pipeweaver-midi-discover").unwrap(),
            midi_mgr,
            bool_states: HashMap::new(),
            axis_states: HashMap::new(),
        }
    }
}

impl eframe::App for PwvMidiGUI {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx();
        {
            if self.stop.load(Ordering::Relaxed) || (!self.ui_on.load(Ordering::Relaxed)) {
                ctx.send_viewport_cmd(ViewportCommand::Close)
            }
        }

        if ctx.input(|i| i.viewport().close_requested()) {
            self.ui_on.store(false, Ordering::Relaxed);
        }

        CentralPanel::default().show_inside(ui, |ui| {
            let mut conn_ok = true;
            let (before_port, after_port) = {
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

                Frame::default()
                    .inner_margin(4)
                    .outer_margin(5)
                    .stroke(Stroke::new(3.0, Color32::DARK_GRAY))
                    .corner_radius(CornerRadius::same(10))
                    .show(ui, |ui| {
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
                    })
                    .inner
            };

            let status = self.state.lock().unwrap().status.clone();
            match status {
                None => {
                    ui.label("Pipeweaver not connected!");
                }
                Some(status) => {
                    let profile = &status.audio.profile;
                    egui::scroll_area::ScrollArea::horizontal().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing();
                            for psd in &profile.devices.sources.physical_devices {
                                let id = psd.description.id;
                                let name = psd.description.name.clone();
                                ui.add(SourceDeviceWidget {
                                    state: self.state.clone(),
                                    bool_states: &mut self.bool_states,
                                    axis_states: &mut self.axis_states,
                                    id,
                                    name,
                                });
                            }
                            ui.end_row();
                            ui.spacing();
                            for vsd in &profile.devices.sources.virtual_devices {
                                let id = vsd.description.id;
                                let name = vsd.description.name.clone();
                                ui.add(SourceDeviceWidget {
                                    state: self.state.clone(),
                                    bool_states: &mut self.bool_states,
                                    axis_states: &mut self.axis_states,
                                    id,
                                    name,
                                });
                            }
                            ui.end_row();
                            ui.spacing();
                            for ptd in &profile.devices.targets.physical_devices {
                                let id = ptd.description.id;
                                let name = ptd.description.name.clone();
                                ui.add(TargetDeviceWidget {
                                    state: self.state.clone(),
                                    bool_states: &mut self.bool_states,
                                    axis_states: &mut self.axis_states,
                                    id,
                                    name,
                                });
                            }
                            ui.end_row();
                            ui.spacing();
                            for vtd in &profile.devices.targets.virtual_devices {
                                let id = vtd.description.id;
                                let name = vtd.description.name.clone();
                                ui.add(TargetDeviceWidget {
                                    state: self.state.clone(),
                                    bool_states: &mut self.bool_states,
                                    axis_states: &mut self.axis_states,
                                    id,
                                    name,
                                });
                            }
                            ui.end_row();
                        });

                        ui.spacing();
                        ui.vertical(|ui| {
                            ui.add(RoutingTableWidget {
                                state: self.state.clone(),
                                bool_states: &mut self.bool_states,
                            });
                        });

                        ui.end_row();
                        ui.spacing();
                    });
                }
            }

            let mut midi_mgr = self.midi_mgr.lock().unwrap();

            if before_port != after_port {
                midi_mgr.disconnect();
                midi_mgr.connect(self.state.clone()).unwrap();
            } else if !conn_ok {
                midi_mgr.disconnect();
            } else if conn_ok && midi_mgr.conn.is_none() {
                midi_mgr.connect(self.state.clone()).unwrap();
            }
        });
    }
}
