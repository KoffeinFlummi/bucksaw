bucksaw
=======

Tool for looking inside flight logs of drones using the Cleanflight family of firmwares (Betaflight, INAV). Primary use case is analyzing vibrations and tune performance. Still work in progress.

A Web Assembly version is publicly accessible at: https://bucksaw.koffeinflummi.de/

The performance on the web version is obviously not quite up to par with the native version, but it requires no setup and works even on mobile. Be careful with larger FFT sizes, or just larger log files in general. All processing is done in Web Assembly in your browser, so your log file never leaves your device.

![](https://raw.githubusercontent.com/KoffeinFlummi/bucksaw/master/assets/screenshots.png)

# Setup

## Native

- Install Rust (https://rustup.rs/)
- `$ cargo run` to run debug build.
- `$ cargo install --path .` to install.

## Web Assembly

- `$ cargo install trunk`
- `$ trunk serve --open` to start a local development server and open the result in the browser.
- `$ trunk build --release` to build release files. Copy `/dist` folder to webserver of choice.
