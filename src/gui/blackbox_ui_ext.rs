use blackbox_log::headers::{Firmware, ParseError};
use egui::Image;

/// Give some of the types defined by blackbox_log crate methods to draw them
pub trait BlackboxUiExt {
    fn show(&self, ui: &mut egui::Ui);
}

impl BlackboxUiExt for Firmware {
    fn show(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let image = match self {
                Firmware::Betaflight(_fv) => {
                    if ui.visuals().dark_mode {
                        Image::new(egui::include_image!("../../assets/betaflight_icon_dark.png"))
                    } else {
                        Image::new(egui::include_image!("../../assets/betaflight_icon_light.png"))
                    }
                }
                Firmware::Inav(_fv) => {
                    Image::new(egui::include_image!("../../assets/inav_icon.png"))
                }
            };

            ui.add(image.max_height(10.0));

            ui.label(format!("{} {}", self.name(), self.version()));
        });
    }
}

impl BlackboxUiExt for ParseError {
    fn show(&self, ui: &mut egui::Ui) {
        match self {
            ParseError::UnsupportedFirmwareVersion(fw) => {
                ui.vertical(|ui| {
                    ui.label("Unsupported FW version:");
                    fw.show(ui);
                });
            },
            ParseError::InvalidFirmware(fw_str) => {
                ui.vertical(|ui| {
                    ui.label("Unsupported FW:");
                    ui.label(fw_str);
                });
            }
            ParseError::UnsupportedDataVersion => {
                ui.label("Unsupported data version");
            }
            ParseError::InvalidHeader { header, value } => {
                ui.label("Invalid header:");
                ui.monospace(format!("{} = {}", header, value));
            }
            ParseError::MissingHeader | ParseError::IncompleteHeaders => {
                ui.label("Missing header");
            }
            ParseError::MissingField { frame, field } => {
                ui.label("Missing field:");
                ui.monospace(format!("{} = {}", frame, field));
            }
            ParseError::MalformedFrameDef(frame) => {
                ui.label("Malformed frame def.:");
                ui.monospace(format!("{}", frame));
            }
        }
    }
}
