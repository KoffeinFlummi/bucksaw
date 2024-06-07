use std::f64::consts::PI;
use std::sync::mpsc::{channel, Receiver, TryRecvError};

use egui::Color32;
use egui_oszi::TimeseriesGroup;
use num_complex::Complex;

use crate::flight_data::FlightData;
use crate::utils::execute;

const FFT_SIZE: usize = 512;
const FFT_OVERLAP_DENOMINATOR: usize = 32;

const COLORGRAD_LOOKUP_SIZE: usize = 128;
const TIME_DOMAIN_TEX_WIDTH: usize = 1024;
const THROTTLE_DOMAIN_BUCKETS: usize = 256;

#[derive(PartialEq, Clone, Copy)]
enum VibeDomain {
    Time,
    Throttle,
}

#[derive(Clone)]
struct FftChunk {
    time: f64,
    fft: [f64; FFT_SIZE/2], // only real signals :(
    throttle: f64,
}

impl FftChunk {
    pub fn calculate(time: f64, data: &[f64], throttle: f64) -> Option<Self> {
        if data.len() < FFT_SIZE {
            return None;
        }

        // convert to complex and apply hann window
        let mut data: Vec<Complex<f64>> = data
            .into_iter()
            .enumerate()
            .map(|(i, r)| {
                let window = 0.5 * (1.0 - (2.0 * PI * (i as f64) / (FFT_SIZE as f64)).cos());
                Complex::new(*r * window, 0.0)
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let mut planner = rustfft::FftPlanner::<f64>::new();
        let plan = planner.plan_fft_forward(FFT_SIZE);
        plan.process(&mut data);

        let fft = data[data.len()/2..]
            .iter()
            .map(|c| c.re.log10())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        Some(Self {
            time,
            fft,
            throttle,
        })
    }
}

struct FftAxis {
    ctx: egui::Context,

    chunks: Vec<FftChunk>,
    chunk_receiver: Option<Receiver<FftChunk>>,

    time_textures: Vec<(f64, egui::TextureHandle)>,
    time_texture_receiver: Option<Receiver<(f64, egui::TextureHandle)>>,
    throttle_texture: Option<egui::TextureHandle>,
    throttle_texture_receiver: Option<Receiver<egui::TextureHandle>>,

    colorgrad_lookup: [egui::Color32; COLORGRAD_LOOKUP_SIZE],
}

impl FftAxis {
    pub fn new(
        ctx: &egui::Context,
        time: Vec<f64>,
        data: Vec<f64>,
        throttle: Vec<f64>
    ) -> Self {
        let (chunk_sender, chunk_receiver) = channel();

        let time = time.clone();
        let throttle = throttle.clone();

        execute(async move {
            let time_chunks: Vec<_> = time.chunks(FFT_SIZE/FFT_OVERLAP_DENOMINATOR).collect();
            let data_chunks: Vec<_> = data.chunks(FFT_SIZE/FFT_OVERLAP_DENOMINATOR).collect();
            let throttle_chunks: Vec<_> = throttle.chunks(FFT_SIZE/FFT_OVERLAP_DENOMINATOR).collect();
            let time_windows = time_chunks
                .windows(FFT_OVERLAP_DENOMINATOR)
                .map(|window| window.into_iter().map(|c| c.into_iter()).flatten().copied().collect::<Vec<_>>());
            let data_windows = data_chunks
                .windows(FFT_OVERLAP_DENOMINATOR)
                .map(|window| window.into_iter().map(|c| c.into_iter()).flatten().copied().collect::<Vec<_>>());
            let throttle_windows = throttle_chunks
                .windows(FFT_OVERLAP_DENOMINATOR)
                .map(|window| window.into_iter().map(|c| c.into_iter()).flatten().copied().collect::<Vec<_>>());

            for (time, (data, throttle)) in time_windows.zip(data_windows.zip(throttle_windows)) {
                if let Some(chunk) = FftChunk::calculate(time[0], &data, throttle[throttle.len()/2]) {
                    chunk_sender.send(chunk).unwrap();
                }
            }
        });

        Self {
            ctx: ctx.clone(),

            chunks: Vec::new(),
            chunk_receiver: Some(chunk_receiver),
            time_textures: Vec::new(),
            time_texture_receiver: None,
            throttle_texture: None,
            throttle_texture_receiver: None,

            colorgrad_lookup: (0..COLORGRAD_LOOKUP_SIZE)
                .map(move |i| {
                    let f = (i as f64) / (COLORGRAD_LOOKUP_SIZE as f64);
                    let rgba = colorgrad::inferno().at(f).to_rgba8();
                    egui::Color32::from_rgb(rgba[0], rgba[1], rgba[2])
                })
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        }
    }

    pub fn recreate_textures(&mut self) {
        let (time_texture_sender, time_texture_receiver) = channel();
        let (throttle_texture_sender, throttle_texture_receiver) = channel();

        let chunks = self.chunks.clone();
        let colorgrad_lookup = self.colorgrad_lookup.clone();
        let ctx = self.ctx.clone();
        execute(async move {
            let max = 0.75 * chunks.iter()
                .map(|chunk| chunk.fft.iter().fold(f64::NEG_INFINITY, |a, b| f64::max(a, *b)))
                .fold(f64::NEG_INFINITY, |a, b| f64::max(a, b));

            for (i, columns) in chunks.chunks(TIME_DOMAIN_TEX_WIDTH).enumerate() {
                let mut image = egui::ColorImage::new([columns.len(), FFT_SIZE/2], Color32::TRANSPARENT);

                for x in 0..columns.len() {
                    for y in 0..columns[x].fft.len() {
                        let val = columns[x].fft[y];
                        let f = f64::max(0.0, val) / max;
                        let i = (f * (COLORGRAD_LOOKUP_SIZE as f64)) as usize;
                        let i = usize::min(i, COLORGRAD_LOOKUP_SIZE - 1);
                        image[(x, y)] = colorgrad_lookup[i];
                    }
                }

                let tex_name = format!("tex_{:?}", i);
                let tex_handle = ctx.load_texture(tex_name, image, Default::default());
                time_texture_sender.send((columns[0].time, tex_handle)).unwrap();
                ctx.request_repaint();
            }
        });

        let chunks = self.chunks.clone();
        let colorgrad_lookup = self.colorgrad_lookup.clone();
        let ctx = self.ctx.clone();
        execute(async move {
            const ARRAY_REPEAT_VALUE: std::vec::Vec<FftChunk> = Vec::new();
            let mut throttle_buckets: [Vec<FftChunk>; THROTTLE_DOMAIN_BUCKETS] = [ARRAY_REPEAT_VALUE; THROTTLE_DOMAIN_BUCKETS];
            for chunk in chunks {
                let bucket_i = ((chunk.throttle / 1000.0) * THROTTLE_DOMAIN_BUCKETS as f64) as usize;
                let bucket_i = usize::min(bucket_i, THROTTLE_DOMAIN_BUCKETS - 1);
                throttle_buckets[bucket_i].push(chunk);
            }

            let mut throttle_averages = Vec::new();
            for bucket in throttle_buckets.into_iter() {
                let size = bucket.len();
                let avg: [f64; FFT_SIZE/2] = bucket.into_iter()
                    .map(|chunk| chunk.fft)
                    .fold([0f64; FFT_SIZE/2], |a, b| {
                        a.into_iter()
                            .zip(b.into_iter())
                            .map(|(a, b)| {
                                if a.is_normal() && b.is_normal() {
                                    a+b
                                } else if a.is_normal() {
                                    a
                                } else {
                                    b
                                }
                            })
                            .collect::<Vec<_>>()
                            .try_into()
                            .unwrap()
                    })
                    .into_iter()
                    .map(|v| v / (size as f64))
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap();
                throttle_averages.push(avg);
            }

            let max = 0.75 * throttle_averages.iter()
                .map(|chunk| chunk.iter().fold(f64::NEG_INFINITY, |a, b| f64::max(a, *b)))
                .fold(f64::NEG_INFINITY, |a, b| f64::max(a, b));

            let mut image = egui::ColorImage::new([THROTTLE_DOMAIN_BUCKETS, FFT_SIZE/2], Color32::TRANSPARENT);

            for x in 0..THROTTLE_DOMAIN_BUCKETS {
                for y in 0..FFT_SIZE/2 {
                    let val = throttle_averages[x][y];
                    let f = f64::max(0.0, val) / max;
                    let i = (f * (COLORGRAD_LOOKUP_SIZE as f64)) as usize;
                    let i = usize::min(i, COLORGRAD_LOOKUP_SIZE - 1);
                    image[(x, y)] = colorgrad_lookup[i];
                }
            }

            let tex_handle = ctx.load_texture("throttle_fft", image, Default::default());
            throttle_texture_sender.send(tex_handle).unwrap();
            ctx.request_repaint();
        });

        self.time_texture_receiver = Some(time_texture_receiver);
        self.throttle_texture_receiver = Some(throttle_texture_receiver);
    }

    pub fn process_updates(&mut self) {
        let chunks_done = if let Some(receiver) = &self.chunk_receiver {
            loop {
                match receiver.try_recv() {
                    Ok(chunk) => { self.chunks.push(chunk); },
                    Err(TryRecvError::Empty) => { break false; }
                    Err(TryRecvError::Disconnected) => { break true; }
                }
            }
        } else {
            false
        };

        if chunks_done {
            self.chunk_receiver = None;
            self.recreate_textures();
        }

        if let Some(receiver) = &self.time_texture_receiver {
            loop {
                match receiver.try_recv() {
                    Ok((t, tex)) => { self.time_textures.push((t, tex)); },
                    Err(_) => { break; }
                }
            }
        }

        if let Some(receiver) = &self.throttle_texture_receiver {
            while let Ok(texture) = receiver.try_recv() {
                self.throttle_texture = Some(texture);
            }
        }
    }

    pub fn show_time(&mut self, ui: &mut egui::Ui) {
        egui_plot::Plot::new(ui.next_auto_id())
            .legend(egui_plot::Legend::default())
            .set_margin_fraction(egui::Vec2::new(0.0, 0.0))
            .show_grid(false)
            .allow_drag([true, false])
            .allow_zoom([true, false])
            .allow_scroll(false)
            .include_y(0.0)
            .include_y(1.0)
            .link_axis("time_vibes", true, true)
            .link_cursor("time_vibes", true, true)
            .height(ui.available_height())
            .show(ui, |plot_ui| {
                let duration = self.time_textures.windows(2).next().map(|w| w[1].0 - w[0].0).unwrap_or(1.0);

                for (t, texture) in self.time_textures.iter() {
                    let plot_image = egui_plot::PlotImage::new(
                        texture,
                        egui_plot::PlotPoint::new(*t + duration / 2.0, 0.5),
                        egui::Vec2::new(duration as f32, 1.0),
                    );

                    plot_ui.image(plot_image);
                }
            })
            .response;
    }

    pub fn show_throttle(&mut self, ui: &mut egui::Ui) {
        egui_plot::Plot::new(ui.next_auto_id())
            .legend(egui_plot::Legend::default())
            .set_margin_fraction(egui::Vec2::new(0.0, 0.0))
            .show_grid(false)
            .allow_drag(false)
            .allow_zoom(false)
            .allow_scroll(false)
            .include_y(0.0)
            .include_y(1.0)
            .link_axis("throttle_vibes", true, true)
            .link_cursor("throttle_vibes", true, true)
            .height(ui.available_height())
            .show(ui, |plot_ui| {
                if let Some(texture) = self.throttle_texture.as_mut() {
                    let plot_image = egui_plot::PlotImage::new(
                        texture,
                        egui_plot::PlotPoint::new(0.5, 0.5),
                        egui::Vec2::new(1.0, 1.0),
                    );

                    plot_ui.image(plot_image);
                }
            })
            .response;
    }

    pub fn show(&mut self, ui: &mut egui::Ui, domain: VibeDomain) {
        self.process_updates();

        match domain {
            VibeDomain::Time => self.show_time(ui),
            VibeDomain::Throttle => self.show_throttle(ui)
        }
    }
}

struct FftVectorSeries {
    axes: [FftAxis; 3]
}

impl FftVectorSeries {
    pub fn new(ctx: &egui::Context, time: Vec<f64>, data: [Vec<f64>; 3], throttle: Vec<f64>) -> Self {
        let axes = [
            FftAxis::new(ctx, time.clone(), data[0].clone(), throttle.clone()),
            FftAxis::new(ctx, time.clone(), data[1].clone(), throttle.clone()),
            FftAxis::new(ctx, time, data[2].clone(), throttle)
        ];

        Self { axes }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, domain: VibeDomain) {
        for (i, axis) in self.axes.iter_mut().enumerate() {
            ui.vertical(|ui| {
                ui.set_height(ui.available_height() / (3 - i) as f32);
                axis.show(ui, domain);
            });
        }
    }
}

pub struct VibeTab {
    domain: VibeDomain,

    //gyro_raw_enabled: bool,
    //gyro_filtered_enabled: bool,
    //dterm_raw_enabled: bool,
    //dterm_filtered_enabled: bool,

    gyro_raw_ffts: Option<FftVectorSeries>,
    gyro_filtered_ffts: Option<FftVectorSeries>,
    //dterm_raw_ffts: Option<FftVectorSeries>,
    //dterm_filtered_ffts: Option<FftVectorSeries>,
}

impl VibeTab {
    pub fn new() -> Self {
        Self {
            domain: VibeDomain::Time,

            //gyro_raw_enabled: true,
            //gyro_filtered_enabled: true,
            //dterm_raw_enabled: false,
            //dterm_filtered_enabled: false,

            gyro_raw_ffts: None,
            gyro_filtered_ffts: None,
            //dterm_raw_ffts: None,
            //dterm_filtered_ffts: None,
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        fd: &FlightData,
        _timeseries_group: &mut TimeseriesGroup
    ) {
        ui.horizontal(|ui| {
            ui.label("Domain:");
            ui.selectable_value(&mut self.domain, VibeDomain::Time, "🕙 Time");
            ui.selectable_value(&mut self.domain, VibeDomain::Throttle, "🏃 Throttle");

            //ui.separator();

            //ui.label("Series:");
            //ui.toggle_value(&mut self.gyro_raw_enabled, "Gyro (raw)");
            //ui.toggle_value(&mut self.gyro_filtered_enabled, "Gyro (filtered)");
            //ui.toggle_value(&mut self.dterm_raw_enabled, "D term (raw)");
            //ui.toggle_value(&mut self.dterm_filtered_enabled, "D term (filtered)");
        });

        ui.separator();

        if self.gyro_raw_ffts.is_none() {
            self.gyro_raw_ffts = Some(FftVectorSeries::new(ui.ctx(), fd.times.clone(), fd.gyro_unfilt.clone().unwrap().0, fd.setpoint.as_ref().unwrap()[3].clone())); // TODO: unwrap
        }

        if self.gyro_filtered_ffts.is_none() {
            self.gyro_filtered_ffts = Some(FftVectorSeries::new(ui.ctx(), fd.times.clone(), fd.gyro_adc.clone().unwrap().0, fd.setpoint.as_ref().unwrap()[3].clone())); // TODO: unwrap
        }

        let Some(gyro_raw_ffts) = self.gyro_raw_ffts.as_mut() else { return };
        let Some(gyro_filtered_ffts) = self.gyro_filtered_ffts.as_mut() else { return };

        ui.columns(2, |columns| {
            columns[0].heading("Gyro (raw)");
            gyro_raw_ffts.show(&mut columns[0], self.domain);

            columns[1].heading("Gyro (filtered)");
            gyro_filtered_ffts.show(&mut columns[1], self.domain);
        });
    }
}
