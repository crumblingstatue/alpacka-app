#![forbid(unsafe_code)]
#![feature(let_chains, array_windows)]

use {app::AlpackaApp, eframe::NativeOptions};

mod alpm_util;
mod app;
mod config;

fn main() -> anyhow::Result<()> {
    let mut app = AlpackaApp::new()?;
    eframe::run_native(
        "alpacka",
        NativeOptions::default(),
        Box::new(move |cc| {
            app.sync_from_config(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )
    .unwrap();
    Ok(())
}
