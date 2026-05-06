use crate::widgets::{AxisBehaviourState, BoolBehaviourState, SourceDeviceWidget, TargetDeviceWidget};
use crate::pipeweaver_main::SharedState;
use crate::pwv_controllers::{AxisCommand, BoolCommand};
use eframe::{EframePumpStatus, UserEvent, egui};
use egui::Button;
use midi_msg::MidiMsg;
use midir::{MidiInput, MidiInputConnection};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::{io, os::fd::AsRawFd as _};
use winit::event_loop::{ControlFlow, EventLoop};

pub async fn run(state: Arc<Mutex<SharedState>>) -> io::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default(),
        ..Default::default()
    };

    let mut eventloop = EventLoop::<UserEvent>::with_user_event().build().unwrap();
    eventloop.set_control_flow(ControlFlow::Poll);

    let app = PwvMidiGUI::new(state);

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
                    match state_handle.lock().unwrap().midi_tree.exec(&msg) {
                        Ok(_) => {}
                        Err(e) => {
                            if e.kind() == io::ErrorKind::InvalidData {
                                eprintln!("{}", e)
                            }
                        }
                    }
                    state_handle.lock().unwrap().last_midi_event = Some(msg);
                },
                (),
            )
            .unwrap();
        self.conn = Some(conn);
        println!("connected to {}", name);
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
        egui::CentralPanel::default().show_inside(ui, |ui| {
            let mut conn_ok = true;
            let (before_port, after_port) = {
                let mut state = self.state.lock().unwrap();
                ui.horizontal(|ui| {
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
                    egui::ComboBox::from_label("MIDI Device")
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
                    let txt = if state.learn_msg == None {
                        String::from("None")
                    } else {
                        format!("{:?}", state.learn_msg.clone().unwrap())
                    };
                    ui.label(txt);
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

                    (before_port, after_port)
                })
                .inner
            };

            let tx = self.state.lock().unwrap().tx.clone();
            match &self.state.lock().unwrap().status {
                None => {
                    ui.label("Pipeweaver not connected!");
                }
                Some(status) => {
                    if let Some(tx) = tx {
                        let profile = &status.audio.profile;

                        egui::scroll_area::ScrollArea::horizontal().show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing();
                                for psd in &profile.devices.sources.physical_devices {
                                    let id = psd.description.id;
                                    let name = psd.description.name.clone();
                                    ui.add(SourceDeviceWidget {
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
                                    for rt in &profile.routes {
                                        ui.label(format!("{:?}", rt).as_str()); // TODO custom widget for these
                                    }
                                });

                                ui.end_row();
                                ui.spacing();
                        });
                    }
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
