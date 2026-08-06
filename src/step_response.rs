use realfft::num_complex::Complex32;
use realfft::FftError;

use std::collections::VecDeque;
use std::num::NonZero;
use std::ops::RangeInclusive;

#[derive(Debug)]
#[allow(dead_code)]
pub enum StepResponseError {
    InvalidInputData,
    NoWindowShift,
    NoWindowsLeft,
    NoFilteredWindowsLeft,
    Fft(FftError),
    FftLengthMismatch,
}

#[derive(Clone, PartialEq)]
pub struct StepResponseConfiguration {
    /// Length of the step response to keep
    pub response_length: f64,
    /// Length of each window in seconds
    pub window_length: f64,
    /// Number of overlapping windows
    pub window_overlap: f64,
    /// Alpha for Tukey window (1.0 is Hanning window)
    pub tukey_alpha: f64,
    /// Initial Gyro Smoothing
    pub gyro_mov_avg_window: NonZero<usize>,
    /// Minimum setpoint magnitude
    pub min_setpoint: f32,
    /// Individual Response "Y-Correction"
    pub enable_prefilter_normalization: bool,
    /// Minimum absolute mean for Y-correction
    pub prefilter_normalization_minimum: f32,
    /// Permissible range for steady-state values after normalization
    pub normalized_steady_state_range: RangeInclusive<f32>,
    /// Enable additional check on mean
    pub enable_normalized_steady_state_mean_check: bool,
    /// Permissible range for steady-state mean after normalization
    pub normalized_steady_state_mean_range: RangeInclusive<f32>,
    /// Start time for steady-state check
    pub steady_state_start_seconds: f64,
    /// End time for steady-state check
    pub steady_state_end_seconds: f64,
}

impl Default for StepResponseConfiguration {
    fn default() -> Self {
        Self {
            response_length: 0.5,
            window_length: 2.0,
            window_overlap: 0.9,
            tukey_alpha: 1.0,
            gyro_mov_avg_window: NonZero::new(15).unwrap(),
            min_setpoint: 20.0,
            enable_prefilter_normalization: true,
            prefilter_normalization_minimum: 0.1,
            normalized_steady_state_range: 0.5..=3.0,
            enable_normalized_steady_state_mean_check: true,
            normalized_steady_state_mean_range: 0.75..=1.25,
            steady_state_start_seconds: 0.2,
            steady_state_end_seconds: 0.5,
        }
    }
}

fn fft_forward(data: &[f32]) -> Result<Vec<Complex32>, FftError> {
    let mut input = data.to_vec();
    let planner = realfft::RealFftPlanner::<f32>::new().plan_fft_forward(input.len());
    let mut output = planner.make_output_vec();
    planner.process(&mut input, &mut output)?;
    Ok(output)
}

// Corrected fft_inverse to use the original real signal length N
fn fft_inverse(
    data: &[Complex32],
    original_length_n: usize,
) -> Result<Vec<f32>, StepResponseError> {
    let mut input = data.to_vec();

    // The inverse planner needs the length of the original real signal (N)
    let planner = realfft::RealFftPlanner::<f32>::new().plan_fft_inverse(original_length_n);
    let mut output = planner.make_output_vec(); // Output will have length N

    // Check if planner input length matches provided data length
    // Required complex input length depends on N (even/odd)
    let expected_complex_len = if original_length_n % 2 == 0 {
        original_length_n / 2 + 1
    } else {
        (original_length_n + 1) / 2
    };

    if input.len() != expected_complex_len {
        return Err(StepResponseError::FftLengthMismatch);
    }

    planner
        .process(&mut input, &mut output)
        .map_err(StepResponseError::Fft)?;

    // Normalize the IFFT output (realfft doesn't normalize by default)
    let scale = 1.0 / original_length_n as f32;
    output.iter_mut().for_each(|x| *x *= scale);
    Ok(output)
}

/// Creates a Tukey window for signal tapering with configurable alpha parameter.
fn tukey_window(num: usize, alpha: f64) -> Vec<f32> {
    if alpha <= 0.0 {
        return vec![1.0; num];
    } else if alpha >= 1.0 {
        // Full Hanning window when alpha = 1.0
        let mut window = vec![0.0; num];
        for i in 0..num {
            window[i] = 0.5
                * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / (num as f64 - 1.0)).cos()) as f32;
        }
        return window;
    }

    let mut window = vec![1.0; num];
    let alpha_half = alpha / 2.0;
    let n_alpha = (alpha_half * (num as f64 - 1.0)).floor() as usize;

    // Apply cosine tapering to both ends
    for i in 0..n_alpha {
        window[i] = 0.5 * (1.0 + (std::f64::consts::PI * i as f64 / (n_alpha as f64)).cos()) as f32;
        window[num - 1 - i] = window[i];
    }

    window
}

/// Generates overlapping windowed segments from input/output data for parallel analysis.
fn winstacker_contiguous(
    input_data: &[f32],
    output_data: &[f32],
    window_samples: usize,
    window_overlap: f64,
) -> Result<(Vec<Vec<f32>>, Vec<Vec<f32>>), StepResponseError> {
    let total_len = input_data.len();
    //let shift = window_samples / superposition_factor;
    let shift = ((window_samples as f64) * (1.0 - window_overlap)) as usize;
    //let shift = 250;
    if shift == 0 {
        return Err(StepResponseError::NoWindowShift);
    }

    let num_windows = if total_len >= window_samples {
        (total_len - window_samples) / shift + 1
    } else {
        0
    };

    if num_windows == 0 {
        return Err(StepResponseError::NoWindowsLeft);
    }

    let mut stacked_input = Vec::with_capacity(num_windows);
    let mut stacked_output = Vec::with_capacity(num_windows);

    for i in 0..num_windows {
        let start = i * shift;
        let end = start + window_samples;
        stacked_input.push(input_data[start..end].to_vec());
        stacked_output.push(output_data[start..end].to_vec());
    }

    Ok((stacked_input, stacked_output))
}

/// Performs Wiener deconvolution to extract system impulse response from input/output signals.
fn wiener_deconvolution_window(
    input_window: &[f32],
    output_window: &[f32],
    _sample_rate: f64,
) -> Result<Vec<f32>, StepResponseError> {
    let n = input_window.len();
    if n == 0 {
        return Err(StepResponseError::InvalidInputData);
    }

    let padded_n = n.next_power_of_two();

    let mut input_padded = input_window.to_vec();
    input_padded.resize(padded_n, 0.0);
    let mut output_padded = output_window.to_vec();
    output_padded.resize(padded_n, 0.0);

    let h_spec = fft_forward(&input_padded).map_err(StepResponseError::Fft)?;
    let g_spec = fft_forward(&output_padded).map_err(StepResponseError::Fft)?;

    // Apply regularization to prevent division by near-zero values
    let regularization_term = 0.0001;
    let epsilon = 1e-9;

    let deconvolved_spec: Vec<_> = h_spec
        .into_iter()
        .zip(g_spec.into_iter())
        .map(|(h, g)| {
            let h_conj = h.conj();

            let denominator = (h * h_conj).re + regularization_term;
            if denominator.abs() > epsilon {
                (g * h_conj) / denominator
            } else {
                Complex32::new(0.0, 0.0)
            }
        })
        .collect();

    // Return only the first n samples
    Ok(fft_inverse(&deconvolved_spec, padded_n)?[0..n].to_vec())
}

fn moving_average(data: &[f32], window_size: NonZero<usize>) -> Vec<f32> {
    let mut current_sum: f32 = 0.0;
    // Using VecDeque to efficiently manage the sliding window sum
    let mut history: VecDeque<f32> = VecDeque::with_capacity(window_size.get());

    data.into_iter()
        .map(|val| {
            history.push_back(*val);
            current_sum += val;

            // If the window is full, remove the oldest element from the sum and the deque
            if history.len() > window_size.get() {
                current_sum -= history.pop_front().unwrap(); // unwrap is safe due to check
            }

            // Calculate the average over the current window contents
            // The effective window size grows until it reaches `window_size`
            current_sum / (history.len() as f32)
        })
        .collect()
}

/// Averages multiple step responses for final result.
fn average_responses(stacked_responses: &[Vec<f32>]) -> Result<Vec<f64>, StepResponseError> {
    if stacked_responses.len() == 0 {
        return Err(StepResponseError::InvalidInputData);
    }

    let response_len = stacked_responses[0].len();
    let mut averaged_response = vec![0.0; response_len];
    let mut active_window_counts = vec![0.0; response_len];

    for i in 0..stacked_responses.len() {
        let response = &stacked_responses[i];
        for j in 0..response.len() {
            let response_value = response[j] as f64;
            if response_value.is_finite() {
                averaged_response[j] += response_value;
                active_window_counts[j] += 1.0;
            }
        }
    }

    for j in 0..response_len {
        if active_window_counts[j] > 0.0 {
            averaged_response[j] /= active_window_counts[j];
        }
    }

    Ok(averaged_response)
}

pub fn calculate_step_response(
    times: &[f64],
    setpoint: &[f32],
    gyro: &[f32],
    sample_rate: f64,
    config: &StepResponseConfiguration,
) -> Result<Vec<(f64, f64)>, StepResponseError> {
    // Basic validation
    if times.is_empty()
        || setpoint.is_empty()
        || gyro.is_empty()
        || setpoint.len() != gyro.len()
        || times.len() != setpoint.len()
        || sample_rate <= 0.0
    {
        return Err(StepResponseError::InvalidInputData);
    }

    // Apply initial smoothing to gyro data if configured
    let gyro_processed = if config.gyro_mov_avg_window.get() > 1 {
        moving_average(gyro, config.gyro_mov_avg_window)
    } else {
        gyro.to_vec()
    };

    // Calculate frame and response lengths in samples
    let window_samples = (config.window_length * sample_rate).ceil() as usize;
    let response_samples = (config.response_length * sample_rate).ceil() as usize;
    if window_samples == 0 || response_samples == 0 {
        return Err(StepResponseError::InvalidInputData);
    }

    // Define steady-state region for quality control checks
    let ss_start_sample = (config.steady_state_start_seconds * sample_rate).floor() as usize;
    let ss_end_sample = (config.steady_state_end_seconds * sample_rate).ceil() as usize;
    let ss_start_i = ss_start_sample.min(response_samples.saturating_sub(1));
    let ss_end_i = ss_end_sample.min(response_samples).max(ss_start_i + 1);

    // Generate overlapping windows for parallel processing
    let (stacked_input_raw, stacked_output_raw) = winstacker_contiguous(
        setpoint,
        &gyro_processed,
        window_samples,
        config.window_overlap,
    )?;

    let window = tukey_window(window_samples, config.tukey_alpha);

    let (_remaining_indices, filtered_windows): (Vec<_>, Vec<_>) = stacked_input_raw
        .iter()
        .zip(stacked_output_raw.iter())
        .enumerate()
        // apply minimum-activity filtering
        .filter(|(_i, (setpoint, _gyro))| {
            setpoint
                .iter()
                .fold(0f32, |max_val, &v| max_val.max(v.abs()))
                >= config.min_setpoint
        })
        // apply windowing function
        .map(|(i, (setpoint, gyro))| {
            let setpoint_windowed: Vec<_> = setpoint
                .iter()
                .zip(window.iter())
                .map(|(&x, &w)| x * w)
                .collect();
            let gyro_windowed: Vec<_> = gyro
                .iter()
                .zip(window.iter())
                .map(|(&x, &w)| x * w)
                .collect();
            (i, (setpoint_windowed, gyro_windowed))
        })
        // apply wiener deconvolution
        .filter_map(|(i, (input, output))| {
            match wiener_deconvolution_window(&input, &output, sample_rate) {
                Ok(response) => Some((i, response)),
                Err(e) => {
                    log::warn!("Deconvolution error, skipping: {:?}", e);
                    None
                }
            }
        })
        // convert to step response via cumsum
        .map(|(i, impulse_response)| {
            let step_response = impulse_response
                .iter()
                .take(response_samples)
                .scan(0.0, |cumsum, &x| {
                    if x.is_finite() {
                        *cumsum += x
                    }
                    Some(*cumsum)
                })
                .collect::<Vec<_>>();
            (i, step_response)
        })
        // apply pre-filtering normalization
        .filter_map(|(i, step_response)| {
            if !config.enable_prefilter_normalization {
                return Some((i, step_response));
            }

            // Calculate mean of the steady-state segment
            let segment = &step_response[ss_start_i..ss_end_i];
            let mean = segment.iter().sum::<f32>() / segment.len() as f32;
            let mean = mean.is_finite().then_some(mean).unwrap_or(0.0);

            (mean.abs() > config.prefilter_normalization_minimum)
                .then_some((i, step_response.iter().map(|v| v / mean).collect()))
        })
        // filter out windows which don't reach a steady state
        .filter_map(|(i, step_response)| {
            let segment = &step_response[ss_start_i..ss_end_i];
            let min = segment.iter().fold(f32::INFINITY, |a, &b| a.min(b));
            let max = segment.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
            let mean = segment.iter().sum::<f32>() / segment.len() as f32;

            // Quality control checks
            let valid = min.is_finite()
                && max.is_finite()
                && &min > config.normalized_steady_state_range.start()
                && &max < config.normalized_steady_state_range.end()
                && (!config.enable_normalized_steady_state_mean_check
                    || config.normalized_steady_state_mean_range.contains(&mean));
            valid.then_some((i, step_response))
        })
        .unzip();

    // TODO: do something with the remaining indices, maybe display graphically?

    if filtered_windows.is_empty() {
        return Err(StepResponseError::NoFilteredWindowsLeft);
    }

    // Average the quality-controlled responses
    let combined_response = average_responses(&filtered_windows)?;

    // Apply post-processing: shift to start at 0 and normalize
    let first = combined_response[0];
    let shifted_response: Vec<_> = combined_response.iter().map(|&v| v - first).collect();

    // Calculate the steady-state mean for final normalization
    let ss_segment = &shifted_response[ss_start_i..ss_end_i];
    let ss_mean = ss_segment.iter().sum::<f64>() / ss_segment.len() as f64;

    let divisor = (ss_mean.abs() > 1e-9).then_some(ss_mean).unwrap_or(1.0);

    // Create time vector for the response (starting from 0)
    Ok(shifted_response
        .into_iter()
        .enumerate()
        .map(|(i, s)| ((i as f64) / sample_rate, s / divisor))
        .collect())
}
