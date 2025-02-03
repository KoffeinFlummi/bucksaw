#![allow(dead_code)]

use egui::Color32;

// Gruvbox colors
// https://github.com/morhetz/gruvbox

const BG_DARK_MODE: Color32 = Color32::from_rgb(0x28, 0x28, 0x28);
const FG_DARK_MODE: Color32 = Color32::from_rgb(0xeb, 0xdb, 0xb2);

const BG_LIGHT_MODE: Color32 = Color32::from_rgb(0xfb, 0xf1, 0xc7);
const FG_LIGHT_MODE: Color32 = Color32::from_rgb(0x3c, 0x38, 0x36);

const RED: Color32 = Color32::from_rgb(0xcc, 0x24, 0x1d);
const GREEN: Color32 = Color32::from_rgb(0x98, 0x97, 0x1a);
const BLUE: Color32 = Color32::from_rgb(0x45, 0x85, 0x88);
const YELLOW: Color32 = Color32::from_rgb(0xd7, 0x99, 0x21);
const PURPLE: Color32 = Color32::from_rgb(0xb1, 0x62, 0x86);
const AQUA: Color32 = Color32::from_rgb(0x68, 0x9d, 0x6a);
const ORANGE: Color32 = Color32::from_rgb(0xd6, 0x5d, 0x0e);

const RED_LIGHT: Color32 = Color32::from_rgb(0xfb, 0x49, 0x34);
const GREEN_LIGHT: Color32 = Color32::from_rgb(0xb8, 0xbb, 0x26);
const BLUE_LIGHT: Color32 = Color32::from_rgb(0x83, 0xa5, 0x98);
const YELLOW_LIGHT: Color32 = Color32::from_rgb(0xfa, 0xbd, 0x2f);
const PURPLE_LIGHT: Color32 = Color32::from_rgb(0xd3, 0x86, 0x9b);
const AQUA_LIGHT: Color32 = Color32::from_rgb(0x8e, 0xc0, 0x7c);
const ORANGE_LIGHT: Color32 = Color32::from_rgb(0xfe, 0x80, 0x19);

const RED_DARK: Color32 = Color32::from_rgb(0x9d, 0x00, 0x06);
const GREEN_DARK: Color32 = Color32::from_rgb(0x79, 0x74, 0x0e);
const BLUE_DARK: Color32 = Color32::from_rgb(0x07, 0x66, 0x78);
const YELLOW_DARK: Color32 = Color32::from_rgb(0xb5, 0x76, 0x14);
const PURPLE_DARK: Color32 = Color32::from_rgb(0x8f, 0x37, 0x71);
const AQUA_DARK: Color32 = Color32::from_rgb(0x42, 0x7b, 0x58);
const ORANGE_DARK: Color32 = Color32::from_rgb(0xaf, 0x3a, 0x03);

const COLORS_DARK_MODE: Colors = Colors {
    triple_primary: [RED_LIGHT, GREEN_LIGHT, BLUE_LIGHT],
    triple_secondary: [RED, GREEN, BLUE],
    quad: [RED_LIGHT, GREEN_LIGHT, BLUE_LIGHT, PURPLE_LIGHT],
    motors: [
        YELLOW_LIGHT,
        PURPLE_LIGHT,
        AQUA_LIGHT,
        ORANGE_LIGHT,
        RED_LIGHT,
        GREEN_LIGHT,
        BLUE_LIGHT,
        FG_DARK_MODE,
    ],

    gyro_unfiltered: RED,
    gyro_filtered: RED_LIGHT,
    setpoint: FG_DARK_MODE,
    p: GREEN_LIGHT,
    i: BLUE_LIGHT,
    d: ORANGE_LIGHT,
    f: YELLOW,

    voltage: BLUE_LIGHT,
    current: RED_LIGHT,
    rssi: AQUA_LIGHT,

    error: RED,
    selected: ORANGE_LIGHT,
};

const COLORS_LIGHT_MODE: Colors = Colors {
    triple_primary: [RED_DARK, GREEN_DARK, BLUE_DARK],
    triple_secondary: [RED, GREEN, BLUE],
    quad: [RED_DARK, GREEN_DARK, BLUE_DARK, PURPLE_DARK],
    motors: [
        YELLOW_DARK,
        PURPLE_DARK,
        AQUA_DARK,
        ORANGE_DARK,
        RED_DARK,
        GREEN_DARK,
        BLUE_DARK,
        FG_LIGHT_MODE,
    ],

    gyro_unfiltered: RED,
    gyro_filtered: RED_DARK,
    setpoint: FG_LIGHT_MODE,
    p: GREEN_DARK,
    i: BLUE_DARK,
    d: ORANGE_DARK,
    f: YELLOW,

    voltage: BLUE_DARK,
    current: RED_DARK,
    rssi: AQUA_DARK,

    error: RED,
    selected: ORANGE_DARK,
};

pub struct Colors {
    pub triple_primary: [Color32; 3],
    pub triple_secondary: [Color32; 3],
    pub quad: [Color32; 4],
    pub motors: [Color32; 8],

    pub gyro_unfiltered: Color32,
    pub gyro_filtered: Color32,
    pub setpoint: Color32,
    pub p: Color32,
    pub i: Color32,
    pub d: Color32,
    pub f: Color32,

    pub voltage: Color32,
    pub current: Color32,
    pub rssi: Color32,

    pub error: Color32,
    pub selected: Color32,
}

impl Colors {
    pub fn get(ui: &egui::Ui) -> Self {
        if ui.visuals().dark_mode {
            COLORS_DARK_MODE
        } else {
            COLORS_LIGHT_MODE
        }
    }
}
