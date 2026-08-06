use realfft::num_complex::Complex32;
use std::collections::VecDeque; // Added for moving average history

fn fft_forward(data: &[f32]) -> Vec<Complex32> {
    // Ensure input is not empty
    if data.is_empty() {
        return Vec::new();
    }
    let mut input = data.to_vec();
    let planner = realfft::RealFftPlanner::<f32>::new().plan_fft_forward(input.len());
    let mut output = planner.make_output_vec();
    // Use a match or expect for better error handling if desired
    let _ = planner.process(&mut input, &mut output); // Error ignored for simplicity like original
    output
}

// Corrected fft_inverse to use the original real signal length N
fn fft_inverse(data: &[Complex32], original_length_n: usize) -> Vec<f32> {
    // Ensure input is not empty and N is valid
    if data.is_empty() || original_length_n == 0 {
        return vec![0.0; original_length_n];
    }
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
        // Length mismatch, cannot perform inverse FFT correctly
        eprintln!("Warning: FFT inverse length mismatch. Expected complex length {}, got {}. Returning zeros.", expected_complex_len, input.len());
        return vec![0.0; original_length_n];
    }

    if planner.process(&mut input, &mut output).is_ok() {
        // Normalize the IFFT output (realfft doesn't normalize by default)
        let scale = 1.0 / original_length_n as f32;
        output.iter_mut().for_each(|x| *x *= scale);
        output
    } else {
        // Error during processing
        eprintln!("Warning: FFT inverse processing failed. Returning zeros.");
        vec![0.0; original_length_n]
    }
}

// Helper function for moving average smoothing
fn moving_average_smooth(data: &[f32], window_size: usize) -> Vec<f32> {
    if window_size <= 1 || data.is_empty() {
        return data.to_vec(); // No smoothing needed or possible
    }

    let mut smoothed_data = Vec::with_capacity(data.len());
    let mut current_sum: f32 = 0.0;
    // Using VecDeque to efficiently manage the sliding window sum
    let mut history: VecDeque<f32> = VecDeque::with_capacity(window_size);

    for &val in data.iter() {
        history.push_back(val);
        current_sum += val;

        // If the window is full, remove the oldest element from the sum and the deque
        if history.len() > window_size {
            current_sum -= history.pop_front().unwrap(); // unwrap is safe due to check
        }

        // Calculate the average over the current window contents
        // The effective window size grows until it reaches `window_size`
        smoothed_data.push(current_sum / (history.len() as f32));
    }

    smoothed_data
}


pub fn calculate_step_response(
    times: &[f64],
    setpoint: &[f32],
    gyro_filtered: &[f32],
    sample_rate: f64,
) -> Vec<(f64, f64)> {
    // Basic validation
    if times.is_empty() || setpoint.is_empty() || gyro_filtered.is_empty() || setpoint.len() != gyro_filtered.len() || times.len() != setpoint.len() || sample_rate <= 0.0 {
        eprintln!("Warning: Invalid input to calculate_step_response. Empty data, length mismatch, or invalid sample rate.");
        return Vec::new(); // Return empty if inputs are invalid
    }

    let n_samples = setpoint.len(); // Original number of samples (N)

    let input_spectrum = fft_forward(setpoint);
    let output_spectrum = fft_forward(gyro_filtered);

    // Ensure FFT outputs are compatible
    if input_spectrum.len() != output_spectrum.len() || input_spectrum.is_empty() {
         eprintln!("Warning: FFT outputs have different lengths or are empty. Cannot calculate frequency response.");
        return Vec::new();
    }

    let input_spec_conj: Vec<_> = input_spectrum.iter().map(|c| c.conj()).collect();

    // Calculate Frequency Response H(f) = (Input*(f) * Output(f)) / (Input*(f) * Input(f))
    // Add a small epsilon to denominator to avoid division by zero/very small numbers
    let epsilon = 1e-9;
    let frequency_response: Vec<_> = input_spectrum
        .iter()
        .zip(output_spectrum.iter())
        .zip(input_spec_conj.iter())
        .map(|((i, o), i_conj)| {
            let denominator = (i_conj * i).re.max(epsilon); // Use real part, ensure positive and non-zero
            (i_conj * o) / denominator
        })
        .collect();

    // Calculate Impulse Response (Inverse FFT of Frequency Response)
    // Pass the original signal length `n_samples`
    let impulse_response = fft_inverse(&frequency_response, n_samples);

    // Calculate Step Response (Cumulative Sum of Impulse Response)
    let step_response: Vec<_> = impulse_response
        .iter()
        .scan(0.0, |cum_sum, &x| {
            // Check for NaN/Inf in impulse response before summing
            if x.is_finite() {
                 *cum_sum += x;
            } else {
                eprintln!("Warning: Non-finite value detected in impulse response. Skipping.");
                // Keep cum_sum as is, or handle as needed
            }
            Some(*cum_sum)
        })
        .collect();

    // --- START: Smoothing Step ---
    // Define the desired smoothing duration in seconds (e.g., 10ms)
    let smoothing_duration_s = 0.01; // 10 ms
    // Calculate dynamic window size based on sample rate
    // Ensure window size is at least 1
    let window_size = ((smoothing_duration_s * sample_rate).round() as usize).max(1);

    // Apply moving average smoothing
    let smoothed_step_response = moving_average_smooth(&step_response, window_size);
    // --- END: Smoothing Step ---


    // Determine the number of samples corresponding to 500ms for truncation
    let num_points_500ms = (sample_rate * 0.5).ceil() as usize;
    // Ensure we don't take more points than available
    let truncated_len = num_points_500ms.min(smoothed_step_response.len());
    if truncated_len == 0 {
        return Vec::new(); // Nothing to normalize or return
    }

    // Calculate the average of the *first* 500ms (or fewer if shorter) of the SMOOTHED response
    // This follows the original logic's intent but applies it to the smoothed data.
    let avg_sum: f32 = smoothed_step_response.iter().take(truncated_len).sum();
    // Ensure divisor is not zero
    let divisor = truncated_len as f32;
    let avg = if divisor > 0.0 { avg_sum / divisor } else { 1.0 }; // Default average to 1 if no data

    // Avoid division by zero or near-zero average for normalization
    let normalization_factor = if avg.abs() < 1e-7 { 1.0 } else { avg };


    let normalized_smoothed: Vec<_> = smoothed_step_response
        .iter()
        .take(truncated_len) // limit to first 500ms (or available)
        .map(|x| x / normalization_factor)
        .collect();

    let start_time = times.first().cloned().unwrap_or(0.0);

    // Combine time data with the smoothed, normalized, truncated step response
    times
        .iter()
        .zip(normalized_smoothed.into_iter()) // Use smoothed, normalized data
        .map(|(&t, s)| (t - start_time, s as f64)) // Adjust time to start from 0
        .collect()
}
