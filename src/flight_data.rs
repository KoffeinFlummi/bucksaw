use std::sync::mpsc::Sender;
use std::collections::HashMap;

use blackbox_log::frame::Frame;
use blackbox_log::frame::FrameDef;
use blackbox_log::headers::DebugMode;
use blackbox_log::headers::Firmware;
use blackbox_log::headers::PwmProtocol;
use blackbox_log::units::FlagSet;

use crate::gui::blackbox_ui_ext::*;

// Since values are usually accessed by axis and not by time,
// we don't use a vector type here and store all axes side by side.
#[derive(Clone, Debug)]
pub struct VectorSeries<T, const N: usize>(pub [Vec<T>; N]);

impl<T, const N: usize> std::ops::Deref for VectorSeries<T, N> {
    type Target = [Vec<T>; N];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, const N: usize> VectorSeries<T, N> {
}

impl<T> VectorSeries<T, 3> {
    pub fn x<'a>(&'a self) -> &'a Vec<T> {
        &self.0[0]
    }

    pub fn y<'a>(&'a self) -> &'a Vec<T> {
        &self.0[1]
    }

    pub fn z<'a>(&'a self) -> &'a Vec<T> {
        &self.0[2]
    }
}

#[derive(Clone)]
pub struct FlightData {
    pub index: usize,

    // Log file metadata
    pub firmware: Firmware,
    pub firmware_date: Option<String>,
    pub board_info: Option<String>,
    pub craft_name: Option<String>,
    pub debug_mode: DebugMode,
    pub features: Vec<String>,
    pub esc_protocol: PwmProtocol,
    pub unknown_headers: HashMap<String, String>,

    // Known timeseries
    pub times: Vec<f64>,
    pub acc_smooth: Option<VectorSeries<f32, 3>>,
    pub gyro_unfilt: Option<VectorSeries<f32, 3>>,
    pub gyro_adc: Option<VectorSeries<f32, 3>>,
    pub battery_voltage: Option<Vec<f32>>,
    pub battery_current: Option<Vec<f32>>,
    pub rssi: Option<Vec<f32>>,

    pub rc_command: Option<VectorSeries<f32, 4>>,
    pub setpoint: Option<VectorSeries<f32, 4>>,

    pub p: Option<VectorSeries<f32, 3>>,
    pub i: Option<VectorSeries<f32, 3>>,
    pub d: Option<VectorSeries<f32, 3>>, // most of the time yaw does not have a d gain, but it is possible
    pub f: Option<VectorSeries<f32, 3>>,

    // TODO: num motors
    pub motor: Option<VectorSeries<f32, 4>>,
    pub erpm: Option<VectorSeries<f32, 4>>,

    // Other values included in log
    pub main_values: HashMap<String, Vec<f64>>,
    pub main_units: HashMap<String, String>,
}

impl FlightData {
    fn try_extract_vector<const N: usize>(&mut self, key: &'static str) -> Option<VectorSeries<f32, N>> {
        let keys: Vec<String> = (0..N)
            .map(|i| format!("{}[{}]", key, i))
            .collect();
        if keys.iter().any(|k| self.main_values.contains_key(k)) {
            let values: Vec<_> = keys.iter()
                .map(|k| self.main_values.get(k).map(|vec| vec.iter().map(|v| *v as f32).collect()).unwrap_or_default())
                .collect();
            let series: [Vec<f32>; N] = values.try_into().unwrap();
            Some(VectorSeries(series))
        } else {
            None
        }
    }

    pub fn extract_known_values(&mut self) {
        self.acc_smooth = self.try_extract_vector("accSmooth");
        self.gyro_unfilt = self.try_extract_vector("gyroUnfilt");
        self.gyro_adc = self.try_extract_vector("gyroADC");
        self.rc_command = self.try_extract_vector("rcCommand");
        self.setpoint = self.try_extract_vector("setpoint");
        self.p = self.try_extract_vector("axisP");
        self.i = self.try_extract_vector("axisI");
        self.d = self.try_extract_vector("axisD");
        self.f = self.try_extract_vector("axisF");
        self.motor = self.try_extract_vector("motor");
        self.erpm = self.try_extract_vector("eRPM");

        self.battery_voltage = self.main_values.get("vbatLatest").map(|vec| vec.iter().map(|v| *v as f32).collect());
        self.battery_current = self.main_values.get("amperageLatest").map(|vec| vec.iter().map(|v| *v as f32).collect());
        self.rssi = self.main_values.get("rssi").map(|vec| vec.iter().map(|v| *v as f32).collect());
    }

    pub async fn parse(
        index: usize,
        headers: blackbox_log::headers::Headers<'_>,
        ctx: &egui::Context,
        progress_sender: Sender<f32>
    ) -> Result<Self, ()> {
        let mut parser = headers.data_parser();

        let main_frame_defs: Vec<_> = parser.main_frame_def().iter().collect();
        let _slow_frame_defs: Vec<_> = parser.slow_frame_def().iter().collect();
        let _gps_frame_defs: Option<Vec<_>> = parser.gps_frame_def().map(|defs| defs.iter().collect());

        let main_units: HashMap<String, String> = main_frame_defs
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
        let mut main_values = HashMap::new();
        let mut i = 0;
        while let Some(next) = parser.next() {
            match next {
                blackbox_log::ParserEvent::Event(_event) => {
                    //println!("{}: {:?}", times.last().copied().unwrap_or_default(), event);
                },
                blackbox_log::ParserEvent::Main(frame) => {
                    if frame.time().value < times.last().map(|l| *l).unwrap_or_default() {
                        continue;
                    }

                    times.push(frame.time().value);

                    for (def, value) in main_frame_defs.iter().zip(frame.iter()) {
                        let float = match value {
                            blackbox_log::frame::MainValue::Amperage(val) => val.value,
                            blackbox_log::frame::MainValue::Voltage(val) => val.value,
                            blackbox_log::frame::MainValue::Acceleration(val) => val.value,
                            blackbox_log::frame::MainValue::Rotation(val) => val.value.to_degrees(),
                            blackbox_log::frame::MainValue::Unsigned(val) => val as f64,
                            blackbox_log::frame::MainValue::Signed(val) => val as f64,
                        };

                        if !main_values.contains_key(def.name) {
                            main_values.insert(def.name.to_string(), Vec::new());
                        }

                        main_values
                            .get_mut(def.name)
                            .unwrap()
                            .push(float);
                    }
                },
                blackbox_log::ParserEvent::Slow(_frame) => {},
                blackbox_log::ParserEvent::Gps(_frame) => {},
            }

            if i == 0 {
                progress_sender.send(parser.stats().progress).unwrap();
                ctx.request_repaint();
                #[cfg(target_arch = "wasm32")]
                async_std::task::sleep(std::time::Duration::from_secs_f32(0.00001)).await;
            }
            i = (i + 1) % 1000;
        }

        let mut flight_data = Self {
            index,
            firmware: headers.firmware(),
            firmware_date: headers.firmware_date().map(|r| r.ok()).flatten().map(|dt| format!("{}", dt)),
            board_info: headers.board_info().map(|x| x.to_string()),
            craft_name: headers.craft_name().map(|x| x.to_string()),
            debug_mode: headers.debug_mode(),
            features: headers.features().as_names().iter().map(|x| x.to_string()).collect(),
            esc_protocol: headers.pwm_protocol(),
            unknown_headers: headers.unknown().iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            times,
            acc_smooth: None,
            gyro_unfilt: None,
            gyro_adc: None,
            rc_command: None,
            battery_voltage: None,
            battery_current: None,
            rssi: None,
            setpoint: None,
            p: None,
            i: None,
            d: None,
            f: None,
            motor: None,
            erpm: None,
            main_values,
            main_units,
        };
        flight_data.extract_known_values();

        Ok(flight_data)
    }

    // TODO: there's gotta be a better way to do this
    pub fn sample_rate(&self) -> f64 {
        const NUM_SAMPLES: usize = 100;
        let mut samples: Vec<u32> = self.times
            .windows(2)
            .map(|w| ((w[1] - w[0]) * 1_000_000.0) as u32)
            .take(NUM_SAMPLES)
            .collect();
        samples.sort();
        let sample_interval = samples[NUM_SAMPLES/2];
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
                if let Some(duration) = self.times.first().and_then(|f| self.times.last().map(|l| l - f)) {
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
