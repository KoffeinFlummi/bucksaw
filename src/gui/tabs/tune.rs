use egui_plot::{Corner, Legend};
use egui_oszi::{TimeseriesGroup, TimeseriesLine, TimeseriesPlot, TimeseriesPlotMemory};

use crate::gui::colors::Colors;
use crate::flight_data::FlightData;

pub struct TuneTab {
    roll_plot: TimeseriesPlotMemory<f64>,
    pitch_plot: TimeseriesPlotMemory<f64>,
    yaw_plot: TimeseriesPlotMemory<f64>,
}

// TODO: duplication
const PLOT_HEIGHT: f32 = 300.0;

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
        let times = &fd.times;
        let legend = Legend::default().position(Corner::LeftTop);

        let colors = Colors::get(ui);


        let axes = [
            (&mut self.roll_plot, "Roll"),
            (&mut self.pitch_plot, "Pitch"),
            (&mut self.yaw_plot, "Yaw")
        ];

        for (i, (plot, name)) in axes.into_iter().enumerate() {
            ui.heading(name);
            ui.add(
                TimeseriesPlot::new(plot)
                    .group(timeseries_group)
                    .legend(legend.clone())
                    .height(PLOT_HEIGHT)
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
    }
}
