use std::collections::HashMap;
use std::sync::mpsc::Sender;

use blackbox_log::frame::Frame;
use blackbox_log::frame::FrameDef;
use blackbox_log::headers::DebugMode;
use blackbox_log::headers::Firmware;
use blackbox_log::headers::PwmProtocol;
use blackbox_log::units::FlagSet;

use crate::gui::blackbox_ui_ext::*;

#[allow(dead_code)]
#[derive(Clone)]
pub struct FlightData {
    pub index: usize,
    pub firmware: Firmware,
    pub firmware_date: Option<String>,
    pub board_info: Option<String>,
    pub craft_name: Option<String>,
    pub debug_mode: DebugMode,
    pub features: Vec<String>,
    pub esc_protocol: PwmProtocol,
    pub unknown_headers: HashMap<String, String>,
    pub times: Vec<f64>,
    pub main_values: HashMap<String, Vec<f32>>,
    pub main_units: HashMap<String, String>,
}

impl FlightData {
    pub async fn parse(
        index: usize,
        headers: blackbox_log::headers::Headers<'_>,
        progress_sender: Sender<f32>,
    ) -> Result<Self, ()> {
        let mut parser = headers.data_parser();

        let main_frame_defs: Vec<_> = parser.main_frame_def().iter().collect();

        let main_units = main_frame_defs
            .iter()
            .filter_map(|def| {
                let unit = match def.unit {
                    blackbox_log::frame::MainUnit::Amperage => Some("A"),
                    blackbox_log::frame::MainUnit::Voltage => Some("V"),
                    blackbox_log::frame::MainUnit::Acceleration => Some("m/s²"),
                    blackbox_log::frame::MainUnit::Rotation => Some("°/s"),
                    blackbox_log::frame::MainUnit::Unitless => None,
                };
                unit.map(|u| (def.name.to_string(), u.to_string()))
            })
            .collect();

        let mut times = Vec::new();
        let mut main_values: HashMap<String, Vec<f32>> = HashMap::new();
        let mut i = 0;

        while let Some(frame) = parser.next() {
            if let blackbox_log::ParserEvent::Main(frame) = frame {
                if frame.time().value < times.last().copied().unwrap_or_default() {
                    continue;
                }

                times.push(frame.time().value);

                for (def, value) in main_frame_defs.iter().zip(frame.iter()) {
                    let float = match value {
                        blackbox_log::frame::MainValue::Amperage(val) => val.value as f32,
                        blackbox_log::frame::MainValue::Voltage(val) => val.value as f32,
                        blackbox_log::frame::MainValue::Acceleration(val) => val.value as f32,
                        blackbox_log::frame::MainValue::Rotation(val) => {
                            val.value.to_degrees() as f32
                        }
                        blackbox_log::frame::MainValue::Unsigned(val) => val as f32,
                        blackbox_log::frame::MainValue::Signed(val) => val as f32,
                    };

                    let entry = main_values.entry(def.name.to_owned()).or_default();

                    entry.push(float);
                }
            }

            if i == 0 {
                progress_sender.send(parser.stats().progress).unwrap();
                #[cfg(target_arch = "wasm32")]
                async_std::task::sleep(std::time::Duration::from_secs_f32(0.00001)).await;
            }
            i = (i + 1) % 1000;
        }

        Ok(Self {
            index,
            firmware: headers.firmware(),
            firmware_date: headers
                .firmware_date()
                .and_then(|r| r.ok())
                .map(|dt| format!("{}", dt)),
            board_info: headers.board_info().map(|x| x.to_string()),
            craft_name: headers.craft_name().map(|x| x.to_string()),
            debug_mode: headers.debug_mode(),
            features: headers
                .features()
                .as_names()
                .iter()
                .map(|x| x.to_string())
                .collect(),
            esc_protocol: headers.pwm_protocol(),
            unknown_headers: headers
                .unknown()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            times,
            main_values,
            main_units,
        })
    }

    fn get_vector_series<const N: usize>(&self, series_name: &str) -> Option<[&Vec<f32>; N]> {
        (0..N)
            .map(|i| self.main_values.get(&format!("{}[{}]", series_name, i)))
            .collect::<Option<Vec<_>>>()
            .and_then(|v| v.try_into().ok())
    }

    pub fn gyro_unfiltered(&self) -> Option<[&Vec<f32>; 3]> {
        self.get_vector_series("gyroUnfilt")
    }

    pub fn gyro_filtered(&self) -> Option<[&Vec<f32>; 3]> {
        self.get_vector_series("gyroADC")
    }

    pub fn accel(&self) -> Option<[&Vec<f32>; 3]> {
        self.get_vector_series("accSmooth")
    }

    pub fn rc_command(&self) -> Option<[&Vec<f32>; 4]> {
        self.get_vector_series("rcCommand")
    }

    pub fn setpoint(&self) -> Option<[&Vec<f32>; 4]> {
        self.get_vector_series("setpoint")
    }

    pub fn p(&self) -> Option<[&Vec<f32>; 3]> {
        self.get_vector_series("axisP")
    }

    pub fn i(&self) -> Option<[&Vec<f32>; 3]> {
        self.get_vector_series("axisI")
    }

    // Note the type signature change here, we might not have D gains for all axes
    pub fn d(&self) -> [Option<&Vec<f32>>; 3] {
        (0..3)
            .map(|i| self.main_values.get(&format!("axisD[{}]", i)))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }

    pub fn f(&self) -> Option<[&Vec<f32>; 3]> {
        self.get_vector_series("axisF")
    }

    pub fn motor(&self) -> Option<Vec<&Vec<f32>>> {
        const N: usize = 4; // TODO
        (0..N)
            .map(|i| self.main_values.get(&format!("motor[{}]", i)))
            .collect::<Option<Vec<_>>>()
    }

    pub fn electrical_rpm(&self) -> Option<Vec<&Vec<f32>>> {
        const N: usize = 4; // TODO
        (0..N)
            .map(|i| self.main_values.get(&format!("eRPM[{}]", i)))
            .collect::<Option<Vec<_>>>()
    }

    pub fn battery_voltage(&self) -> Option<&Vec<f32>> {
        self.main_values.get("vbatLatest")
    }

    pub fn amperage(&self) -> Option<&Vec<f32>> {
        self.main_values.get("amperageLatest")
    }

    pub fn rssi(&self) -> Option<&Vec<f32>> {
        self.main_values.get("rssi")
    }

    // TODO: there's gotta be a better way to do this
    pub fn sample_rate(&self) -> f64 {
        const NUM_SAMPLES: usize = 100;
        let mut samples: Vec<u32> = self
            .times
            .windows(2)
            .map(|w| ((w[1] - w[0]) * 1_000_000.0) as u32)
            .take(NUM_SAMPLES)
            .collect();
        samples.sort();
        let sample_interval = samples[NUM_SAMPLES / 2];
        let rate = 1_000_000.0 / (sample_interval as f64);
        (rate / 100.0).round() * 100.0
    }

    pub fn show(&self, ui: &mut egui::Ui) -> bool {
        egui::Grid::new(ui.next_auto_id())
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                ui.label("FW");
                self.firmware.show(ui);
                ui.end_row();

                ui.label("Board");
                ui.label(self.board_info.clone().unwrap_or_default());
                ui.end_row();

                ui.label("Craft");
                ui.label(self.craft_name.clone().unwrap_or_default());
                ui.end_row();

                ui.label("Duration");
                if let Some(duration) = self
                    .times
                    .first()
                    .and_then(|f| self.times.last().map(|l| l - f))
                {
                    let freq = (self.times.len() as f64) / duration;
                    ui.label(format!("{:.3}s (~{:.0}Hz)", duration, freq));
                } else {
                    ui.label("");
                }
                ui.end_row();
            });

        false
    }
}
