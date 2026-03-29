mod ui;

use {
    crate::{
        config::Config,
        packages::{Dbs, LoadRecv, PkgCache},
    },
    eframe::egui,
    egui_colors::{Colorix, tokens::ThemeColor},
    std::sync::Arc,
    ui::UiState,
};

pub struct AlpackaApp {
    pkgs: PkgCache,
    dbs: Option<Arc<Dbs>>,
    ui: UiState,
    cfg: Config,
    load_recv: LoadRecv,
    open_upgrade_window: bool,
}

impl AlpackaApp {
    pub fn new() -> Self {
        Self {
            pkgs: PkgCache::default(),
            dbs: None,
            ui: UiState::default(),
            cfg: Config::load_or_default(),
            load_recv: crate::packages::spawn_load_thread(),
            open_upgrade_window: false,
        }
    }
    pub fn sync_from_config(&mut self, egui_ctx: &egui::Context) {
        if let Some(color_theme) = &self.cfg.color_theme {
            self.ui.shared.colorix = Some(Colorix::global(
                egui_ctx,
                color_theme.map(ThemeColor::Custom),
            ));
        }
    }
    fn sync_to_config(&mut self) {
        self.cfg.color_theme = self
            .ui
            .shared
            .colorix
            .as_ref()
            .map(|colorix| colorix.theme().map(|theme| theme.rgb()));
    }
}

impl eframe::App for AlpackaApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui::top_panel_ui(self, ui);
        ui::central_panel_ui(self, ui);
        ui::cmd::process_cmds(self, ui);
        ui::modals(self, ui);
        if ui.input(|i| i.viewport().close_requested()) {
            if self.ui.is_pacman_running() {
                ui.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
        }
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.sync_to_config();
        if let Err(e) = self.cfg.save() {
            log::error!("Failed to save config: {e}");
        }
    }
}
