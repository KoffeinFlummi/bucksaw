use realfft::num_complex::Complex32;

fn fft_forward(data: &[f32]) -> Vec<Complex32> {
    let mut input = data.to_vec();
    let planner = realfft::RealFftPlanner::<f32>::new().plan_fft_forward(input.len());
    let mut output = planner.make_output_vec();
    planner.process(&mut input, &mut output).unwrap();
    output
}

fn fft_inverse(data: &[Complex32]) -> Vec<f32> {
    let mut input = data.to_vec();
    let planner = realfft::RealFftPlanner::<f32>::new().plan_fft_inverse(input.len() * 2 - 1);
    let mut output = planner.make_output_vec();
    if planner.process(&mut input, &mut output).is_ok() {
        output
    } else {
        vec![0.0; input.len()]
    }
}

pub fn calculate_step_response(
    times: &[f64],
    setpoint: &[f32],
    gyro_filtered: &[f32],
    sample_rate: f64,
) -> Vec<(f64, f64)> {
    let input_spectrum = fft_forward(setpoint);
    let output_spectrum = fft_forward(gyro_filtered);

    let input_spec_conj: Vec<_> = input_spectrum.iter().map(|c| c.conj()).collect();
    let frequency_response: Vec<_> = input_spectrum
        .iter()
        .zip(output_spectrum.iter())
        .zip(input_spec_conj.iter())
        .map(|((i, o), i_conj)| (i_conj * o) / (i_conj * i))
        .collect();

    let impulse_response = fft_inverse(&frequency_response);
    let step_response: Vec<_> = impulse_response
        .iter()
        .scan(0.0, |cum_sum, x| {
            *cum_sum += *x;
            Some(*cum_sum)
        })
        .collect();

    let avg = step_response.iter().sum::<f32>() / (step_response.len() as f32);
    let normalized = step_response
        .iter()
        .take((sample_rate / 2.0) as usize) // limit to last 500ms
        .map(|x| x / avg);

    let start = times.first().cloned().unwrap_or(0.0);
    times
        .iter()
        .zip(normalized)
        .map(|(t, s)| (*t - start, s as f64))
        .collect()
}
