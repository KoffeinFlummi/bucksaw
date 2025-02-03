use std::sync::mpsc::Sender;

use blackbox_log::headers::ParseError;

use crate::flight_data::FlightData;

pub struct LogFile {
    pub flights: Vec<Result<FlightData, ParseError>>,
}

impl LogFile {
    pub async fn parse(
        file_name: String,
        bytes: Vec<u8>,
        ctx: &egui::Context,
        file_progress_sender: Sender<f32>,
        flight_progress_sender: Sender<f32>,
    ) -> Self {
        ctx.request_repaint();

        let file = blackbox_log::File::new(&bytes);

        let mut flights = Vec::new();
        for (i, header) in file.iter().enumerate() {
            log::info!(
                "Parsing flight {}/{} of {}",
                i + 1,
                file.log_count(),
                file_name
            );

            match header {
                Ok(h) => {
                    let flight = FlightData::parse(i, h, ctx, flight_progress_sender.clone())
                        .await
                        .unwrap();
                    flights.push(Ok(flight));
                }
                Err(e) => {
                    flights.push(Err(e));
                }
            }

            let f = ((i + 1) as f32) / (file.log_count() as f32);
            file_progress_sender.send(f).unwrap();
            ctx.request_repaint();

            #[cfg(target_arch = "wasm32")]
            async_std::task::sleep(std::time::Duration::from_secs_f32(0.00001)).await;
        }

        Self { flights }
    }
}
