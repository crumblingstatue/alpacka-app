use {
    super::{PkgListQuery, PkgListState},
    crate::{
        alpm_util::PkgId,
        app::{
            PacState,
            ui::{SharedUiState, cmd::Cmd},
        },
    },
    eframe::egui,
    egui_extras::{Column, TableBuilder},
};

pub fn ui(
    ui: &mut egui::Ui,
    pac: &mut PacState,
    ui_state: &mut SharedUiState,
    tab_state: &mut PkgListState,
) {
    egui::TopBottomPanel::top("top_panel").show_inside(ui, |ui| {
        ui.horizontal(|ui| {
            let re =
                ui.add(egui::TextEdit::singleline(&mut tab_state.query_src).hint_text("🔍 Query"));
            if ui.input(|inp| inp.key_pressed(egui::Key::Num2) && inp.modifiers.shift) {
                re.request_focus();
            }
            if re.changed() {
                tab_state.query = PkgListQuery::compile(&tab_state.query_src);
                pac.alpacka_filt_pkg_list =
                    pac.alpaca_local_pkg_list
                        .iter()
                        .enumerate()
                        .filter_map(|(i, pkg)| {
                            let filt_lo = tab_state.query.string.to_ascii_lowercase();
                            (pkg.desc.name.contains(&filt_lo)
                                || pkg.desc.desc.as_ref().is_some_and(|desc| {
                                    desc.to_ascii_lowercase().contains(&filt_lo)
                                })
                                || pkg
                                    .desc
                                    .provides
                                    .iter()
                                    .any(|dep| dep.name.contains(&filt_lo)))
                            .then_some(i)
                        })
                        .collect();
            }
            ui.spacing();
            ui.label(format!(
                "{} packages listed",
                pac.alpacka_filt_pkg_list.len()
            ));
        });
        ui.add_space(4.0);
    });
    pkg_list_table_builder(ui)
        .header(18.0, |mut row| {
            row.col(|ui| {
                ui.label("Name");
            });
            row.col(|ui| {
                ui.label("Version");
            });
            row.col(|ui| {
                ui.label("Description");
            });
        })
        .body(|mut body| {
            body.ui_mut().style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            body.rows(22.0, pac.alpacka_filt_pkg_list.len(), |mut row| {
                let idx = pac.alpacka_filt_pkg_list[row.index()];
                let pkg = &pac.alpaca_local_pkg_list[idx];
                row.col(|ui| {
                    if ui.link(pkg.desc.name.as_str()).clicked() {
                        ui_state
                            .cmd
                            .push(Cmd::OpenPkgTab(PkgId::local(pkg.desc.name.as_str())));
                    }
                });
                row.col(|ui| {
                    ui.label(pkg.desc.version.as_str());
                });
                row.col(|ui| {
                    ui.label(pkg.desc.desc.as_deref().unwrap_or("<missing description>"));
                });
            });
        });
}

pub fn pkg_list_table_builder(ui: &mut egui::Ui) -> TableBuilder {
    TableBuilder::new(ui)
        .column(Column::auto())
        .column(Column::auto())
        .column(Column::remainder())
        .auto_shrink(false)
        .striped(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
}
