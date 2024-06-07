mod plot;
mod tune;
mod vibe;

pub use plot::*;
pub use tune::*;
pub use vibe::*;

#[derive(Default, Clone, Copy, PartialEq)]
pub enum FlightViewTab {
    #[default]
    Plot,
    Tune,
    Vibe
}

impl ToString for FlightViewTab {
    fn to_string(&self) -> String {
        match self {
            Self::Plot => "🗠  Plot",
            Self::Tune => "⛭  Tune",
            Self::Vibe => "💃 Vibe",
        }.to_string()
    }
}

