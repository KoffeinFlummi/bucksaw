use std::num::NonZero;
use std::sync::mpsc::channel;
use std::sync::Arc;

use egui::DragValue;
use egui_oszi::{TimeseriesGroup, TimeseriesLine, TimeseriesPlot, TimeseriesPlotMemory};
use egui_plot::{Corner, Legend, PlotPoints};

use crate::gui::colors::Colors;
use crate::gui::flex::{FlexColumns, FlexLayout};
use crate::step_response::{calculate_step_response, StepResponseConfiguration, StepResponseError};
use crate::utils::execute_in_background;
use crate::{flight_data::FlightData, utils::BackgroundCompStore};

use super::{MIN_WIDE_WIDTH, PLOT_HEIGHT};

type StepResponseResult = Result<Vec<(f64, f64)>, StepResponseError>;

pub struct TuneTab {
    roll_plot: TimeseriesPlotMemory<f64, f32>,
    pitch_plot: TimeseriesPlotMemory<f64, f32>,
    yaw_plot: TimeseriesPlotMemory<f64, f32>,
    roll_step_response: BackgroundCompStore<StepResponseResult>,
    pitch_step_response: BackgroundCompStore<StepResponseResult>,
    yaw_step_response: BackgroundCompStore<StepResponseResult>,
    fd: Arc<FlightData>,
    config: StepResponseConfiguration,
}

const AXIS_LABELS: [&str; 3] = ["Roll", "Pitch", "Yaw"];

impl TuneTab {
    pub fn new(fd: Arc<FlightData>) -> Self {
        let config = StepResponseConfiguration::default();
        let roll_step_response = Self::calculate_response(&fd, 0, &config, None);
        let pitch_step_response = Self::calculate_response(&fd, 0, &config, None);
        let yaw_step_response = Self::calculate_response(&fd, 0, &config, None);

        Self {
            roll_plot: TimeseriesPlotMemory::new("roll"),
            pitch_plot: TimeseriesPlotMemory::new("pitch"),
            yaw_plot: TimeseriesPlotMemory::new("yaw"),
            roll_step_response,
            pitch_step_response,
            yaw_step_response,
            fd,
            config,
        }
    }

    fn calculate_response(
        fd: &Arc<FlightData>,
        i: usize,
        config: &StepResponseConfiguration,
        ctx: Option<&egui::Context>,
    ) -> BackgroundCompStore<StepResponseResult> {
        let fd = fd.clone();
        let (sender, receiver) = channel();
        let step_response = BackgroundCompStore::new(receiver);

        let config2 = config.clone();
        let ctx = ctx.cloned();

        execute_in_background(async move {
            let empty_fallback = Vec::new();
            let setpoints = fd.setpoint().unwrap_or([&empty_fallback; 4]);
            let gyro = fd.gyro_filtered().unwrap_or([&empty_fallback; 3]);
            let sr = fd.sample_rate();
            let result = calculate_step_response(&fd.times, setpoints[i], gyro[i], sr, &config2);
            let _ = sender.send(result);
            if let Some(ctx) = ctx {
                ctx.request_repaint();
            }
        });

        step_response
    }

    pub fn plot_step_response(
        ui: &mut egui::Ui,
        i: usize,
        step_response: &[(f64, f64)],
        total_width: f32,
    ) -> egui::Response {
        let height = if ui.available_width() < total_width {
            ui.available_height() / (3 - i) as f32
        } else {
            PLOT_HEIGHT
        };

        egui_plot::Plot::new(ui.next_auto_id())
            .legend(Legend::default().position(Corner::RightBottom))
            .set_margin_fraction(egui::Vec2::new(0.0, 0.1))
            .show_grid(true)
            .allow_drag(false)
            .allow_zoom(false)
            .allow_scroll(false)
            .link_axis("step_response", true, true)
            .link_cursor("step_response", true, true)
            .y_axis_position(egui_plot::HPlacement::Right)
            .y_axis_width(3)
            .height(height)
            .show(ui, |plot_ui| {
                let points = PlotPoints::new(step_response.iter().map(|(x, y)| [*x, *y]).collect());
                let egui_line = egui_plot::Line::new(points)
                    .name(format!("Step Response ({})", AXIS_LABELS[i]))
                    .color(egui::Color32::from_rgb(0xaf, 0x3a, 0x03))
                    .width(2.0);
                plot_ui.line(egui_line);
            })
            .response
    }

    pub fn show(&mut self, ui: &mut egui::Ui, timeseries_group: &mut TimeseriesGroup) {
        let mut recalculate = false;
        let old_config = self.config.clone();

        let prefilter_normalization = self.config.enable_prefilter_normalization;
        let steady_state_mean_check = self.config.enable_normalized_steady_state_mean_check;

        FlexLayout::new(1500.0, "Step Response Settings")
            .add(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Response Length:");
                    ui.add(
                        DragValue::new(&mut self.config.response_length)
                            .clamp_range(0.0..=10.0)
                            .speed(0.05)
                            .suffix("s"),
                    );
                })
                .response
            })
            .add(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Window length/overlap:");
                    ui.add(
                        DragValue::new(&mut self.config.window_length)
                            .clamp_range(0.0..=10.0)
                            .speed(0.05)
                            .suffix("s"),
                    );
                    ui.label("/");
                    let mut perc = self.config.window_overlap * 100.0;
                    ui.add(
                        DragValue::new(&mut perc)
                            .clamp_range(0.0..=99.0)
                            .speed(0.25)
                            .suffix("%"),
                    );
                    self.config.window_overlap = perc / 100.0;
                })
                .response
            })
            //.add(|ui| {
            //    ui.horizontal(|ui| {
            //        ui.label("Tukey Window Alpha:");
            //        ui.add(
            //            DragValue::new(&mut self.config.tukey_alpha)
            //                .clamp_range(0.0..=1.0)
            //                .speed(0.01),
            //        );
            //    })
            //    .response
            //})
            .add(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Gyro mov. avg. window:");
                    let mut val = self.config.gyro_mov_avg_window.get();
                    ui.add(DragValue::new(&mut val).clamp_range(1..=100));
                    self.config.gyro_mov_avg_window = NonZero::new(val).unwrap();
                })
                .response
            })
            .add(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Min. |setpoint|:");
                    ui.add(
                        DragValue::new(&mut self.config.min_setpoint)
                            .clamp_range(0.0..=1000.0)
                            .speed(5.0),
                    );
                })
                .response
            })
            .add(|ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut self.config.enable_prefilter_normalization,
                        "Pre-filter normalization",
                    );
                })
                .response
            })
            .add(|ui| {
                ui.horizontal(|ui| {
                    ui.set_enabled(prefilter_normalization);
                    ui.label("Pre-filter norm. minimum");
                    ui.add(
                        DragValue::new(&mut self.config.prefilter_normalization_minimum)
                            .clamp_range(0.0..=10.0)
                            .speed(0.05),
                    );
                })
                .response
            })
            .add(|ui| {
                ui.horizontal(|ui| {
                    let mut a = self.config.normalized_steady_state_range.start().clone();
                    let mut b = self.config.normalized_steady_state_range.end().clone();
                    ui.label("Normalized steady-state range:");
                    ui.add(DragValue::new(&mut a).clamp_range(0.0..=10.0).speed(0.05));
                    ui.label("-");
                    ui.add(DragValue::new(&mut b).clamp_range(0.0..=10.0).speed(0.05));
                    self.config.normalized_steady_state_range = a..=b;
                })
                .response
            })
            .add(|ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut self.config.enable_normalized_steady_state_mean_check,
                        "Normalized steady state mean check",
                    );
                })
                .response
            })
            .add(|ui| {
                ui.horizontal(|ui| {
                    ui.set_enabled(steady_state_mean_check);
                    let mut a = self
                        .config
                        .normalized_steady_state_mean_range
                        .start()
                        .clone();
                    let mut b = self.config.normalized_steady_state_mean_range.end().clone();
                    ui.label("Normalized steady-state mean range:");
                    ui.add(DragValue::new(&mut a).clamp_range(0.0..=10.0).speed(0.05));
                    ui.label("-");
                    ui.add(DragValue::new(&mut b).clamp_range(0.0..=10.0).speed(0.05));
                    self.config.normalized_steady_state_mean_range = a..=b;
                })
                .response
            })
            .add(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Steady state range");
                    ui.add(
                        DragValue::new(&mut self.config.steady_state_start_seconds)
                            .clamp_range(0.0..=10.0)
                            .speed(0.05)
                            .suffix("s"),
                    );
                    ui.label("-");
                    ui.add(
                        DragValue::new(&mut self.config.steady_state_end_seconds)
                            .clamp_range(0.0..=10.0)
                            .speed(0.05)
                            .suffix("s"),
                    );
                })
                .response
            })
            .add(|ui| {
                ui.horizontal(|ui| {
                    recalculate = ui.button("Recalculate").clicked();
                })
                .response
            })
            .show(ui);

        if recalculate || old_config != self.config {
            self.roll_step_response =
                Self::calculate_response(&self.fd, 0, &self.config, Some(ui.ctx()));
            self.pitch_step_response =
                Self::calculate_response(&self.fd, 1, &self.config, Some(ui.ctx()));
            self.yaw_step_response =
                Self::calculate_response(&self.fd, 2, &self.config, Some(ui.ctx()));
        }

        ui.separator();

        let total_width = ui.available_width();
        let times = &self.fd.times;
        let colors = Colors::get(ui);
        FlexColumns::new(MIN_WIDE_WIDTH)
            .column(|ui| {
                ui.vertical(|ui| {
                    ui.heading("Time Domain");

                    let axes = [
                        &mut self.roll_plot,
                        &mut self.pitch_plot,
                        &mut self.yaw_plot,
                    ];
                    for (i, plot) in axes.into_iter().enumerate() {
                        let height = if ui.available_width() < total_width {
                            ui.available_height() / (3 - i) as f32
                        } else {
                            PLOT_HEIGHT
                        };

                        let label = AXIS_LABELS[i];
                        ui.add(
                            TimeseriesPlot::new(plot)
                                .group(timeseries_group)
                                .legend(Legend::default().position(Corner::LeftTop))
                                .height(height)
                                .line(
                                    TimeseriesLine::new(format!("Gyro ({}, unfilt.)", label))
                                        .color(colors.gyro_unfiltered),
                                    times.iter().copied().zip(
                                        self.fd
                                            .gyro_unfiltered()
                                            .map(|s| s[i].iter().copied())
                                            .unwrap_or_default(),
                                    ),
                                )
                                .line(
                                    TimeseriesLine::new(format!("Gyro ({})", label))
                                        .color(colors.gyro_filtered),
                                    times.iter().copied().zip(
                                        self.fd
                                            .gyro_filtered()
                                            .map(|s| s[i].iter().copied())
                                            .unwrap_or_default(),
                                    ),
                                )
                                .line(
                                    TimeseriesLine::new(format!("Setpoint ({})", label))
                                        .color(colors.setpoint),
                                    times.iter().copied().zip(
                                        self.fd
                                            .setpoint()
                                            .map(|s| s[i].iter().copied())
                                            .unwrap_or_default(),
                                    ),
                                )
                                .line(
                                    TimeseriesLine::new(format!("P ({})", label)).color(colors.p),
                                    times.iter().copied().zip(
                                        self.fd
                                            .p()
                                            .map(|s| s[i].iter().copied())
                                            .unwrap_or_default(),
                                    ),
                                )
                                .line(
                                    TimeseriesLine::new(format!("I ({})", label)).color(colors.i),
                                    times.iter().copied().zip(
                                        self.fd
                                            .i()
                                            .map(|s| s[i].iter().copied())
                                            .unwrap_or_default(),
                                    ),
                                )
                                .line(
                                    TimeseriesLine::new(format!("D ({})", label)).color(colors.d),
                                    times.iter().copied().zip(
                                        self.fd.d()[i]
                                            .map(|s| s.iter().copied())
                                            .unwrap_or_default(),
                                    ),
                                )
                                .line(
                                    TimeseriesLine::new(format!("F ({})", label)).color(colors.f),
                                    times.iter().copied().zip(
                                        self.fd
                                            .f()
                                            .map(|s| s[i].iter().copied())
                                            .unwrap_or_default(),
                                    ),
                                ),
                        );
                    }
                })
                .response
            })
            .column(|ui| {
                ui.vertical(|ui| {
                    ui.heading("Step Response");

                    let height = ui.available_height() / 3.0;

                    for (i, result) in [
                        &mut self.roll_step_response,
                        &mut self.pitch_step_response,
                        &mut self.yaw_step_response,
                    ]
                    .iter_mut()
                    .enumerate()
                    {
                        match result.get() {
                            Some(Ok(axis)) => Self::plot_step_response(ui, i, axis, total_width),
                            Some(Err(e)) => {
                                ui.vertical_centered(|ui| {
                                    ui.set_height(height);
                                    ui.heading("Failed to calculate step response");
                                    ui.monospace(format!("{:?}", e));
                                })
                                .response
                            }
                            None => {
                                ui.vertical_centered(|ui| {
                                    ui.set_height(height);
                                })
                                .response
                            }
                        };
                    }
                })
                .response
            })
            .show(ui);
    }
}
