use crate::midi_callbacks::is_event_valid;
use crate::pipeweaver_main::SharedState;
use crate::pwv_controllers::{AxisCommand, BoolCommand};
use crate::widgets::{
    AxisBehaviourState, BoolBehaviourState, RoutingTableWidget, SourceDeviceWidget,
    TargetDeviceWidget,
};
use eframe::epaint::{Color32, CornerRadius, Stroke};
use eframe::{EframePumpStatus, UserEvent};
use egui::{Button, CentralPanel, ComboBox, DragValue, Frame};
use midi_msg::ControlChange::CC;
use midi_msg::{Channel, ChannelVoiceMsg, MidiMsg};
use midir::{MidiInput, MidiInputConnection};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::{io, os::fd::AsRawFd as _};
use winit::event_loop::{ControlFlow, EventLoop};

pub(crate) async fn run(state: Arc<Mutex<SharedState>>) -> io::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default(),
        ..Default::default()
    };

    let mut eventloop = EventLoop::<UserEvent>::with_user_event().build().unwrap();
    eventloop.set_control_flow(ControlFlow::Poll);

    let app = PwvMidiGUI::new(state.clone());

    let mut winit_app = eframe::create_native(
        "Pipeweaver-MIDI",
        options,
        Box::new(|_| Ok(Box::<PwvMidiGUI>::new(app))),
        &eventloop,
    );

    let eventloop_fd = tokio::io::unix::AsyncFd::new(eventloop.as_raw_fd())?;
    let mut control_flow = ControlFlow::Poll;

    loop {
        let mut guard = match control_flow {
            ControlFlow::Poll => None,
            ControlFlow::Wait => Some(eventloop_fd.readable().await?),
            ControlFlow::WaitUntil(deadline) => {
                tokio::time::timeout_at(deadline.into(), eventloop_fd.readable())
                    .await
                    .ok()
                    .transpose()?
            }
        };

        match winit_app.pump_eframe_app(&mut eventloop, None) {
            EframePumpStatus::Continue(next) => control_flow = next,
            EframePumpStatus::Exit(code) => {
                log::info!("exit code: {code}");
                break;
            }
        }

        if let Some(mut guard) = guard.take() {
            guard.clear_ready();
        }
    }

    Ok::<_, io::Error>(())
}

pub(crate) struct PwvMidiGUI {
    state: Arc<Mutex<SharedState>>,
    midi_dsc: MidiInput,
    conn: Option<MidiInputConnection<()>>,
    bool_states: HashMap<BoolCommand, BoolBehaviourState>,
    axis_states: HashMap<AxisCommand, AxisBehaviourState>,
}

impl PwvMidiGUI {
    fn new(state: Arc<Mutex<SharedState>>) -> Self {
        Self {
            state,
            midi_dsc: MidiInput::new("midi-discover").unwrap(),
            conn: None,
            bool_states: HashMap::new(),
            axis_states: HashMap::new(),
        }
    }

    fn connect(&mut self) -> io::Result<()> {
        let midi = MidiInput::new("midi-main").unwrap();
        let port = &self.state.lock().unwrap().current_port;
        let state_handle = self.state.clone();
        let name = self.midi_dsc.port_name(port).unwrap();
        let conn = midi
            .connect(
                &port,
                name.as_str(),
                move |_tm, data, _t| {
                    let msg = MidiMsg::from_midi(data).unwrap().0;
                    if is_event_valid(msg.clone()) {
                        state_handle.lock().unwrap().midi_tree.exec(&msg).unwrap();
                        state_handle.lock().unwrap().last_midi_event = Some(msg);
                    }
                },
                (),
            )
            .unwrap();
        self.conn = Some(conn);
        Ok(())
    }

    fn disconnect(&mut self) {
        if let Some(conn) = self.conn.take() {
            conn.close();
        }
    }
}

impl eframe::App for PwvMidiGUI {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
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

                            let (mut sel_channel, mut is_cc, mut sel_value) =
                                match state.clone().learn_msg {
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
                                                | ChannelVoiceMsg::NoteOn { note, velocity: _ } => {
                                                    (false, note)
                                                }
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
                                    .add(Button::new(if state.learning {
                                        "Learning..."
                                    } else {
                                        "Learn"
                                    }))
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
                                        ui.selectable_value(
                                            &mut sel_channel,
                                            Channel::Ch10,
                                            "Ch10",
                                        );
                                        ui.selectable_value(
                                            &mut sel_channel,
                                            Channel::Ch11,
                                            "Ch11",
                                        );
                                        ui.selectable_value(
                                            &mut sel_channel,
                                            Channel::Ch12,
                                            "Ch12",
                                        );
                                        ui.selectable_value(
                                            &mut sel_channel,
                                            Channel::Ch13,
                                            "Ch13",
                                        );
                                        ui.selectable_value(
                                            &mut sel_channel,
                                            Channel::Ch14,
                                            "Ch14",
                                        );
                                        ui.selectable_value(
                                            &mut sel_channel,
                                            Channel::Ch15,
                                            "Ch15",
                                        );
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

            // let tx = self.state.lock().unwrap().tx.clone();

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

            if before_port != after_port {
                self.disconnect();
                self.connect().unwrap();
            } else if !conn_ok {
                self.disconnect();
            } else if conn_ok && self.conn.is_none() {
                self.connect().unwrap();
            }
        });
    }
}
