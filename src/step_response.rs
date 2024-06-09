pub fn calculate_step_response(
    _times: &[f64], // TODO: necessary?
    setpoint: &[f64],
    gyro_filtered: &[f64],
) -> Vec<(f64, f64)> {
    // placeholder. TODO: calculate step response here
    let mut setpoint_and_gyro: Vec<(f64, f64)> = setpoint
        .iter()
        .zip(gyro_filtered.iter())
        .map(|(s, g)| (*s, *g))
        .collect();
    setpoint_and_gyro.sort_by(|(_s, g), (_sa, ga)| g.partial_cmp(ga).unwrap());
    setpoint_and_gyro.sort_by(|(s, _g), (sa, _ga)| s.partial_cmp(sa).unwrap());
    setpoint_and_gyro
}
