use {
    super::SharedUiState,
    crate::{
        app::{PkgCache, ui::ico},
        packages::Dbs,
        query_syntax::PkgListQuery,
    },
    eframe::egui,
    egui_dock::TabViewer,
    package::PkgTab,
};

mod color_theme;
pub mod local_pkg_list;
pub mod package;
pub mod remote_pkg_list;
pub mod upgrade_list;

pub struct TabViewState<'pkgs, 'dbs, 'ui> {
    pub pkgs: &'pkgs mut PkgCache,
    pub dbs: Option<&'dbs Dbs>,
    pub ui: &'ui mut SharedUiState,
}

impl TabViewer for TabViewState<'_, '_, '_> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        let Some(dbs) = self.dbs.as_ref() else {
            return "<dbs not loaded>".into();
        };
        match tab {
            Tab::LocalPkgList(_) => format!("Local packages ({})", dbs.local_pkgs().len(),).into(),
            Tab::RemotePkgList(_) => format!(
                "Remote packages ({})",
                dbs.remotes().map(|(_, db)| db.pkgs.len()).sum::<usize>()
            )
            .into(),
            Tab::UpgradeList(_) => "Upgrade list".into(),
            Tab::Pkg(pkg) => format!("{} {}", ico::PKG, pkg.id.display(dbs)).into(),
            Tab::ColorTheme => "🎨 Color theme".into(),
            Tab::LoggerUi => "Log".into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        let Some(dbs) = self.dbs.as_ref() else {
            ui.label("Databases not loaded");
            return;
        };
        match tab {
            Tab::LocalPkgList(state) => local_pkg_list::ui(ui, self.pkgs, dbs, self.ui, state),
            Tab::RemotePkgList(state) => remote_pkg_list::ui(ui, self.pkgs, dbs, self.ui, state),
            Tab::UpgradeList(state) => upgrade_list::ui(ui, self.pkgs, dbs, self.ui, state),
            Tab::Pkg(tab) => package::ui(ui, dbs, self.ui, tab),
            Tab::ColorTheme => color_theme::ui(ui, &mut self.ui.colorix),
            Tab::LoggerUi => egui_logger::logger_ui().show(ui),
        }
    }

    fn closeable(&mut self, tab: &mut Self::Tab) -> bool {
        #[expect(clippy::match_like_matches_macro)]
        match tab {
            Tab::LocalPkgList(_) | Tab::RemotePkgList(_) => false,
            _ => true,
        }
    }

    fn force_close(&mut self, tab: &mut Self::Tab) -> bool {
        match tab {
            Tab::LocalPkgList(_) | Tab::RemotePkgList(_) | Tab::ColorTheme | Tab::LoggerUi => false,
            Tab::UpgradeList(state) => state.force_close,
            Tab::Pkg(pkg_tab) => pkg_tab.force_close,
        }
    }
}

pub enum Tab {
    LocalPkgList(PkgListState),
    RemotePkgList(PkgListState),
    UpgradeList(upgrade_list::State),
    Pkg(PkgTab),
    ColorTheme,
    LoggerUi,
}
impl Tab {
    pub(crate) fn default_tabs() -> Vec<Self> {
        vec![
            Self::LocalPkgList(PkgListState::default()),
            Self::RemotePkgList(PkgListState::default()),
        ]
    }
}

#[derive(Default)]
pub struct PkgListState {
    query_src: String,
    query: PkgListQuery,
}

fn query_focus(ui: &egui::Ui, re: &egui::Response) {
    let [ctrl, shift, num2, f] = ui.input(|inp| {
        [
            inp.modifiers.ctrl,
            inp.modifiers.shift,
            inp.key_pressed(egui::Key::Num2),
            inp.key_pressed(egui::Key::F),
        ]
    });
    if shift && num2 || ctrl && f {
        re.request_focus();
    }
}

fn query_edit(ui: &mut egui::Ui, s: &mut String) -> egui::Response {
    let re = ui.add(egui::TextEdit::singleline(s).hint_text("🔍 Query (ctrl+f, @)"));
    query_focus(ui, &re);
    re
}
