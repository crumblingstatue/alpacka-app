use {
    super::remote_pkg_list::installed_label_for_remote_pkg,
    crate::{
        alpm_util::{PkgId, deduped_files},
        app::{
            PacState,
            ui::{SharedUiState, cmd::Cmd},
        },
    },
    alpacka::{Pkg, dep::PkgDepsExt},
    eframe::egui,
    humansize::format_size_i,
    std::process::Command,
};

pub struct PkgTab {
    pub id: PkgId,
    tab: PkgTabTab,
    pub force_close: bool,
    files_filt_string: String,
}

impl PkgTab {
    pub fn new(id: PkgId) -> Self {
        Self {
            id,
            tab: PkgTabTab::default(),
            force_close: false,
            files_filt_string: String::new(),
        }
    }
}

#[derive(PartialEq, Default)]
enum PkgTabTab {
    #[default]
    General,
    Files,
}

pub fn ui(ui: &mut egui::Ui, pac: &PacState, ui_state: &mut SharedUiState, pkg_tab: &mut PkgTab) {
    if ui.input(|inp| {
        let esc = inp.key_pressed(egui::Key::Escape);
        let ctrl_w = inp.modifiers.ctrl && inp.key_pressed(egui::Key::W);
        esc || ctrl_w
    }) {
        pkg_tab.force_close = true;
    }
    let remote = pkg_tab.id.is_remote();
    if remote {
        pkg_ui(
            ui,
            ui_state,
            pkg_tab,
            pac.alpacka_syncdbs
                .iter()
                .flat_map(|db| db.pkgs.iter().map(|pkg| (pkg, db.name.as_str()))),
            &pac.alpaca_local_pkg_list,
            remote,
        );
    } else {
        pkg_ui(
            ui,
            ui_state,
            pkg_tab,
            pac.alpaca_local_pkg_list.iter().map(|pkg| (pkg, "local")),
            &pac.alpaca_local_pkg_list,
            remote,
        );
    }
}

fn pkg_ui<'a, I>(
    ui: &mut egui::Ui,
    ui_state: &mut SharedUiState,
    pkg_tab: &mut PkgTab,
    pkg_list: I,
    local_list: &[Pkg],
    remote: bool,
) where
    I: IntoIterator<Item = (&'a Pkg, &'a str)> + Clone,
{
    match pkg_list
        .clone()
        .into_iter()
        .find(|(pkg, db_name)| pkg_tab.id.matches_pkg(&pkg.desc, db_name))
    {
        Some((pkg, db_name)) => {
            ui.horizontal(|ui| {
                ui.label(format!("{db_name}/"));
                ui.heading(pkg.desc.name.as_str());
                ui.label(pkg.desc.version.as_str());
                if remote {
                    installed_label_for_remote_pkg(ui, ui_state, &pkg.desc, local_list);
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.selectable_value(&mut pkg_tab.tab, PkgTabTab::General, "General");
                ui.selectable_value(&mut pkg_tab.tab, PkgTabTab::Files, "File list");
            });
            ui.separator();
            match pkg_tab.tab {
                PkgTabTab::General => {
                    ui.label(pkg.desc.desc.as_deref().unwrap_or("<no description>"));
                    if let Some(url) = pkg.desc.url.as_deref() {
                        ui.horizontal(|ui| {
                            ui.label("URL");
                            ui.hyperlink(url);
                        });
                    }
                    ui.label(format!(
                        "Installed size: {}",
                        format_size_i(pkg.desc.size, humansize::BINARY)
                    ));
                    let deps = &pkg.desc.depends;
                    ui.heading(format!("Dependencies ({})", deps.len()));
                    if deps.is_empty() {
                        ui.label("<none>");
                    } else {
                        ui.horizontal_wrapped(|ui| {
                            for dep in deps {
                                let resolved =
                                    pkg_list.clone().into_iter().find(|(pkg, _db_name)| {
                                        pkg.desc.name == dep.name
                                            || pkg.desc.provides.iter().any(|dep2| {
                                                // TODO: This might not be correct/enough
                                                dep2.name == dep.name
                                                    && dep2.ver.as_ref().map(|v| &v.ver)
                                                        >= dep.ver.as_ref().map(|v| &v.ver)
                                            })
                                    });
                                match resolved {
                                    Some((pkg, _db_name)) => {
                                        let label = if dep.name == pkg.desc.name {
                                            dep.name.as_str()
                                        } else {
                                            &format!("{} ({})", dep.name, pkg.desc.name)
                                        };
                                        if ui.link(label).clicked() {
                                            ui_state.cmd.push(Cmd::OpenPkgTab(PkgId::qualified(
                                                &pkg_tab.id.db,
                                                pkg.desc.name.as_str(),
                                            )));
                                        }
                                    }
                                    None => {
                                        ui.label(format!("{} (unresolved)", dep.name));
                                    }
                                }
                            }
                        });
                    }
                    let opt_deps = &pkg.desc.opt_depends;
                    ui.heading(format!("Optional dependencies ({})", opt_deps.len()));
                    if opt_deps.is_empty() {
                        ui.label("<none>");
                    } else {
                        for opt_dep in opt_deps {
                            ui.horizontal(|ui| {
                                let installed = local_list
                                    .iter()
                                    .any(|pkg| pkg.desc.name == opt_dep.dep.name);
                                if installed {
                                    if ui.link(opt_dep.dep.name.as_str()).clicked() {
                                        ui_state.cmd.push(Cmd::OpenPkgTab(PkgId::qualified(
                                            &pkg_tab.id.db,
                                            opt_dep.dep.name.as_str(),
                                        )));

                                        if let Some(ver) =
                                            opt_dep.dep.ver.as_ref().map(|v| v.ver.as_str())
                                        {
                                            ui.label(format!("={ver}"));
                                        }
                                    }
                                } else {
                                    ui.label(opt_dep.dep.name.as_str());
                                }
                                if let Some(desc) = &opt_dep.reason {
                                    ui.label(desc.as_str());
                                }
                                if installed {
                                    ui.label("[installed]");
                                }
                            });
                        }
                    }
                    let reqs: Vec<_> = pkg
                        .required_by(pkg_list.clone().into_iter().map(|(pkg, _)| pkg))
                        .collect();
                    ui.heading(format!("Required by ({})", reqs.len()));
                    if reqs.is_empty() {
                        ui.label("<none>");
                    } else {
                        ui.horizontal_wrapped(|ui| {
                            for req in reqs {
                                if ui.link(req.desc.name.as_str()).clicked() {
                                    ui_state.cmd.push(Cmd::OpenPkgTab(PkgId::qualified(
                                        &pkg_tab.id.db,
                                        req.desc.name.as_str(),
                                    )));
                                }
                            }
                        });
                    }
                    let opt_for: Vec<_> = pkg
                        .optional_for(pkg_list.into_iter().map(|(pkg, _)| pkg))
                        .collect();
                    ui.heading(format!("Optional for ({})", opt_for.len()));
                    if opt_for.is_empty() {
                        ui.label("<none>");
                    } else {
                        ui.horizontal_wrapped(|ui| {
                            for pkg in opt_for {
                                if ui.link(pkg.desc.name.as_str()).clicked() {
                                    ui_state.cmd.push(Cmd::OpenPkgTab(PkgId::qualified(
                                        &pkg_tab.id.db,
                                        pkg.desc.name.as_str(),
                                    )));
                                }
                            }
                        });
                    }
                    let provides = &pkg.desc.provides;
                    ui.heading(format!("Provides ({})", provides.len()));
                    for dep in provides {
                        ui.label(dep.name.as_str());
                    }
                }
                PkgTabTab::Files => {
                    ui.add(
                        egui::TextEdit::singleline(&mut pkg_tab.files_filt_string)
                            .hint_text("🔍 Filter"),
                    );
                    let files = &pkg.files;
                    let deduped_files = deduped_files(files).filter(|file| {
                        file.to_ascii_lowercase()
                            .contains(&pkg_tab.files_filt_string.to_ascii_lowercase())
                    });
                    for file in deduped_files {
                        let name = format!("/{}", file);
                        if ui.link(&name).clicked() {
                            Command::new("xdg-open").arg(name).status().unwrap();
                        }
                    }
                }
            }
        }
        None => {
            ui.label("<Unresolved package>");
        }
    }
}
