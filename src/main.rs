#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod flight_data;
mod gui;
mod iter;
mod log_file;
mod step_response;
mod utils;

use gui::App;

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    env_logger::init();

    use std::path::PathBuf;

    #[cfg(feature = "profiling")]
    puffin::set_scopes_on(true);

    let args: Vec<_> = std::env::args().collect();
    let path = args.get(1).map(PathBuf::from);

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "bucksaw",
        native_options,
        Box::new(|cc| Box::new(App::new(cc, path))),
    )
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

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
