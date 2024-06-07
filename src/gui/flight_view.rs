use egui_oszi::TimeseriesGroup;

use crate::flight_data::FlightData;
use crate::gui::tabs::*;

pub struct FlightView {
    pub data: FlightData,
    plot_group: TimeseriesGroup,
    plot_tab: PlotTab,
    tune_tab: TuneTab,
    vibe_tab: VibeTab,
}

impl FlightView {
    pub fn new(data: FlightData) -> Self {
        Self {
            data,
            plot_group: TimeseriesGroup::new("timeseries_plots", false),
            plot_tab: PlotTab::new(),
            tune_tab: TuneTab::new(),
            vibe_tab: VibeTab::new(),
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, tab: FlightViewTab) {
        ui.vertical(|ui| {
            match tab {
                FlightViewTab::Plot => {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        self.plot_tab.show(ui, &self.data, &mut self.plot_group)
                    });
                }
                FlightViewTab::Tune => self.tune_tab.show(ui, &self.data, &mut self.plot_group),
                FlightViewTab::Vibe => self.vibe_tab.show(ui, &self.data, &mut self.plot_group),
            }
        });
    }
}
