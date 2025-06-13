#![forbid(unsafe_code)]
#![feature(array_windows, default_field_values)]
#![warn(
    clippy::pedantic,
    clippy::unwrap_used,
    clippy::use_self,
    clippy::indexing_slicing,
    clippy::suboptimal_flops,
    clippy::missing_const_for_fn,
    unused_qualifications
)]

use {
    app::AlpackaApp,
    eframe::{NativeOptions, egui::ViewportCommand},
};

mod app;
mod config;
mod packages;
mod query_syntax;
mod util;
mod vercmp;

fn main() {
    let mut app = AlpackaApp::new();
    if let Err(e) = eframe::run_native(
        "alpacka",
        NativeOptions::default(),
        Box::new(move |cc| {
            cc.egui_ctx
                .send_viewport_cmd(ViewportCommand::Maximized(true));
            app.sync_from_config(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    ) {
        eprintln!("Fatal error: {e}");
    }
}
