use log::info;
use rfd::FileDialog;
use std::{collections::HashMap, path::PathBuf};

use crate::{
    get_children_recursive, get_hierarchy, get_parents, get_records, TweakRecord, TweakRecordVm,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    // Example stuff:
    vms: Option<HashMap<String, TweakRecordVm>>,
    gamepath: PathBuf,

    #[serde(skip)]
    show_setup: bool,

    // cache packages
    #[serde(skip)]
    packages: Vec<String>,
    #[serde(skip)]
    filtered_packages: Vec<String>,
    // regenerate token
    regenerate_filtered_packages: bool,
    // filters
    #[serde(skip)]
    filter: String,
    #[serde(skip)]
    filter_package: String,
    #[serde(skip)]
    query: String,
    // selected
    #[serde(skip)]
    current_record_name: String,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            vms: None,
            packages: vec![],
            filtered_packages: vec![],
            regenerate_filtered_packages: true,
            show_setup: false,
            gamepath: PathBuf::from(""),
            filter: "".to_owned(),
            filter_package: "".to_owned(),
            query: "".to_owned(),
            current_record_name: "".to_owned(),
        }
    }
}

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        // menu bar
        self.menu_view(ctx, _frame);

        let healthy = self.check_health();
        if !healthy || self.show_setup {
            // show only game path input dialogue
            self.setup_view(ctx);
            return;
        }

        if self.packages.is_empty() {
            if let Some(vms) = &self.vms {
                self.packages = get_package_names(&vms.keys().collect::<Vec<_>>());
            }
        }

        // filter the packages if query is active
        if self.query.is_empty() && self.regenerate_filtered_packages {
            self.filtered_packages = self.packages.clone();
            self.regenerate_filtered_packages = false;
        }

        // main ui
        self.left_panel(ctx);
        self.main_panel(ctx);
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }

    /// Check if setup was succesfull
    pub fn check_health(&mut self) -> bool {
        if !PathBuf::from(&self.gamepath).exists() {
            return false;
        }

        if let Some(vms) = &self.vms {
            return !vms.is_empty();
        }

        false
    }

    /// Generates the viewmodels from the tweak source path
    pub fn first_setup(&mut self) {
        let path = PathBuf::from(&self.gamepath);
        if !PathBuf::from(&self.gamepath).exists() {
            return;
        }

        let records: Vec<TweakRecord> = get_records(&path);
        info!("Found {} records", records.len());

        let vms = get_hierarchy(records);
        info!("Found {} vms", vms.len());

        self.vms = Some(vms);
    }

    /// View for the app setup
    fn setup_view(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("Tweak Utils");
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Tweak folder path: ");
                ui.label(self.gamepath.display().to_string());
                if ui.button("...").clicked() {
                    let dir = FileDialog::new().set_directory("/").pick_folder();
                    if let Some(folder) = dir {
                        self.gamepath = folder;
                    }
                }
            });

            if ui.button("Generate").clicked() {
                self.first_setup();
                self.show_setup = false;
            }
        });
    }

    /// View for the app menu bar
    fn menu_view(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Setup").clicked() {
                        self.show_setup = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::global_theme_preference_buttons(ui);
                    ui.add_space(16.0);

                    ui.label(format!("v{}", VERSION));
                });
            });
        });
    }

    /// View for the left records list panel
    fn left_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("left_panel").show(ctx, |ui| {
            ui.heading("Tweak Records");

            if let Some(vms) = &self.vms {
                ui.horizontal(|ui| {
                    ui.label("Search all: ");
                    let response = ui.text_edit_singleline(&mut self.query);
                    if response.changed() {
                        // text changed: set regen token
                        self.regenerate_filtered_packages = true;
                    }
                    if ui.button("x").clicked() {
                        self.query.clear();
                    }
                });
                ui.separator();

                ui.horizontal(|ui| {
                    // text filter
                    ui.label("Filter: ");
                    ui.text_edit_singleline(&mut self.filter);
                    if ui.button("x").clicked() {
                        self.filter.clear();
                    }
                    // package filter
                    ui.separator();
                    egui::ComboBox::from_id_salt("cb_package")
                        .wrap_mode(egui::TextWrapMode::Truncate)
                        .selected_text(format!("{:?}", &mut self.filter_package))
                        .show_ui(ui, |ui| {
                            for p in &self.filtered_packages {
                                ui.selectable_value(&mut self.filter_package, p.clone(), p);
                            }
                        });
                });
                ui.separator();

                // hierarchy view
                if self.query.is_empty() {
                    let mut top_level_records = vms
                        .iter()
                        .filter(|p| p.1.parent.is_none())
                        .collect::<Vec<_>>();
                    top_level_records.sort_by_key(|k| k.0);

                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            egui::Grid::new("hierarchy_grid")
                                .num_columns(1)
                                //.striped(true)
                                .show(ui, |ui| {
                                    for (name, vm) in top_level_records {
                                        if !self.filter.is_empty()
                                            && !name
                                                .to_lowercase()
                                                .contains(&self.filter.to_lowercase())
                                        {
                                            continue;
                                        }

                                        add_tree_node(
                                            ui,
                                            vm,
                                            name,
                                            vms,
                                            &mut self.current_record_name,
                                        );

                                        ui.end_row();
                                    }
                                });
                        });
                }
                // query view
                else {
                    let mut result = vms
                        .iter()
                        .filter(|p| p.0.to_lowercase().contains(&self.query.to_lowercase()))
                        .collect::<Vec<_>>();
                    result.sort_by_key(|k| k.0);

                    if self.regenerate_filtered_packages {
                        self.filtered_packages =
                            get_package_names(&result.iter().map(|f| f.0).collect::<Vec<_>>());
                        self.regenerate_filtered_packages = false;
                    }

                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            egui::Grid::new("query_grid")
                                .num_columns(1)
                                //.striped(true)
                                .show(ui, |ui| {
                                    for (name, _vm) in result {
                                        // filter by package
                                        if !self.filter_package.is_empty()
                                            && !name.to_lowercase().contains(&format!(
                                                "{}.",
                                                self.filter_package.to_lowercase()
                                            ))
                                        {
                                            continue;
                                        }
                                        // filter by name
                                        if !self.filter.is_empty()
                                            && !name
                                                .to_lowercase()
                                                .contains(&self.filter.to_lowercase())
                                        {
                                            continue;
                                        }

                                        if ui
                                            .add(egui::Label::new(name).sense(egui::Sense::click()))
                                            .clicked()
                                        {
                                            self.current_record_name = name.to_owned();
                                        }
                                        ui.end_row();
                                    }
                                });
                        });
                }
            }
        });
    }

    /// View for the record main (details)
    fn main_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("Details");
            ui.separator();

            if let Some(vms) = &self.vms {
                if let Some(record) = vms.get(&self.current_record_name) {
                    // breadcrumb
                    let parents = get_parents(vms, &self.current_record_name);

                    egui::ScrollArea::horizontal().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            for (i, p) in parents.iter().enumerate() {
                                if ui.button(p).clicked() {
                                    self.current_record_name = p.to_string();
                                }
                                if i < parents.len() - 1 {
                                    ui.label(">");
                                }
                            }
                        });
                    });

                    // record name
                    ui.horizontal(|ui| {
                        ui.label("Record: ");
                        ui.text_edit_singleline(&mut self.current_record_name.as_str());
                        // parent name
                        if let Some(parent) = &record.parent {
                            ui.label(" : ");
                            if ui.button(parent).clicked() {
                                self.current_record_name = parent.to_string();
                            }
                        }
                    });

                    // get details
                    ui.separator();

                    if record.children.is_some() {
                        // list in ui
                        egui::CollapsingHeader::new("Children records").show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .auto_shrink([false; 2])
                                .show(ui, |ui| {
                                    for child_name in
                                        get_children_recursive(vms, &self.current_record_name)
                                    {
                                        if let Some(_child_vm) = vms.get(child_name.as_str()) {
                                            if ui.button(&child_name).clicked() {
                                                // navigate to record
                                                self.current_record_name = child_name.to_string();
                                            }
                                        }
                                    }
                                });
                        });

                        // list as json
                        egui::CollapsingHeader::new("Children records (text)").show(ui, |ui| {
                            ui.horizontal(|ui| {
                                if ui.button("Copy to clipboard").clicked() {
                                    let result =
                                        get_children_recursive(vms, &self.current_record_name);
                                    if let Ok(json) = serde_json::to_string_pretty(&result) {
                                        ui.output_mut(|o| o.copied_text = json);
                                    }
                                }
                                if ui.button("Generate tweakXL instances").clicked() {
                                    if let Some(record_name) =
                                        self.current_record_name.split('.').nth(1)
                                    {
                                        if let Some(package) =
                                            self.current_record_name.split('.').nth(0)
                                        {
                                            let mut text =
                                                format!("{}.$(name):\n  $instances:\n", package);

                                            // add self
                                            text += format!("    - {{ name: {} }}\n", record_name)
                                                .as_str();
                                            // add children
                                            let children = get_children_recursive(
                                                vms,
                                                &self.current_record_name,
                                            );
                                            for c in children {
                                                if let Some(child_record_name) = c.split('.').nth(1)
                                                {
                                                    if let Some(child_package_name) =
                                                        c.split('.').nth(0)
                                                    {
                                                        if child_package_name == package {
                                                            text += format!(
                                                                "    - {{ name: {} }}\n",
                                                                child_record_name
                                                            )
                                                            .as_str();
                                                        }
                                                    }
                                                }
                                            }
                                            ui.output_mut(|o| o.copied_text = text);
                                        }
                                    }
                                }
                            });

                            egui::ScrollArea::vertical()
                                .auto_shrink([false; 2])
                                .show(ui, |ui| {
                                    let result =
                                        get_children_recursive(vms, &self.current_record_name);
                                    if let Ok(json) = serde_json::to_string_pretty(&result) {
                                        //egui::Frame::none().fill(egui::Color32::DARK_GRAY).show(
                                        //    ui,
                                        //    |ui| {
                                        ui.add_sized(
                                            ui.available_size(),
                                            egui::TextEdit::multiline(&mut json.as_str()),
                                        );
                                        //    },
                                        //);
                                    }
                                });
                        });
                    }
                }
            }
        });
    }
}

fn get_package_names(vms: &[&String]) -> Vec<String> {
    let mut packages = vms
        .iter()
        .filter(|p| p.contains('.'))
        .filter_map(|f| f.split('.').next())
        .map(|f| f.to_owned())
        .collect::<Vec<_>>();
    packages.sort();
    packages.dedup();
    packages
}

/// Adds an expandable node to the ui tree recursively
fn add_tree_node(
    ui: &mut egui::Ui,
    vm: &TweakRecordVm,
    name: &String,
    vms: &HashMap<String, TweakRecordVm>,
    current_record_name: &mut String,
) {
    if let Some(children) = &vm.children {
        let r = egui::CollapsingHeader::new(name).show(ui, |ui| {
            for child_name in children {
                if let Some(child_vm) = vms.get(child_name) {
                    add_tree_node(ui, child_vm, child_name, vms, current_record_name);
                }
            }
        });
        if r.header_response.clicked() {
            *current_record_name = name.to_owned();
        }
    } else if ui
        .add(egui::Label::new(name).sense(egui::Sense::click()))
        .clicked()
    {
        // show details
        *current_record_name = name.to_owned();
    }
}
