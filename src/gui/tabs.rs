mod plot;
mod tune;
mod vibe;

use std::fmt::Display;

pub use plot::*;
pub use tune::*;
pub use vibe::*;

const PLOT_HEIGHT: f32 = 300.0;
const MIN_WIDE_WIDTH: f32 = 1000.0;

#[derive(Default, Clone, Copy, PartialEq)]
pub enum FlightViewTab {
    #[default]
    Plot,
    Tune,
    Vibe,
}

impl Display for FlightViewTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let val = match self {
            Self::Plot => "🗠  Plot",
            Self::Tune => "⛭  Tune",
            Self::Vibe => "💃 Vibe",
        };
        write!(f, "{val}",)
    }
}
