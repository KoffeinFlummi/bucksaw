use std::sync::mpsc::Sender;

use blackbox_log::headers::ParseError;

use crate::flight_data::FlightData;

pub struct LogFile {
    pub flights: Vec<Result<FlightData, ParseError>>,
}

impl LogFile {
    pub fn parse(
        file_name: String,
        bytes: Vec<u8>,
        ctx: &egui::Context,
        file_progress_sender: Sender<f32>,
        flight_progress_sender: Sender<f32>
    ) -> Self {
        ctx.request_repaint();

        let file = blackbox_log::File::new(&bytes);

        let mut flights = Vec::new();
        for (i, header) in file.iter().enumerate() {
            log::info!("Parsing flight {}/{} of {}", i+1, file.log_count(), file_name);

            let flight = header.map(|h|
                FlightData::parse(
                    i,
                    h,
                    ctx,
                    flight_progress_sender.clone()
                ).unwrap()
            );
            flights.push(flight);

            let f = ((i+1) as f32) / (file.log_count() as f32);
            file_progress_sender.send(f).unwrap();
            ctx.request_repaint();
        }

        Self {
            flights,
        }

    }
}
