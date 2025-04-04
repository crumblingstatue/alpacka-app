use {
    super::remote_pkg_list::installed_label_for_remote_pkg,
    crate::{
        app::{
            Packages,
            ui::{SharedUiState, cmd::Cmd},
        },
        packages::{Db, DbIdx, PkgIdx, PkgRef},
        util::deduped_files,
    },
    alpacka::Pkg,
    eframe::egui,
    humansize::format_size_i,
    std::process::Command,
};

pub struct PkgTab {
    pub id: PkgRef,
    tab: PkgTabTab,
    pub force_close: bool,
    files_filt_string: String,
}

impl PkgTab {
    pub fn new(id: PkgRef) -> Self {
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

pub fn ui(ui: &mut egui::Ui, pac: &Packages, ui_state: &mut SharedUiState, pkg_tab: &mut PkgTab) {
    if ui.input(|inp| {
        let esc = inp.key_pressed(egui::Key::Escape);
        let ctrl_w = inp.modifiers.ctrl && inp.key_pressed(egui::Key::W);
        esc || ctrl_w
    }) {
        pkg_tab.force_close = true;
    }
    pkg_ui(ui, ui_state, pkg_tab, &pac.dbs);
}

fn db_name_is_arch(name: &str) -> bool {
    [
        "core",
        "extra",
        "core-testing",
        "extra-testing",
        "multilib",
        "multilib-testing",
    ]
    .into_iter()
    .any(|repo| name == repo)
}

fn pkg_ui(ui: &mut egui::Ui, ui_state: &mut SharedUiState, pkg_tab: &mut PkgTab, dbs: &[Db]) {
    let (db_id, pkg_id) = pkg_tab.id.into_components();
    let Some(db) = dbs.get(db_id.to_usize()) else {
        ui.label("Unresolved database");
        return;
    };
    let db_name = &db.name;
    let remote = pkg_tab.id.is_remote();
    match db.pkgs.get(pkg_id.to_usize()) {
        Some(pkg) => {
            ui.horizontal(|ui| {
                ui.label(format!("{db_name}/"));
                ui.heading(pkg.desc.name.as_str());
                ui.label(pkg.desc.version.as_str());
                if remote {
                    installed_label_for_remote_pkg(ui, ui_state, &pkg.desc, &dbs[0].pkgs);
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
                    general_tab_ui(ui, ui_state, dbs, pkg, db_name);
                }
                PkgTabTab::Files => files_tab_ui(ui, ui_state, pkg_tab, pkg),
            }
        }
        None => {
            ui.label("<Unresolved package>");
        }
    }
}

fn files_tab_ui(ui: &mut egui::Ui, ui_state: &mut SharedUiState, pkg_tab: &mut PkgTab, pkg: &Pkg) {
    ui.add(egui::TextEdit::singleline(&mut pkg_tab.files_filt_string).hint_text("🔍 Filter"));
    let files = &pkg.files;
    let deduped_files = deduped_files(files).filter(|file| {
        file.to_ascii_lowercase()
            .contains(&pkg_tab.files_filt_string.to_ascii_lowercase())
    });
    for file in deduped_files {
        let name = format!("/{file}");
        if ui.link(&name).clicked()
            && let Err(e) = Command::new("xdg-open").arg(name).status()
        {
            ui_state.error_popup = Some(e.to_string());
        }
    }
}

fn general_tab_ui(
    ui: &mut egui::Ui,
    ui_state: &mut SharedUiState,
    dbs: &[Db],
    pkg: &Pkg,
    db_name: &str,
) {
    ui.label(pkg.desc.desc.as_deref().unwrap_or("<no description>"));
    if let Some(url) = pkg.desc.url.as_deref() {
        ui.horizontal(|ui| {
            ui.label("Upstream URL");
            ui.hyperlink(url);
        });
    }
    if db_name_is_arch(db_name) {
        ui.horizontal(|ui| {
            ui.label("Arch package URL");
            ui.hyperlink(format!(
                "https://archlinux.org/packages/{db_name}/{}/{}",
                pkg.desc.arch, pkg.desc.name
            ));
        });
        ui.horizontal(|ui| {
            ui.label("Package Source URL");
            ui.hyperlink(format!(
                "https://gitlab.archlinux.org/archlinux/packaging/packages/{}",
                pkg.desc.name
            ));
        });
    }
    ui.label(format!(
        "Installed size: {}",
        format_size_i(pkg.desc.size, humansize::BINARY)
    ));
    deps_ui(ui, ui_state, dbs, pkg);
    opt_deps_ui(ui, ui_state, &dbs[0].pkgs, pkg);
    required_by_ui(ui, ui_state, pkg, dbs);
    optional_for_ui(ui, ui_state, pkg, dbs);
    provides_ui(ui, pkg);
}

fn required_by_ui(ui: &mut egui::Ui, ui_state: &mut SharedUiState, pkg: &Pkg, dbs: &[Db]) {
    let mut reqs = Vec::new();
    for (db_i, db) in dbs.iter().enumerate() {
        for (pkg_i, pkg2) in db.pkgs.iter().enumerate() {
            if alpacka::dep::pkg_matches_dep(&pkg.desc, &pkg2.desc) {
                reqs.push((
                    PkgRef::from_components(DbIdx::from_usize(db_i), PkgIdx::from_usize(pkg_i)),
                    pkg2,
                ));
            }
        }
    }
    ui.heading(format!("Required by ({})", reqs.len()));
    if reqs.is_empty() {
        ui.label("<none>");
    } else {
        ui.horizontal_wrapped(|ui| {
            for (ref_, req) in reqs {
                if ui.link(req.desc.name.as_str()).clicked() {
                    ui_state.cmd.push(Cmd::OpenPkgTab(ref_));
                }
            }
        });
    }
}

fn provides_ui(ui: &mut egui::Ui, pkg: &Pkg) {
    let provides = &pkg.desc.provides;
    ui.heading(format!("Provides ({})", provides.len()));
    for dep in provides {
        ui.label(dep.name.as_str());
    }
}

fn optional_for_ui(ui: &mut egui::Ui, ui_state: &mut SharedUiState, pkg: &Pkg, dbs: &[Db]) {
    let opt_for = pkgs_that_optionally_depend_on(pkg, dbs);
    ui.heading(format!("Optional for ({})", opt_for.len()));
    if opt_for.is_empty() {
        ui.label("<none>");
    } else {
        ui.horizontal_wrapped(|ui| {
            for (ref_, pkg) in opt_for {
                if ui.link(pkg.desc.name.as_str()).clicked() {
                    ui_state.cmd.push(Cmd::OpenPkgTab(ref_));
                }
            }
        });
    }
}

fn pkgs_that_optionally_depend_on<'db>(
    dependency: &Pkg,
    dbs: &'db [Db],
) -> Vec<(PkgRef, &'db Pkg)> {
    let mut pkgs = Vec::new();
    for (db_i, db) in dbs.iter().enumerate() {
        for (pkg_i, pkg) in db.pkgs.iter().enumerate() {
            if pkg_optionally_depends_on(pkg, dependency) {
                pkgs.push((
                    PkgRef::from_components(DbIdx::from_usize(db_i), PkgIdx::from_usize(pkg_i)),
                    pkg,
                ));
            }
        }
    }
    pkgs
}

fn pkg_optionally_depends_on(pkg: &Pkg, dependency: &Pkg) -> bool {
    alpacka::dep::pkg_matches_opt_dep(&dependency.desc, &pkg.desc)
}

fn opt_deps_ui(ui: &mut egui::Ui, ui_state: &mut SharedUiState, local_list: &[Pkg], pkg: &Pkg) {
    let opt_deps = &pkg.desc.opt_depends;
    ui.heading(format!("Optional dependencies ({})", opt_deps.len()));
    if opt_deps.is_empty() {
        ui.label("<none>");
    } else {
        for opt_dep in opt_deps {
            ui.horizontal(|ui| {
                let installed = local_list.iter().enumerate().find_map(|(i, pkg)| {
                    (pkg.desc.name == opt_dep.dep.name).then_some(PkgIdx::from_usize(i))
                });
                if let Some(ref_) = installed {
                    if ui.link(opt_dep.dep.name.as_str()).clicked() {
                        ui_state.cmd.push(Cmd::OpenPkgTab(PkgRef::local(ref_)));

                        if let Some(ver) = opt_dep.dep.ver.as_ref().map(|v| v.ver.as_str()) {
                            ui.label(format!("={ver}"));
                        }
                    }
                } else {
                    ui.label(opt_dep.dep.name.as_str());
                }
                if let Some(desc) = &opt_dep.reason {
                    ui.label(desc.as_str());
                }
                if installed.is_some() {
                    ui.label("[installed]");
                }
            });
        }
    }
}

fn deps_ui(ui: &mut egui::Ui, ui_state: &mut SharedUiState, dbs: &[Db], pkg: &Pkg) {
    let deps = &pkg.desc.depends;
    ui.heading(format!("Dependencies ({})", deps.len()));
    if deps.is_empty() {
        ui.label("<none>");
    } else {
        ui.horizontal_wrapped(|ui| {
            for dep in deps {
                match resolve_dep(dep, dbs) {
                    Some((ref_, pkg)) => {
                        let label = if dep.name == pkg.desc.name {
                            dep.name.as_str()
                        } else {
                            &format!("{} ({})", dep.name, pkg.desc.name)
                        };
                        if ui.link(label).clicked() {
                            ui_state.cmd.push(Cmd::OpenPkgTab(ref_));
                        }
                    }
                    None => {
                        ui.label(format!("{} (unresolved)", dep.name));
                    }
                }
            }
        });
    }
}

fn resolve_dep<'db>(dep: &alpacka::Depend, dbs: &'db [Db]) -> Option<(PkgRef, &'db Pkg)> {
    for (db_i, db) in dbs.iter().enumerate() {
        for (pkg_i, pkg) in db.pkgs.iter().enumerate() {
            if pkg.desc.name == dep.name
                || pkg.desc.provides.iter().any(|dep2| {
                    // TODO: This might not be correct/enough
                    dep2.name == dep.name
                        && dep2.ver.as_ref().map(|v| &v.ver) >= dep.ver.as_ref().map(|v| &v.ver)
                })
            {
                return Some((
                    PkgRef::from_components(DbIdx::from_usize(db_i), PkgIdx::from_usize(pkg_i)),
                    pkg,
                ));
            }
        }
    }
    None
}
