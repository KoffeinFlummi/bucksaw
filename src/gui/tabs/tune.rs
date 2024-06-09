use egui_plot::{Corner, Legend};
use egui_oszi::{TimeseriesGroup, TimeseriesLine, TimeseriesPlot, TimeseriesPlotMemory};

use crate::gui::colors::Colors;
use crate::flight_data::FlightData;
use crate::gui::flex::FlexColumns;

pub struct TuneTab {
    roll_plot: TimeseriesPlotMemory<f64>,
    pitch_plot: TimeseriesPlotMemory<f64>,
    yaw_plot: TimeseriesPlotMemory<f64>,
}

// TODO: duplication
const MIN_WIDE_WIDTH: f32 = 1000.0;

impl TuneTab {
    pub fn new() -> Self {
        Self {
            roll_plot: TimeseriesPlotMemory::new("roll"),
            pitch_plot: TimeseriesPlotMemory::new("pitch"),
            yaw_plot: TimeseriesPlotMemory::new("yaw"),
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        fd: &FlightData,
        timeseries_group: &mut TimeseriesGroup
    ) {
        let total_width = ui.available_width();
        let times = &fd.times;
        let colors = Colors::get(ui);
        FlexColumns::new(MIN_WIDE_WIDTH)
            .column(|ui| ui.vertical(|ui| {
                ui.heading("Time Domain");

                let axes = [&mut self.roll_plot, &mut self.pitch_plot, &mut self.yaw_plot];
                for (i, plot) in axes.into_iter().enumerate() {
                    let height = if ui.available_width() < total_width {
                        ui.available_height() / (3 - i) as f32
                    } else {
                        300.0
                    };

                    ui.add(
                        TimeseriesPlot::new(plot)
                            .group(timeseries_group)
                            .legend(Legend::default().position(Corner::LeftTop))
                            .height(height)
                            .line(
                                TimeseriesLine::new("Gyro (unfilt.)").color(colors.gyro_unfiltered),
                                times.iter().copied().zip(fd.gyro_unfilt.as_ref().map(|s| s[i].iter().copied()).unwrap_or_default())
                            )
                            .line(
                                TimeseriesLine::new("Gyro").color(colors.gyro_filtered),
                                times.iter().copied().zip(fd.gyro_adc.as_ref().map(|s| s[i].iter().copied()).unwrap_or_default())
                            )
                            .line(
                                TimeseriesLine::new("Setpoint").color(colors.setpoint),
                                times.iter().copied().zip(fd.setpoint.as_ref().map(|s| s[i].iter().copied()).unwrap_or_default())
                            )
                            .line(
                                TimeseriesLine::new("P").color(colors.p),
                                times.iter().copied().zip(fd.p.as_ref().map(|s| s[i].iter().copied()).unwrap_or_default())
                            )
                            .line(
                                TimeseriesLine::new("I").color(colors.i),
                                times.iter().copied().zip(fd.i.as_ref().map(|s| s[i].iter().copied()).unwrap_or_default())
                            )
                            .line(
                                TimeseriesLine::new("D").color(colors.d),
                                times.iter().copied().zip(fd.d.as_ref().map(|s| s[i].iter().copied()).unwrap_or_default())
                            )
                            .line(
                                TimeseriesLine::new("F").color(colors.f),
                                times.iter().copied().zip(fd.f.as_ref().map(|s| s[i].iter().copied()).unwrap_or_default())
                            )
                    );
                }
            }).response)
            .column(|ui| {
                ui.heading("Step Response")

            })
            .show(ui);
    }
}
