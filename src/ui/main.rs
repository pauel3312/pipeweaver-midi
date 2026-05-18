use crate::common::SharedState;
use crate::midi::manager::MidiMgr;
use crate::pipeweaver_controllers::commands::{AxisCommand, BoolCommand};
use crate::tray::TrayState;
use crate::ui::behaviour_selectors::{AxisBehaviourState, BoolBehaviourState};
use crate::ui::device_widgets::{RoutingTableWidget, SourceDeviceWidget, TargetDeviceWidget};
use crate::ui::util_widgets::{ConfigWidget, MidiDeviceWidget};
use egui::{CentralPanel, ScrollArea, ViewportCommand};
use midir::MidiInput;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::time::sleep;

pub(crate) async fn run(
    state: Arc<Mutex<SharedState>>,
    midi_mgr: Arc<Mutex<MidiMgr>>,
    stop: Arc<AtomicBool>,
    tray: Arc<Mutex<TrayState>>,
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
            ScrollArea::both().show(ui, |ui| {
                ui.add(MidiDeviceWidget {
                    state: self.state.clone(),
                    midi_dsc: &self.midi_dsc,
                    midi_mgr: self.midi_mgr.clone(),
                });

                let status = self.state.lock().unwrap().status.clone();
                match status {
                    None => {
                        ui.label("Pipeweaver not connected!");
                    }
                    Some(status) => {
                        let profile = &status.audio.profile;
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
                    }
                }
                ui.add(ConfigWidget::new(&mut self.state.lock().unwrap().config))
            });
        });
    }
}
