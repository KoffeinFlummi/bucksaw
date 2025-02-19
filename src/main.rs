#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod flight_data;
mod gui;
mod iter;
mod log_file;
mod step_response;
mod utils;

use gui::App;
use std::path::PathBuf;

fn main() {
    init_logger();

    let path = path_arg();

    run_app(path);
}
fn run_app(path: Option<PathBuf>) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let native_options = eframe::NativeOptions::default();
        eframe::run_native(
            "bucksaw",
            native_options,
            Box::new(|cc| Box::new(App::new(cc, path))),
        )
        .expect("failed to start eframe");
    }

    #[cfg(target_arch = "wasm32")]
    {
        wasm_bindgen_futures::spawn_local(async {
            eframe::WebRunner::new()
                .start(
                    "canvas", // see index.html
                    eframe::WebOptions::default(),
                    Box::new(|cc| Box::new(App::new(cc, None))),
                )
                .await
                .expect("failed to start eframe");
        });
    }
}
fn init_logger() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
        #[cfg(feature = "profiling")]
        puffin::set_scopes_on(true);
    }

    #[cfg(target_arch = "wasm32")]
    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();
}

fn path_arg() -> Option<PathBuf> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let args: Vec<_> = std::env::args().collect();
        args.get(1).map(PathBuf::from)
    }

    #[cfg(target_arch = "wasm32")]
    None
}
