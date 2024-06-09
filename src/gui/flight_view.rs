use std::sync::Arc;

use egui_oszi::TimeseriesGroup;

use crate::flight_data::FlightData;
use crate::gui::tabs::*;

pub struct FlightView {
    pub data: Arc<FlightData>,
    plot_group: TimeseriesGroup,
    plot_tab: PlotTab,
    tune_tab: TuneTab,
    vibe_tab: VibeTab,
}

impl FlightView {
    pub fn new(ctx: &egui::Context, data: FlightData) -> Self {
        let data = Arc::new(data);

        Self {
            plot_tab: PlotTab::new(),
            tune_tab: TuneTab::new(ctx, data.clone()),
            vibe_tab: VibeTab::new(ctx, data.clone()),
            data,
            plot_group: TimeseriesGroup::new("timeseries_plots", false),
        }
    }

    pub fn set_flight(&mut self, flight: FlightData) {
        self.data = Arc::new(flight);
        self.tune_tab.set_flight(self.data.clone());
        self.vibe_tab.set_flight(self.data.clone());
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
