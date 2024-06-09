use egui::Align2;
use egui::Vec2;

#[cfg(not(target_arch = "wasm32"))]
use std::io::Read;
#[cfg(not(target_arch = "wasm32"))]
use std::fs::File;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};

use egui::ProgressBar;

use crate::log_file::*;
use crate::utils::execute;

pub struct OpenFileDialog {
    file_receiver: Receiver<LogFile>,
    file_progress_receiver: Receiver<f32>,
    flight_progress_receiver: Receiver<f32>,

    file_progress: f32,
    flight_progress: f32,
}

impl OpenFileDialog {
    pub fn new(ctx: &egui::Context) -> Self {
        let (file_sender, file_receiver) = channel();
        let (file_progress_sender, file_progress_receiver) = channel();
        let (flight_progress_sender, flight_progress_receiver) = channel();
        let ctx = ctx.clone();

        execute(async move {
            if let Some(file) = rfd::AsyncFileDialog::new().pick_file().await {
                let name = file.file_name();
                let bytes = file.read().await;

                let log_data = LogFile::parse(
                    name,
                    bytes,
                    &ctx,
                    file_progress_sender,
                    flight_progress_sender
                ).await;

                file_sender.send(log_data).unwrap();

                ctx.request_repaint();
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

    // TODO: clean up code duplication with new
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_path(ctx: &egui::Context, path: PathBuf) -> Self {
        let (file_sender, file_receiver) = channel();
        let (file_progress_sender, file_progress_receiver) = channel();
        let (flight_progress_sender, flight_progress_receiver) = channel();
        let ctx = ctx.clone();

        execute(async move {
            let name = path.file_name().unwrap().to_str().unwrap().into(); // TODO
            let mut bytes = Vec::new();
            let mut f = File::open(path).unwrap(); // TODO
            f.read_to_end(&mut bytes).unwrap(); // TODO

            let log_data = LogFile::parse(
                name,
                bytes,
                &ctx,
                file_progress_sender,
                flight_progress_sender
            ).await;

            file_sender.send(log_data).unwrap();

            ctx.request_repaint();
        });

        Self {
            file_receiver,
            file_progress_receiver,
            flight_progress_receiver,

            file_progress: 0.0,
            flight_progress: 0.0,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<LogFile> {
        while let Ok(file_progress) = self.file_progress_receiver.try_recv() {
            self.file_progress = file_progress;
        }

        while let Ok(flight_progress) = self.flight_progress_receiver.try_recv() {
            self.flight_progress = flight_progress;
        }

        egui::Window::new("Open File")
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
                        .text("text")
                        .show_percentage()
                        .animate(true);

                    let flight_pb = ProgressBar::new(self.flight_progress)
                        .desired_width(ui.available_width())
                        .text("text")
                        .show_percentage()
                        .animate(true);

                    ui.add(file_pb);
                    ui.add(flight_pb);
                });
            });


        self.file_receiver.try_recv().ok()
    }
}
