mod ui;

use {
    crate::config::Config,
    alpacka::Pkg,
    egui_colors::{Colorix, tokens::ThemeColor},
    smol_str::SmolStr,
    ui::UiState,
};

pub struct AlpackaApp {
    pac: PacState,
    ui: UiState,
    cfg: Config,
    pac_recv: std::sync::mpsc::Receiver<anyhow::Result<PacState>>,
}

struct SyncDb {
    name: SmolStr,
    pkgs: Vec<Pkg>,
}

/// Used to index into a package list in order to refer to a package efficiently
#[derive(Clone, Copy, Debug)]
pub struct PkgIdx(u32);

impl PkgIdx {
    /// Create from an usize index.
    ///
    /// It's expected that the usize doesn't exceed `u32::MAX` (there won't be billions of packages).
    #[expect(clippy::cast_possible_truncation)]
    fn from_usize(idx: usize) -> Self {
        Self(idx as u32)
    }
    fn to_usize(self) -> usize {
        self.0 as usize
    }
}

#[derive(Default)]
struct PacState {
    local_pkg_list: Vec<alpacka::Pkg>,
    filt_local_pkgs: Vec<PkgIdx>,
    filt_remote_pkgs: Vec<(SmolStr, PkgIdx)>,
    syncdbs: Vec<SyncDb>,
}

impl PacState {
    fn new_spawned() -> std::sync::mpsc::Receiver<anyhow::Result<Self>> {
        let (send, recv) = std::sync::mpsc::channel();
        std::thread::spawn(move || send.send(Self::new()));
        recv
    }
    fn new() -> anyhow::Result<Self> {
        let mut local_db = alpacka::read_local_db()?;
        local_db.sort_by(|a, b| a.desc.name.cmp(&b.desc.name));
        let mut syncdbs = Vec::new();
        for db_name in [
            "core-testing",
            "core",
            "extra-testing",
            "extra",
            "multilib-testing",
            "multilib",
        ] {
            let mut pkgs = alpacka::read_syncdb(db_name)?;
            pkgs.sort_by(|a, b| a.desc.name.cmp(&b.desc.name));
            syncdbs.push(SyncDb {
                name: db_name.into(),
                pkgs,
            });
        }
        Ok(Self {
            filt_local_pkgs: (0..local_db.len()).map(PkgIdx::from_usize).collect(),
            local_pkg_list: local_db,
            filt_remote_pkgs: {
                let mut vec = Vec::new();
                for db in &syncdbs {
                    for i in 0..db.pkgs.len() {
                        vec.push((db.name.clone(), PkgIdx::from_usize(i)));
                    }
                }
                vec
            },
            syncdbs,
        })
    }
}

impl AlpackaApp {
    pub fn new() -> Self {
        Self {
            pac: PacState::default(),
            ui: UiState::default(),
            cfg: Config::load_or_default(),
            pac_recv: PacState::new_spawned(),
        }
    }
    pub fn sync_from_config(&mut self, egui_ctx: &eframe::egui::Context) {
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
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        ui::top_panel_ui(self, ctx);
        ui::central_panel_ui(self, ctx);
        ui::cmd::process_cmds(self, ctx);
        ui::modals(self, ctx);
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.sync_to_config();
        if let Err(e) = self.cfg.save() {
            eprintln!("Failed to save config: {e}");
        }
    }
}
