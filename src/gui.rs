pub mod blackbox_ui_ext;
pub mod colors;
pub mod flex;
pub mod flight_view;
pub mod open_file;
pub mod tabs;

use std::path::PathBuf;
use std::sync::Arc;

use blackbox_log::headers::ParseError;
use egui::Layout;
use egui::Vec2;
use itertools::Itertools;

use crate::flight_data::FlightData;
use crate::gui::blackbox_ui_ext::*;
use crate::gui::colors::Colors;
use crate::gui::flight_view::*;
use crate::gui::open_file::*;
use crate::gui::tabs::*;
use crate::log_file::*;

type FlightDataAndView = Vec<(Result<Arc<FlightData>, ParseError>, Option<FlightView>)>;

pub struct App {
    open_file_dialog: Option<OpenFileDialog>,
    flight_view_tab: FlightViewTab,
    flights_data_views: FlightDataAndView,
    selected: usize,
    left_panel_open: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext, path: Option<PathBuf>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let open_file_dialog = Some(OpenFileDialog::new(path));
        Self {
            open_file_dialog,
            flight_view_tab: FlightViewTab::Plot,
            flights_data_views: Default::default(),
            selected: Default::default(),
            left_panel_open: true,
        }
    }

    fn open_log(&mut self, ctx: &egui::Context, file_data: LogFile) {
        self.flights_data_views = file_data
            .flights
            .into_iter()
            .map(|f| match f {
                Ok(f) => {
                    let f = Arc::new(f);
                    (Ok(f.clone()), Some(FlightView::new(ctx, f)))
                }
                Err(e) => (Err(e), None),
            })
            .collect_vec();

        let single_log = self.flights_data_views.len() == 1;
        self.open_file_dialog = None;

        if single_log || ctx.available_rect().width() < 1000.0 {
            self.left_panel_open = false;
        }
    }
}

impl eframe::App for App {
    /// Main draw method of the application
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(feature = "profiling")]
        puffin::profile_function!();
        #[cfg(feature = "profiling")]
        puffin::GlobalProfiler::lock().new_frame();
        #[cfg(feature = "profiling")]
        puffin_egui::profiler_window(ctx);

        if let Some(open_file_dialog) = self.open_file_dialog.as_mut() {
            match open_file_dialog.show(ctx) {
                Some(Some(result)) => {
                    self.open_log(ctx, result);
                }
                Some(None) => {
                    self.open_file_dialog = None;
                }
                None => {} // Not done yet.
            }
        }

        let enabled = self.open_file_dialog.is_none();

        let width = ctx.available_rect().width();
        let narrow = width < 400.0;

        egui::TopBottomPanel::top("menubar")
            .min_height(30.0)
            .max_height(30.0)
            .show(ctx, |ui| {
                ui.set_enabled(enabled);
                ui.horizontal_centered(|ui| {
                    if ui
                        .button(if self.left_panel_open { "⏴" } else { "☰" })
                        .clicked()
                    {
                        self.left_panel_open = !self.left_panel_open;
                    }

                    // TODO: right panel (ℹ)

                    if ui
                        .button(if narrow { "🗁 " } else { "🗁  Open File" })
                        .clicked()
                    {
                        self.open_file_dialog = Some(OpenFileDialog::new(None));
                        ctx.request_repaint();
                    }

                    ui.separator();

                    const TABS: [FlightViewTab; 3] = [
                        FlightViewTab::Plot,
                        FlightViewTab::Tune,
                        FlightViewTab::Vibe,
                    ];
                    for tab in TABS.into_iter() {
                        let label = if narrow {
                            tab.to_string().split(' ').next().unwrap().to_string()
                        } else {
                            tab.to_string()
                        };
                        ui.selectable_value(&mut self.flight_view_tab, tab, label);
                    }

                    ui.separator();

                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.hyperlink_to("", env!("CARGO_PKG_REPOSITORY"));
                        ui.separator();
                        egui::widgets::global_dark_light_mode_switch(ui);
                        ui.separator();
                    });
                });
            });

        if self.left_panel_open {
            let panel_draw = |ui: &mut egui::Ui| {
                ui.set_enabled(enabled);
                ui.set_width(ui.available_width());
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.set_width(ui.available_width());

                    let colors = Colors::get(ui);
                    let row_colors: Vec<_> = self
                        .flights_data_views
                        .iter()
                        .map(|(result, _)| match result {
                            Err(_) => Some(colors.error.gamma_multiply(0.3)),
                            Ok(flight) if self.selected == flight.index => {
                                Some(ui.visuals().selection.bg_fill.gamma_multiply(0.5))
                            }
                            Ok(_) => None,
                        })
                        .collect();
                    egui::Grid::new("flight_list")
                        .with_row_color(move |i, _style| row_colors.get(i).copied().flatten())
                        .num_columns(1)
                        .spacing(Vec2::new(0.0, 10.0))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());

                            for (i, (parse_result, _)) in self.flights_data_views.iter().enumerate()
                            {
                                ui.vertical(|ui| {
                                    ui.set_width(ui.available_width());

                                    ui.horizontal(|ui| {
                                        if parse_result.is_ok() {
                                            ui.label("Flight ");
                                        } else {
                                            ui.label("⚠ Flight ");
                                        }
                                        ui.monospace(format!("#{}", i + 1));

                                        if parse_result.is_ok() {
                                            ui.with_layout(
                                                Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if ui.button("➡").clicked() {
                                                        self.selected = i;
                                                    }
                                                },
                                            );
                                        }
                                    });

                                    match parse_result {
                                        Ok(flight) => {
                                            flight.show(ui);
                                        }
                                        Err(error) => {
                                            error.show(ui);
                                        }
                                    }
                                });

                                ui.end_row();
                            }
                        });
                });
            };
            if narrow {
                egui::CentralPanel::default().show(ctx, panel_draw);
            } else {
                egui::SidePanel::left("browserpanel")
                    .resizable(true)
                    .min_width(100.0)
                    .max_width(300.0)
                    .show(ctx, panel_draw);
            }
        }

        if !(self.left_panel_open && narrow) {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.set_enabled(enabled);

                if let Some((_, Some(view))) = self.flights_data_views.get_mut(self.selected) {
                    view.show(ui, self.flight_view_tab);
                }
            });
        }
    }
}
