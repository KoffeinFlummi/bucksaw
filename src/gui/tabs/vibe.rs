use std::f32::consts::PI;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::{Arc, OnceLock};

use itertools::Itertools;

use egui::{Color32, DragValue};
use egui_oszi::TimeseriesGroup;

use crate::flight_data::FlightData;
use crate::gui::flex::*;
use crate::utils::execute;
use crate::iter::IterExt;

const COLORGRAD_LOOKUP_SIZE: usize = 128;
const TIME_DOMAIN_TEX_WIDTH: usize = 1024;
const THROTTLE_DOMAIN_BUCKETS: usize = 256;
const FFT_SIZE_OPTIONS: [usize; 4] = [256, 512, 1024, 2048];
const MIN_WIDE_WIDTH: f32 = 1000.0;

#[derive(PartialEq, Clone, Copy)]
enum VibeDomain {
    Time,
    Throttle,
}

#[derive(PartialEq, Clone, Copy, Default, Debug)]
enum Colorscheme {
    Turbo,
    Viridis,
    #[default]
    Inferno,
}

impl Into<colorgrad::Gradient> for Colorscheme {
    fn into(self) -> colorgrad::Gradient {
        match self {
            Self::Turbo => colorgrad::turbo(),
            Self::Viridis => colorgrad::viridis(),
            Self::Inferno => colorgrad::inferno(),
        }
    }
}

#[derive(PartialEq, Clone)]
struct FftSettings {
    pub size: usize,
    pub step_size: usize,
    pub plot_colorscheme: Colorscheme,
    pub plot_max: f32,
    color_lookup_table: Option<(Colorscheme, [egui::Color32; COLORGRAD_LOOKUP_SIZE])>,
}

impl FftSettings {
    fn color_lookup_table<'a>(&'a mut self) -> &'a [egui::Color32; COLORGRAD_LOOKUP_SIZE] {
        if self.color_lookup_table.map(|(t, _)| t != self.plot_colorscheme).unwrap_or(true) {
            let gradient: colorgrad::Gradient = self.plot_colorscheme.into();
            let table = (0..COLORGRAD_LOOKUP_SIZE)
                .map(move |i| {
                    let f = (i as f64) / (COLORGRAD_LOOKUP_SIZE as f64);
                    let rgba = gradient.at(f).to_rgba8();
                    egui::Color32::from_rgb(rgba[0], rgba[1], rgba[2])
                })
                .collect::<Vec<_>>()
                .try_into()
                .unwrap();

            self.color_lookup_table = Some((self.plot_colorscheme, table));
        }

        self.color_lookup_table.as_ref().map(|(_, t)| t).unwrap()
    }

    pub fn color_at(&mut self, f: f32) -> egui::Color32 {
        let i = (f * (COLORGRAD_LOOKUP_SIZE as f32)) as usize;
        let i = usize::min(i, COLORGRAD_LOOKUP_SIZE - 1);
        self.color_lookup_table()[i]
    }

    pub fn needs_recalculating(&self, other: &Self) -> bool {
        self.size != other.size || self.step_size != other.step_size
    }

    pub fn needs_redrawing(&self, other: &Self) -> bool {
        self.needs_recalculating(other) ||
            self.plot_colorscheme != other.plot_colorscheme ||
            self.plot_max != other.plot_max
    }
}

impl Default for FftSettings {
    fn default() -> Self {
        Self {
            size: 256,
            step_size: 8,
            plot_colorscheme: Colorscheme::default(),
            plot_max: 10.0,
            color_lookup_table: None,
        }
    }
}

#[derive(Clone)]
struct FftChunk {
    time: f64,
    fft: Vec<f32>,
    throttle: f32,
}

impl FftChunk {
    pub fn hamming_window(fft_size: usize) -> &'static [f32] {
        // TODO
        static LOOKUP: OnceLock<[Vec<f32>; FFT_SIZE_OPTIONS.len()]> = OnceLock::new();
        let lookup = LOOKUP.get_or_init(|| {
            FFT_SIZE_OPTIONS
                .into_iter()
                .map(|fft_size| {
                    (0..fft_size)
                        .map(|i| 0.53836 * (1.0 - (2.0 * PI * (i as f32) / (fft_size as f32)).cos()))
                        .collect()
                })
                .collect::<Vec<_>>()
                .try_into()
                .unwrap()

        });

        &lookup[FFT_SIZE_OPTIONS.iter().position(|s| *s == fft_size).unwrap()]
    }

    pub fn calculate(time: f64, data: &[f32], throttle: f32) -> Self {
        // convert to complex and apply hamming window
        let window = Self::hamming_window(data.len());
        let mut input: Vec<_> = data.iter().zip(window.iter()).map(|(d, w)| d * w).collect();

        let planner = realfft::RealFftPlanner::<f32>::new()
            .plan_fft_forward(data.len());
        let mut output = planner.make_output_vec();
        planner.process(&mut input, &mut output).unwrap();

        let fft = output
            .into_iter()
            .rev()
            .map(|c| (c.re.powi(2) + c.im.powi(2)).log10())
            .collect();

        Self {
            time,
            fft,
            throttle,
        }
    }
}

struct FftAxis {
    ctx: egui::Context,
    fft_settings: FftSettings,

    i: usize,
    flight_data: Arc<FlightData>,
    value_callback: Box<fn(&FlightData) -> &[Vec<f32>; 3]>,

    chunks: Vec<FftChunk>,
    chunk_receiver: Option<Receiver<Vec<FftChunk>>>,
    time_textures: Vec<(f64, f64, egui::TextureHandle)>,
    time_texture_receiver: Option<Receiver<(f64, f64, egui::TextureHandle)>>,
    throttle_texture: Option<egui::TextureHandle>,
    throttle_texture_receiver: Option<Receiver<egui::TextureHandle>>,
}

impl FftAxis {
    pub fn new(
        ctx: &egui::Context,
        fft_settings: FftSettings,
        i: usize,
        flight_data: Arc<FlightData>,
        value_callback: fn(&FlightData) -> &[Vec<f32>; 3]
    ) -> Self {
        let mut new = Self {
            ctx: ctx.clone(),
            fft_settings,

            i,
            flight_data,
            value_callback: Box::new(value_callback),

            chunks: Vec::new(),
            chunk_receiver: None,
            time_textures: Vec::new(),
            time_texture_receiver: None,
            throttle_texture: None,
            throttle_texture_receiver: None,
        };
        new.recalculate_ffts();
        new
    }

    pub fn create_image(data: &[FftChunk], max: f32, fft_settings: &mut FftSettings) -> egui::ColorImage {
        let mut image = egui::ColorImage::new([data.len(), data[0].fft.len()], Color32::TRANSPARENT);

        for x in 0..data.len() {
            for y in 0..data[x].fft.len() {
                let val = data[x].fft[y];
                image[(x, y)] = fft_settings.color_at(f32::max(0.0, val) / max);
            }
        }

        image
    }

    pub fn recalculate_ffts(&mut self) {
        let (chunk_sender, chunk_receiver) = channel();

        self.chunks.truncate(0);
        self.chunk_receiver = Some(chunk_receiver);
        self.time_textures.truncate(0);
        self.throttle_texture = None;

        let fd = self.flight_data.clone();
        let cb = self.value_callback.clone();
        let i = self.i;
        let fft_size = self.fft_settings.size;
        let fft_step_size = self.fft_settings.step_size;
        let ctx = self.ctx.clone();
        execute(async move {
            let throttle = &fd.setpoint.as_ref().unwrap()[3];
            let values = &(cb(&fd))[i];

            let time_windows = fd.times.iter().copied().overlapping_windows(fft_size, fft_step_size);
            let data_windows = values.iter().copied().overlapping_windows(fft_size, fft_step_size);
            let throttle_windows = throttle.iter().copied().overlapping_windows(fft_size, fft_step_size);

            time_windows.zip(data_windows.zip(throttle_windows))
                .filter(|(time, _)| time.len() == fft_size)
                .map(|(time, (data, throttle))| FftChunk::calculate(time[0], &data, throttle[throttle.len()/2]))
                .chunks(100)
                .into_iter()
                .for_each(|chunks| {
                    chunk_sender.send(chunks.collect()).unwrap();
                    ctx.request_repaint();
                });
        });
    }

    pub fn redraw_textures(&mut self) {
        self.time_textures.truncate(0);
        self.throttle_texture = None;

        let (time_texture_sender, time_texture_receiver) = channel();
        let (throttle_texture_sender, throttle_texture_receiver) = channel();

        let fft_size = self.fft_settings.size;

        let chunks = self.chunks.clone(); // TODO
        let mut fft_settings = self.fft_settings.clone();
        let fft_max = self.fft_settings.plot_max;
        let ctx = self.ctx.clone();
        execute(async move {
            for (i, columns) in chunks.chunks(TIME_DOMAIN_TEX_WIDTH).enumerate() {
                let image = Self::create_image(columns, fft_max, &mut fft_settings);
                let tex_handle = ctx.load_texture(format!("tex_{:?}", i), image, Default::default());
                let start = columns.first().unwrap().time;
                let end = columns.last().unwrap().time;
                time_texture_sender.send((start, end, tex_handle)).unwrap();
                ctx.request_repaint();
            }
        });

        let chunks = self.chunks.clone(); // TODO
        let mut fft_settings = self.fft_settings.clone();
        let fft_max = self.fft_settings.plot_max;
        let ctx = self.ctx.clone();
        execute(async move {
            const ARRAY_REPEAT_VALUE: std::vec::Vec<FftChunk> = Vec::new();
            let mut throttle_buckets: [Vec<FftChunk>; THROTTLE_DOMAIN_BUCKETS] = [ARRAY_REPEAT_VALUE; THROTTLE_DOMAIN_BUCKETS];
            for chunk in chunks {
                let bucket_i = ((chunk.throttle / 1000.0) * THROTTLE_DOMAIN_BUCKETS as f32) as usize;
                let bucket_i = usize::min(bucket_i, THROTTLE_DOMAIN_BUCKETS - 1);
                throttle_buckets[bucket_i].push(chunk);
            }

            let mut throttle_averages = Vec::new();
            for bucket in throttle_buckets.into_iter() {
                let size = bucket.len();
                let avg = bucket.into_iter()
                    .map(|chunk| chunk.fft)
                    .fold(vec![0f32; fft_size/2], |a, b| {
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
                            .collect()
                    })
                    .into_iter()
                    .map(|v| v / (size as f32))
                    .collect::<Vec<_>>();
                throttle_averages.push(avg);
            }

            let mut image = egui::ColorImage::new([THROTTLE_DOMAIN_BUCKETS, fft_size/2], Color32::TRANSPARENT);

            for x in 0..THROTTLE_DOMAIN_BUCKETS {
                for y in 0..fft_size/2 {
                    let val = throttle_averages[x][y];
                    image[(x, y)] = fft_settings.color_at(f32::max(0.0, val) / fft_max);
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
                    Ok(chunks) => { self.chunks.extend(chunks.into_iter()); },
                    Err(TryRecvError::Empty) => { break false; }
                    Err(TryRecvError::Disconnected) => { break true; }
                }
            }
        } else {
            false
        };

        if chunks_done {
            self.chunk_receiver = None;
            self.redraw_textures();
        }

        if let Some(receiver) = &self.time_texture_receiver {
            loop {
                match receiver.try_recv() {
                    Ok((t_start, t_end, tex)) => { self.time_textures.push((t_start, t_end, tex)); },
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

    pub fn set_fft_settings(&mut self, fft_settings: FftSettings) {
        let old_fft_settings = self.fft_settings.clone();
        self.fft_settings = fft_settings;

        if self.fft_settings.needs_recalculating(&old_fft_settings) {
            self.recalculate_ffts();
        } else if self.fft_settings.needs_redrawing(&old_fft_settings) {
            self.redraw_textures();
        }
    }

    pub fn set_flight(&mut self, fd: Arc<FlightData>) {
        self.flight_data = fd;
        self.recalculate_ffts();
    }

    pub fn show_time(&mut self, ui: &mut egui::Ui, total_width: f32) -> egui::Response {
        let max_freq = self.flight_data.sample_rate() / 2.0;
        let height = if ui.available_width() < total_width {
            ui.available_height()
        } else {
            300.0
        };

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
            .y_axis_position(egui_plot::HPlacement::Right)
            .y_axis_width(3)
            .y_axis_formatter(move |gm, _, _| format!("{:.0}Hz", gm.value * max_freq))
            .label_formatter(move |_name, val| format!("{:.0}Hz\n{:.3}s", val.y * max_freq, val.x))
            .height(height)
            .show(ui, |plot_ui| {
                for (t_start, t_end, texture) in self.time_textures.iter() {
                    let center = (t_start + t_end) / 2.0;
                    let duration = t_end - t_start;
                    let plot_image = egui_plot::PlotImage::new(
                        texture,
                        egui_plot::PlotPoint::new(center, 0.5),
                        egui::Vec2::new(duration as f32, 1.0),
                    );

                    plot_ui.image(plot_image);
                }
            })
            .response
    }

    pub fn show_throttle(&mut self, ui: &mut egui::Ui, total_width: f32) -> egui::Response {
        let max_freq = self.flight_data.sample_rate() / 2.0;
        let height = if ui.available_width() < total_width {
            ui.available_height()
        } else {
            300.0
        };

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
            .x_axis_formatter(move |gm, _, _| format!("{:.0}%", gm.value * 100.0))
            .y_axis_position(egui_plot::HPlacement::Right)
            .y_axis_width(3)
            .y_axis_formatter(move |gm, _, _| format!("{:.0}Hz", gm.value * max_freq))
            .label_formatter(move |_, val| format!("{:.0}Hz\n{:.0}%", val.y * max_freq, val.x * 100.0))
            .height(height)
            .reset()
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
            .response
    }

    pub fn show(&mut self, ui: &mut egui::Ui, domain: VibeDomain, total_width: f32) -> egui::Response {
        self.process_updates();

        if self.chunks.len() == 0 {
            ui.label("")
        } else {
            match domain {
                VibeDomain::Time => self.show_time(ui, total_width),
                VibeDomain::Throttle => self.show_throttle(ui, total_width)
            }
        }
    }
}

struct FftVectorSeries {
    axes: [FftAxis; 3]
}

impl FftVectorSeries {
    pub fn new(
        ctx: &egui::Context,
        fft_settings: FftSettings,
        fd: Arc<FlightData>,
        value_callback: fn(&FlightData) -> &[Vec<f32>; 3]
    ) -> Self {
        let axes = [
            FftAxis::new(ctx, fft_settings.clone(), 0, fd.clone(), value_callback.clone()),
            FftAxis::new(ctx, fft_settings.clone(), 1, fd.clone(), value_callback.clone()),
            FftAxis::new(ctx, fft_settings.clone(), 2, fd, value_callback),
        ];

        Self { axes }
    }

    pub fn set_fft_settings(&mut self, fft_settings: FftSettings) {
        self.axes[0].set_fft_settings(fft_settings.clone());
        self.axes[1].set_fft_settings(fft_settings.clone());
        self.axes[2].set_fft_settings(fft_settings);
    }

    pub fn set_flight(&mut self, fd: Arc<FlightData>) {
        self.axes[0].set_flight(fd.clone());
        self.axes[1].set_flight(fd.clone());
        self.axes[2].set_flight(fd);
    }

    pub fn show(&mut self, ui: &mut egui::Ui, domain: VibeDomain, total_width: f32) -> egui::Response {
        ui.vertical(|ui| {
            for (i, axis) in self.axes.iter_mut().enumerate() {
                ui.vertical(|ui| {
                    ui.set_height(ui.available_height() / (3 - i) as f32);
                    axis.show(ui, domain, total_width);
                });
            }
        }).response
    }
}

pub struct VibeTab {
    domain: VibeDomain,

    gyro_raw_enabled: bool,
    gyro_filtered_enabled: bool,
    dterm_raw_enabled: bool,
    dterm_filtered_enabled: bool,

    fft_settings: FftSettings,

    gyro_raw_ffts: FftVectorSeries,
    gyro_filtered_ffts: FftVectorSeries,
    //dterm_raw_ffts: FftVectorSeries,
    dterm_filtered_ffts: FftVectorSeries,
}

impl VibeTab {
    pub fn new(ctx: &egui::Context, fd: Arc<FlightData>) -> Self {
        let fft_settings = FftSettings::default();

        // TODO: unwrap
        let gyro_raw_ffts = FftVectorSeries::new(ctx, fft_settings.clone(), fd.clone(), |fd: &FlightData| &fd.gyro_unfilt.as_ref().unwrap().0);
        let gyro_filtered_ffts = FftVectorSeries::new(ctx, fft_settings.clone(), fd.clone(), |fd: &FlightData| &fd.gyro_adc.as_ref().unwrap().0);
        let dterm_filtered_ffts = FftVectorSeries::new(ctx, fft_settings.clone(), fd.clone(), |fd: &FlightData| &fd.d.as_ref().unwrap().0);

        Self {
            domain: VibeDomain::Time,

            gyro_raw_enabled: fd.gyro_unfilt.as_ref().map(|v| v[0].len() > 0).unwrap_or(false),
            gyro_filtered_enabled: true,
            dterm_raw_enabled: false, // TODO
            dterm_filtered_enabled: true,

            fft_settings,

            gyro_raw_ffts,
            gyro_filtered_ffts,
            //dterm_raw_ffts,
            dterm_filtered_ffts,
        }
    }

    pub fn update_fft_settings(&mut self) {
        self.gyro_raw_ffts.set_fft_settings(self.fft_settings.clone());
        self.gyro_filtered_ffts.set_fft_settings(self.fft_settings.clone());
        //self.dterm_raw_ffts.set_fft_settings(self.fft_settings.clone());
        self.dterm_filtered_ffts.set_fft_settings(self.fft_settings.clone());
    }

    pub fn set_flight(&mut self, fd: Arc<FlightData>) {
        self.gyro_raw_ffts.set_flight(fd.clone());
        self.gyro_filtered_ffts.set_flight(fd.clone());
        //self.dterm_raw_ffts.set_flight(fd.clone());
        self.dterm_filtered_ffts.set_flight(fd.clone());
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        _fd: &FlightData,
        _timeseries_group: &mut TimeseriesGroup
    ) {
        let old_fft_settings = self.fft_settings.clone();
        let fft_size = self.fft_settings.size;
        let total_width = ui.available_width();

        FlexLayout::new(1500.0, "Settings")
            .add(|ui| ui.horizontal(|ui| {
                ui.label("Domain:");
                ui.selectable_value(&mut self.domain, VibeDomain::Time, "🕙 Time");
                ui.selectable_value(&mut self.domain, VibeDomain::Throttle, "🏃 Throttle");
            }).response)
            .add(|ui| ui.horizontal(|ui| {
                ui.label("Series:");
                ui.toggle_value(&mut self.gyro_raw_enabled, "Gyro (raw)");
                ui.toggle_value(&mut self.gyro_filtered_enabled, "Gyro (filtered)");
                ui.toggle_value(&mut self.dterm_raw_enabled, "D term (raw)");
                ui.toggle_value(&mut self.dterm_filtered_enabled, "D term (filtered)");
            }).response)
            .add(|ui| ui.horizontal(|ui| {
                ui.label("FFT Size:");
                for size in &FFT_SIZE_OPTIONS {
                    ui.selectable_value(&mut self.fft_settings.size, *size, format!("{}", size));
                }
            }).response)
            .add(|ui| ui.horizontal(|ui| {
                ui.label("FFT Step Size:");
                for value in &[1, 8, 32, 128, 256, 512, 1024] {
                    ui.add_enabled_ui(*value <= fft_size, |ui| {
                        ui.selectable_value(&mut self.fft_settings.step_size, *value, format!("{}", value))
                    });
                }
            }).response)
            .add(|ui| ui.horizontal(|ui| {
                ui.label("Colorscheme:");
                for value in &[Colorscheme::Turbo, Colorscheme::Viridis, Colorscheme::Inferno] {
                    ui.selectable_value(&mut self.fft_settings.plot_colorscheme, *value, format!("{:?}", value));
                }
            }).response)
            .add(|ui| ui.horizontal(|ui| {
                ui.label("FFTMax:");
                ui.add(DragValue::new(&mut self.fft_settings.plot_max).clamp_range(0.0..=100.0).speed(0.01));
            }).response)
            .show(ui);

        if self.fft_settings != old_fft_settings {
            self.fft_settings.step_size = usize::min(self.fft_settings.step_size, self.fft_settings.size);
            self.update_fft_settings();
        }

        ui.separator();

        FlexColumns::new(MIN_WIDE_WIDTH)
            .column_enabled(self.gyro_raw_enabled, |ui| {
                ui.heading("Gyro (raw)");
                self.gyro_raw_ffts.show(ui, self.domain, total_width)
            })
            .column_enabled(self.gyro_filtered_enabled, |ui| {
                ui.heading("Gyro (filtered)");
                self.gyro_filtered_ffts.show(ui, self.domain, total_width)
            })
            .column_enabled(self.dterm_raw_enabled, |ui| {
                ui.heading("D Term (raw)")
                //self.dterm_raw_ffts.show(ui, self.domain, total_width)
            })
            .column_enabled(self.dterm_filtered_enabled, |ui| {
                ui.heading("D Term (filtered)");
                self.dterm_filtered_ffts.show(ui, self.domain, total_width)
            })
            .show(ui);
    }
}
