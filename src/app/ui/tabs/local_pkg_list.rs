use {
    super::{PkgListQuery, PkgListState},
    crate::{
        app::{
            PkgCache,
            ui::{SharedUiState, cmd::Cmd},
        },
        packages::{Dbs, PkgIdx, PkgRef},
    },
    alpacka::InstallReason,
    eframe::egui,
    egui_extras::{Column, TableBuilder},
};

pub fn ui(
    ui: &mut egui::Ui,
    pkgs: &mut PkgCache,
    dbs: &Dbs,
    ui_state: &mut SharedUiState,
    tab_state: &mut PkgListState,
) {
    egui::TopBottomPanel::top("top_panel").show_inside(ui, |ui| {
        ui.horizontal(|ui| {
            if super::query_edit(ui, &mut tab_state.query_src).changed() {
                tab_state.query = PkgListQuery::compile(&tab_state.query_src);
                pkgs.filt_local_pkgs =
                    dbs.local_pkgs()
                        .iter()
                        .enumerate()
                        .filter_map(|(i, pkg)| {
                            if tab_state.query.flags.explicitly_installed
                                && !matches!(pkg.desc.install_reason, InstallReason::Explicit)
                            {
                                return None;
                            }
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
                            .then_some(PkgIdx::from_usize(i))
                        })
                        .collect();
            }
            ui.spacing();
            ui.label(format!("{} packages listed", pkgs.filt_local_pkgs.len()));
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
            body.rows(22.0, pkgs.filt_local_pkgs.len(), |mut row| {
                let Some(idx) = pkgs.filt_local_pkgs.get(row.index()) else {
                    row.col(|ui| {
                        ui.label("<Unresolved package index>");
                    });
                    return;
                };
                let Some(pkg) = dbs.resolve_local(*idx) else {
                    row.col(|ui| {
                        ui.label("<Unresolved package>");
                    });
                    return;
                };
                row.col(|ui| {
                    let mut text = egui::RichText::new("📦");
                    let hover_text;
                    if matches!(pkg.desc.install_reason, InstallReason::Explicit) {
                        hover_text = "Explicitly installed";
                        text = text.strong();
                    } else {
                        hover_text = "Installed as a depdenency";
                        text = text.weak();
                    }
                    ui.label(text)
                        .on_hover_text(hover_text)
                        .on_hover_cursor(egui::CursorIcon::Help);
                    let re = ui.link(pkg.desc.name.as_str());
                    re.context_menu(|ui| {
                        if ui
                            .button(format!("🗑 Remove `{}` (-Rscn)", pkg.desc.name))
                            .clicked()
                        {
                            ui.close_menu();
                            ui_state.cmd.push(Cmd::Rscn(pkg.desc.name.clone()));
                        }
                    });
                    if re.clicked() {
                        ui_state.cmd.push(Cmd::OpenPkgTab(PkgRef::local(*idx)));
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

pub fn pkg_list_table_builder(ui: &'_ mut egui::Ui) -> TableBuilder<'_> {
    TableBuilder::new(ui)
        .column(Column::auto())
        .column(Column::auto())
        .column(Column::remainder())
        .auto_shrink(false)
        .striped(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
}
