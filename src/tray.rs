use crate::{APP_NAME, ICON};
use anyhow::Result;

use image::GenericImageView;

use ksni::menu::StandardItem;
use ksni::{Category, Icon, MenuItem, Status, ToolTip, Tray, TrayMethods};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;
use egui::{Context, ViewportCommand};
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;

pub(super) struct TrayState {
    pub(super) ui_on: Arc<AtomicBool>,
    pub(super) ctx: Option<Context>
}


enum TrayMessages {
    Toggle,
    Quit,
}

pub async fn spawn_tray(stop: Arc<AtomicBool>, tray_state: Arc<Mutex<TrayState>>) -> Result<()> {
    println!("Spawning Tray");

    let (icon_tx, mut icon_rx) = mpsc::channel(20);
    let icon = TrayIcon::new(icon_tx);
    let handle = icon.disable_dbus_name(true).assume_sni_available(true).spawn().await?;
    let mut interval = tokio::time::interval(Duration::from_millis(100));

    loop {
        tokio::select! {
            Some(msg) = icon_rx.recv() => {
                match msg {
                    TrayMessages::Toggle => {
                        let ui_on = tray_state.lock().unwrap().ui_on.clone();
                        let prev_on = ui_on.load(Ordering::Relaxed);
                        ui_on.store(!prev_on, Ordering::Relaxed);
                    },
                    TrayMessages::Quit => {
                        stop.store(true, Ordering::Relaxed);
                        match tray_state.lock().unwrap().ctx.clone() {
                            Some(ctx) => {
                                ctx.send_viewport_cmd(
                                    ViewportCommand::Close
                                )
                            },
                            None => {},
                        };
                        break;
                    }
                }
            }
            _ = interval.tick() => {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
            }
        }
    }

    println!("Stopping Tray");
    if !handle.is_closed() {
        handle.shutdown();
    }

    // Remove the temporary icon file
    println!("Tray Stopped");
    Ok(())
}

struct TrayIcon {
    tx: Sender<TrayMessages>,
}

impl TrayIcon {
    fn new(tx: Sender<TrayMessages>) -> Self {
        Self { tx }
    }
}

impl Tray for TrayIcon {
    fn id(&self) -> String {
        APP_NAME.to_string()
    }
    fn category(&self) -> Category {
        Category::SystemServices
    }
    fn title(&self) -> String {
        APP_NAME.to_string()
    }
    fn status(&self) -> Status {
        Status::Active
    }
    fn icon_pixmap(&self) -> Vec<Icon> {
        static TRAY_ICON: LazyLock<Icon> = LazyLock::new(|| {
            let img = image::load_from_memory_with_format(ICON, image::ImageFormat::Png).expect("Unable to Load Image");

            let (width, height) = img.dimensions();
            let mut data = img.into_rgba8().into_vec();

            for pixel in data.chunks_exact_mut(4) {
                pixel.rotate_right(1) // RGBA to ARGB
            }

            Icon {
                width: width as i32,
                height: height as i32,
                data,
            }
        });

        vec![TRAY_ICON.clone()]
    }

    fn tool_tip(&self) -> ToolTip {
        ToolTip {
            title: String::from(APP_NAME),
            description: String::from("PipeWeaver Audio Mixer"),
            ..Default::default()
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            StandardItem {
                label: String::from("Toggle UI"),
                activate: Box::new(|this: &mut TrayIcon| {
                    let _ = this.tx.try_send(TrayMessages::Toggle);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: String::from("Quit"),
                activate: Box::new(|this: &mut TrayIcon| {
                    let _ = this.tx.try_send(TrayMessages::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}
