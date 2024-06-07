use egui_plot::{Corner, Legend};
use egui_oszi::{TimeseriesGroup, TimeseriesLine, TimeseriesPlot, TimeseriesPlotMemory};

use crate::gui::colors::Colors;
use crate::flight_data::FlightData;

pub struct PlotTab {
    gyro_plot: TimeseriesPlotMemory<f64>,
    acc_plot: TimeseriesPlotMemory<f64>,
    rc_plot: TimeseriesPlotMemory<f64>,
    battery_plot: TimeseriesPlotMemory<f64>,
    rssi_plot: TimeseriesPlotMemory<f64>,
    motor_plot: TimeseriesPlotMemory<f64>,
    erpm_plot: TimeseriesPlotMemory<f64>,
}

// TODO: duplication
const PLOT_HEIGHT: f32 = 300.0;

impl PlotTab {
    pub fn new() -> Self {
        Self {
            gyro_plot: TimeseriesPlotMemory::new("gyro"),
            acc_plot: TimeseriesPlotMemory::new("acc"),
            rc_plot: TimeseriesPlotMemory::new("rc"),
            battery_plot: TimeseriesPlotMemory::new("battery"),
            rssi_plot: TimeseriesPlotMemory::new("rssi"),

            motor_plot: TimeseriesPlotMemory::new("motors"),
            erpm_plot: TimeseriesPlotMemory::new("erpm"),
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

        ui.heading("Gyroscope");
        ui.add(
            TimeseriesPlot::new(&mut self.gyro_plot)
                .group(timeseries_group)
                .legend(legend.clone())
                .height(PLOT_HEIGHT)
                .line(
                    TimeseriesLine::new("gyroUnfilt[0]").color(colors.triple_secondary[0]),
                    times.iter().copied().zip(fd.gyro_unfilt.as_ref().map(|s| s.x().iter().copied()).unwrap_or_default())
                )
                .line(
                    TimeseriesLine::new("gyroUnfilt[1]").color(colors.triple_secondary[1]),
                    times.iter().copied().zip(fd.gyro_unfilt.as_ref().map(|s| s.y().iter().copied()).unwrap_or_default())
                )
                .line(
                    TimeseriesLine::new("gyroUnfilt[2]").color(colors.triple_secondary[2]),
                    times.iter().copied().zip(fd.gyro_unfilt.as_ref().map(|s| s.z().iter().copied()).unwrap_or_default())
                )
                .line(
                    TimeseriesLine::new("gyroADC[0]").color(colors.triple_primary[0]),
                    times.iter().copied().zip(fd.gyro_adc.as_ref().map(|s| s.x().iter().copied()).unwrap_or_default())
                )
                .line(
                    TimeseriesLine::new("gyroADC[1]").color(colors.triple_primary[1]),
                    times.iter().copied().zip(fd.gyro_adc.as_ref().map(|s| s.y().iter().copied()).unwrap_or_default())
                )
                .line(
                    TimeseriesLine::new("gyroADC[2]").color(colors.triple_primary[2]),
                    times.iter().copied().zip(fd.gyro_adc.as_ref().map(|s| s.z().iter().copied()).unwrap_or_default())
                )
        );

        ui.heading("Accelerometer");
        ui.add(
            TimeseriesPlot::new(&mut self.acc_plot)
                .group(timeseries_group)
                .legend(legend.clone())
                .height(PLOT_HEIGHT)
                .line(
                    TimeseriesLine::new("accSmooth[0]").color(colors.triple_primary[0]),
                    times.iter().copied().zip(fd.acc_smooth.as_ref().map(|s| s.x().iter().copied()).unwrap_or_default())
                )
                .line(
                    TimeseriesLine::new("accSmooth[1]").color(colors.triple_primary[1]),
                    times.iter().copied().zip(fd.acc_smooth.as_ref().map(|s| s.y().iter().copied()).unwrap_or_default())
                )
                .line(
                    TimeseriesLine::new("accSmooth[2]").color(colors.triple_primary[2]),
                    times.iter().copied().zip(fd.acc_smooth.as_ref().map(|s| s.z().iter().copied()).unwrap_or_default())
                )
        );

        ui.heading("RC Commands");
        ui.add(
            TimeseriesPlot::new(&mut self.rc_plot)
                .group(timeseries_group)
                .legend(legend.clone())
                .height(PLOT_HEIGHT)
                .line(
                    TimeseriesLine::new("rcCommand[0]").color(colors.quad[0]),
                    times.iter().copied().zip(fd.rc_command.as_ref().map(|s| s[0].iter().copied()).unwrap_or_default())
                )
                .line(
                    TimeseriesLine::new("rcCommand[1]").color(colors.quad[1]),
                    times.iter().copied().zip(fd.rc_command.as_ref().map(|s| s[1].iter().copied()).unwrap_or_default())
                )
                .line(
                    TimeseriesLine::new("rcCommand[2]").color(colors.quad[2]),
                    times.iter().copied().zip(fd.rc_command.as_ref().map(|s| s[2].iter().copied()).unwrap_or_default())
                )
                .line(
                    TimeseriesLine::new("rcCommand[3]").color(colors.quad[3]),
                    times.iter().copied().zip(fd.rc_command.as_ref().map(|s| s[3].iter().copied()).unwrap_or_default())
                )
        );

        ui.heading("Motors");
        ui.add(
            TimeseriesPlot::new(&mut self.motor_plot)
                .group(timeseries_group)
                .legend(legend.clone())
                .height(PLOT_HEIGHT)
                .line(
                    TimeseriesLine::new("motor[0]").color(colors.motors[0]),
                    times.iter().copied().zip(fd.motor.as_ref().map(|s| s[0].iter().copied()).unwrap_or_default())
                )
                .line(
                    TimeseriesLine::new("motor[1]").color(colors.motors[1]),
                    times.iter().copied().zip(fd.motor.as_ref().map(|s| s[1].iter().copied()).unwrap_or_default())
                )
                .line(
                    TimeseriesLine::new("motor[2]").color(colors.motors[2]),
                    times.iter().copied().zip(fd.motor.as_ref().map(|s| s[2].iter().copied()).unwrap_or_default())
                )
                .line(
                    TimeseriesLine::new("motor[3]").color(colors.motors[3]),
                    times.iter().copied().zip(fd.motor.as_ref().map(|s| s[3].iter().copied()).unwrap_or_default())
                )
        );

        ui.heading("eRPM");
        ui.add(
            TimeseriesPlot::new(&mut self.erpm_plot)
                .group(timeseries_group)
                .legend(legend.clone())
                .height(PLOT_HEIGHT)
                .line(
                    TimeseriesLine::new("eRPM[0]").color(colors.motors[0]),
                    times.iter().copied().zip(fd.erpm.as_ref().map(|s| s[0].iter().copied()).unwrap_or_default())
                )
                .line(
                    TimeseriesLine::new("eRPM[1]").color(colors.motors[1]),
                    times.iter().copied().zip(fd.erpm.as_ref().map(|s| s[1].iter().copied()).unwrap_or_default())
                )
                .line(
                    TimeseriesLine::new("eRPM[2]").color(colors.motors[2]),
                    times.iter().copied().zip(fd.erpm.as_ref().map(|s| s[2].iter().copied()).unwrap_or_default())
                )
                .line(
                    TimeseriesLine::new("eRPM[3]").color(colors.motors[3]),
                    times.iter().copied().zip(fd.erpm.as_ref().map(|s| s[3].iter().copied()).unwrap_or_default())
                )
        );

        ui.heading("Battery");
        ui.add(
            TimeseriesPlot::new(&mut self.battery_plot)
                .group(timeseries_group)
                .legend(legend.clone())
                .height(PLOT_HEIGHT)
                .line(
                    TimeseriesLine::new("vbatLatest").color(colors.voltage),
                    times.iter().copied().zip(fd.battery_voltage.as_ref().map(|s| s.iter().copied()).unwrap_or_default())
                )
                .line(
                    TimeseriesLine::new("amperageLatest").color(colors.current),
                    times.iter().copied().zip(fd.battery_current.as_ref().map(|s| s.iter().copied()).unwrap_or_default())
                )
        );

        ui.heading("RSSI");
        ui.add(
            TimeseriesPlot::new(&mut self.rssi_plot)
                .group(timeseries_group)
                .legend(legend.clone())
                .height(PLOT_HEIGHT)
                .line(
                    TimeseriesLine::new("rssi").color(colors.rssi),
                    times.iter().copied().zip(fd.rssi.as_ref().map(|s| s.iter().copied()).unwrap_or_default())
                )
        );
    }
}
