use egui::Align2;
use egui::Vec2;

#[cfg(not(target_arch = "wasm32"))]
use std::fs::File;
#[cfg(not(target_arch = "wasm32"))]
use std::io::Read;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

use std::sync::mpsc::TryRecvError;
use std::sync::mpsc::{channel, Receiver};

use egui::ProgressBar;

use crate::log_file::*;
use crate::utils::execute_in_background;

pub struct OpenFileDialog {
    file_receiver: Receiver<Option<LogFile>>,
    file_progress_receiver: Receiver<f32>,
    flight_progress_receiver: Receiver<f32>,

    file_progress: f32,
    flight_progress: f32,
}

impl OpenFileDialog {
    pub fn new(path: Option<PathBuf>) -> Self {
        // Setup 3 different channel for receiving:
        // Flight loading % [0,1]
        // File loading % [0,1]
        // Option<LogFile> final result
        let (flight_progress_sender, flight_progress_receiver) = channel();
        let (file_progress_sender, file_progress_receiver) = channel();
        let (file_sender, file_receiver) = channel();

        // File parsing happens in the background task
        execute_in_background(async move {
            match Self::pick_read_file(path).await {
                Some((name, bytes)) => {
                    let log_data =
                        LogFile::parse(name, bytes, file_progress_sender, flight_progress_sender)
                            .await;

                    file_sender.send(Some(log_data)).unwrap();
                }
                None => file_sender.send(None).unwrap(),
            }
        });

        Self {
            file_receiver,
            file_progress_receiver,
            flight_progress_receiver,

            file_progress: 0.0,
            flight_progress: 0.0,
        }
    }

    // This function needs to be async due to blocking rfd::FileDialog is not available on wasm32
    async fn pick_read_file(path: Option<PathBuf>) -> Option<(String, Vec<u8>)> {
        match path {
            Some(path) => {
                let name = path
                    .file_name()?
                    .to_str()
                    .to_owned()
                    .map(|f| f.to_string())?;
                let mut bytes = Vec::new();
                let mut f = File::open(path).ok()?;
                f.read_to_end(&mut bytes).ok()?;
                Some((name, bytes))
            }
            None => {
                let file = rfd::AsyncFileDialog::new().pick_file().await?;
                Some((file.file_name(), file.read().await))
            }
        }
    }

    // Show Loading&parsing progress bars popup
    pub fn show(&mut self, ctx: &egui::Context) -> Result<Option<LogFile>, TryRecvError> {
        if let Ok(flight_progress) = self.flight_progress_receiver.try_recv() {
            self.flight_progress = flight_progress;
        }

        if let Ok(file_progress) = self.file_progress_receiver.try_recv() {
            self.file_progress = file_progress;
        }

        egui::Window::new("Parsing File")
            .anchor(Align2::CENTER_CENTER, Vec2::splat(0.0))
            .movable(false)
            .resizable(false)
            .collapsible(false)
            .min_width(f32::min(400.0, ctx.available_rect().width()))
            .max_width(f32::min(400.0, ctx.available_rect().width()))
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    let file_pb = ProgressBar::new(self.file_progress)
                        .desired_width(ui.available_width())
                        .show_percentage()
                        .animate(true);

                    let flight_pb = ProgressBar::new(self.flight_progress)
                        .desired_width(ui.available_width())
                        .show_percentage()
                        .animate(true);

                    ui.add(file_pb);
                    ui.add(flight_pb);
                });
            });

        self.file_receiver.try_recv()
    }
}
