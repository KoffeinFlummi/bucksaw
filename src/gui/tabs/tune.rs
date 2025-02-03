use std::sync::Arc;

use egui_oszi::{TimeseriesGroup, TimeseriesLine, TimeseriesPlot, TimeseriesPlotMemory};
use egui_plot::{Corner, Legend, PlotPoints};

use crate::flight_data::FlightData;
use crate::gui::colors::Colors;
use crate::gui::flex::FlexColumns;
use crate::step_response::calculate_step_response;

pub struct TuneTab {
    roll_plot: TimeseriesPlotMemory<f64, f32>,
    pitch_plot: TimeseriesPlotMemory<f64, f32>,
    yaw_plot: TimeseriesPlotMemory<f64, f32>,

    roll_step_response: Vec<(f64, f64)>,
    pitch_step_response: Vec<(f64, f64)>,
    yaw_step_response: Vec<(f64, f64)>,
}

// TODO: duplication
const MIN_WIDE_WIDTH: f32 = 1000.0;
const AXIS_LABELS: [&str; 3] = ["Roll", "Pitch", "Yaw"];

impl TuneTab {
    pub fn new(_ctx: &egui::Context, fd: Arc<FlightData>) -> Self {
        let setpoints = fd.setpoint().unwrap(); // TODO
        let gyro = fd.gyro_filtered().unwrap(); // TODO
                                                // TODO: calculate step response in background thread
        Self {
            roll_plot: TimeseriesPlotMemory::new("roll"),
            pitch_plot: TimeseriesPlotMemory::new("pitch"),
            yaw_plot: TimeseriesPlotMemory::new("yaw"),

            roll_step_response: calculate_step_response(
                &fd.times,
                setpoints[0],
                gyro[0],
                fd.sample_rate(),
            ),
            pitch_step_response: calculate_step_response(
                &fd.times,
                setpoints[1],
                gyro[1],
                fd.sample_rate(),
            ),
            yaw_step_response: calculate_step_response(
                &fd.times,
                setpoints[2],
                gyro[2],
                fd.sample_rate(),
            ),
        }
    }

    pub fn set_flight(&mut self, fd: Arc<FlightData>) {
        let setpoints = fd.setpoint().unwrap(); // TODO
        let gyro = fd.gyro_filtered().unwrap(); // TODO
        self.roll_step_response =
            calculate_step_response(&fd.times, setpoints[0], gyro[0], fd.sample_rate());
        self.pitch_step_response =
            calculate_step_response(&fd.times, setpoints[1], gyro[1], fd.sample_rate());
        self.yaw_step_response =
            calculate_step_response(&fd.times, setpoints[2], gyro[2], fd.sample_rate());
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
            300.0
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

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        fd: &FlightData,
        timeseries_group: &mut TimeseriesGroup,
    ) {
        let total_width = ui.available_width();
        let times = &fd.times;
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
                            300.0
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
                                        fd.gyro_unfiltered()
                                            .map(|s| s[i].iter().copied())
                                            .unwrap_or_default(),
                                    ),
                                )
                                .line(
                                    TimeseriesLine::new(format!("Gyro ({})", label))
                                        .color(colors.gyro_filtered),
                                    times.iter().copied().zip(
                                        fd.gyro_filtered()
                                            .map(|s| s[i].iter().copied())
                                            .unwrap_or_default(),
                                    ),
                                )
                                .line(
                                    TimeseriesLine::new(format!("Setpoint ({})", label))
                                        .color(colors.setpoint),
                                    times.iter().copied().zip(
                                        fd.setpoint()
                                            .map(|s| s[i].iter().copied())
                                            .unwrap_or_default(),
                                    ),
                                )
                                .line(
                                    TimeseriesLine::new(format!("P ({})", label)).color(colors.p),
                                    times.iter().copied().zip(
                                        fd.p().map(|s| s[i].iter().copied()).unwrap_or_default(),
                                    ),
                                )
                                .line(
                                    TimeseriesLine::new(format!("I ({})", label)).color(colors.i),
                                    times.iter().copied().zip(
                                        fd.i().map(|s| s[i].iter().copied()).unwrap_or_default(),
                                    ),
                                )
                                .line(
                                    TimeseriesLine::new(format!("D ({})", label)).color(colors.d),
                                    times.iter().copied().zip(
                                        fd.d()[i].map(|s| s.iter().copied()).unwrap_or_default(),
                                    ),
                                )
                                .line(
                                    TimeseriesLine::new(format!("F ({})", label)).color(colors.f),
                                    times.iter().copied().zip(
                                        fd.f().map(|s| s[i].iter().copied()).unwrap_or_default(),
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

                    for (i, axis) in [
                        &self.roll_step_response,
                        &self.pitch_step_response,
                        &self.yaw_step_response,
                    ]
                    .iter()
                    .enumerate()
                    {
                        Self::plot_step_response(ui, i, axis, total_width);
                    }
                })
                .response
            })
            .show(ui);
    }
}
