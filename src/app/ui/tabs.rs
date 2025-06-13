use {
    super::SharedUiState,
    crate::{app::Packages, query_syntax::PkgListQuery},
    eframe::egui,
    egui_dock::TabViewer,
    package::PkgTab,
};

mod color_theme;
pub mod local_pkg_list;
pub mod package;
pub mod remote_pkg_list;
pub mod upgrade_list;

pub struct TabViewState<'pac, 'ui> {
    pub pkgs: Option<&'pac mut Packages>,
    pub ui: &'ui mut SharedUiState,
}

impl TabViewer for TabViewState<'_, '_> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        let Some(pkgs) = self.pkgs.as_mut() else {
            return "<packages not loaded>".into();
        };
        match tab {
            Tab::LocalPkgList(_) => format!(
                "Local packages ({})",
                pkgs.dbs.first().map_or(0, |db| db.pkgs.len())
            )
            .into(),
            Tab::RemotePkgList(_) => format!(
                "Remote packages ({})",
                pkgs.dbs
                    .iter()
                    .skip(1)
                    .map(|db| db.pkgs.len())
                    .sum::<usize>()
            )
            .into(),
            Tab::UpgradeList(_) => "Upgrade list".into(),
            Tab::Pkg(pkg) => format!("📦 {}", pkg.id.display(&pkgs.dbs)).into(),
            Tab::ColorTheme => "🎨 Color theme".into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        let Some(pkgs) = self.pkgs.as_mut() else {
            ui.label("Packages not loaded");
            return;
        };
        match tab {
            Tab::LocalPkgList(state) => local_pkg_list::ui(ui, pkgs, self.ui, state),
            Tab::RemotePkgList(state) => remote_pkg_list::ui(ui, pkgs, self.ui, state),
            Tab::UpgradeList(state) => upgrade_list::ui(ui, pkgs, self.ui, state),
            Tab::Pkg(tab) => package::ui(ui, pkgs, self.ui, tab),
            Tab::ColorTheme => color_theme::ui(ui, &mut self.ui.colorix),
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
            Tab::LocalPkgList(_) | Tab::RemotePkgList(_) | Tab::ColorTheme => false,
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
