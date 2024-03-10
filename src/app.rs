use egui::{TextEdit, Vec2, Window};
use egui_extras::{Column, TableBuilder};
use indexmap::{indexmap, IndexMap};
use regex::Regex;
use subprocess::Exec;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct MyApp {
    part: String,
    exe_path: String,
    download_datasheet: bool,
    #[serde(skip)]
    is_init: bool,
    #[serde(skip)]
    search_good: bool,
    #[serde(skip)]
    current_part: IndexMap<String, String>,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            part: "C11702".to_owned(),
            exe_path: "JLC2KiCadLib".to_owned(),
            download_datasheet: true,
            is_init: false,
            search_good: true,
            current_part: indexmap! {},
        }
    }
}

impl MyApp {
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

    fn get_part(search_term: &str) -> Option<IndexMap<String, String>> {
        let term = search_term.trim();
        let re_jlc = Regex::new(r"/(C\d+)$").unwrap();
        let re_lcsc = Regex::new(r"_(C\d+)[^/]*\.html$").unwrap();
        let re_lcscnumber = Regex::new(r"^C(\d+)$").unwrap();
        let mut lcscnumber = "";

        // case one, we got passed a URL
        if term.contains("http") {
            if term.contains("jlcpcb.com") {
                if let Some(captures) = re_jlc.captures(term) {
                    lcscnumber = captures.get(1).unwrap().as_str();
                }
            } else if term.contains("lcsc.com") {
                if let Some(captures) = re_lcsc.captures(term) {
                    lcscnumber = captures.get(1).unwrap().as_str();
                }
            }
        // case two, it's the number directly
        } else if term.starts_with("C") {
            lcscnumber = term;
        }

        // ensure we only make requests if what we have looks like an LCSC number and can work,
        // also saves us from urlencoding and such because it will only ever be "C" followed by some numbers
        if re_lcscnumber.is_match(lcscnumber) {
            let client = reqwest::blocking::Client::new();
            let res_or_err = client
                .get(format!("https://cart.jlcpcb.com/shoppingCart/smtGood/getComponentDetail?componentCode={}", lcscnumber))
                .header(reqwest::header::ACCEPT, "application/json")
                .send();
            if let Ok(res) = res_or_err {
                let res_status = res.status();
                if res_status.is_success() {
                    let res_text = res
                        .text()
                        .expect("Issue decoding received response from JLCPCB.");
                    let json: serde_json::Value =
                        serde_json::from_str(&res_text).expect("Issue parsing search result JSON.");
                    println!("{}", json);
                    let parameters = indexmap! {
                        "componentCode" => "Component Code",
                        "firstTypeNameEn" => "Primary Category",
                        "secondTypeNameEn" => "Secondary Category",
                        "componentBrandEn" => "Brand",
                        "componentName" => "Full Name",
                        "componentDesignator" => "Designator",
                        "componentModelEn" => "Model",
                        "componentSpecificationEn" => "Specification",
                        "assemblyProcess" => "Assembly Process",
                        "describe" => "Description",
                        "matchedPartDetail" => "Details",
                        "stockCount" => "Stock",
                        "leastNumber" => "Minimal Quantity",
                        "leastNumberPrice" => "Minimum Price",
                    };

                    // there is a case where we get a fully valid response in an HTML
                    // and JSON sense but it tells us via a code field in the JSON
                    // that no part could be found, in that case we exit early
                    if let Some(code) = json.get("code") {
                        if code != 200 {
                            return None;
                        }
                    }

                    // if the data section is there as expected, we start taking it apart
                    if let Some(data) = json.get("data") {
                        let mut tabledata: IndexMap<String, String> = indexmap! {};

                        // determine if it is a JLCPCB basic or extended assembly part
                        if let Some(parttype) = data.get("componentLibraryType") {
                            if parttype == "base" {
                                tabledata.insert("Type".to_owned(), "Basic".to_owned());
                            } else if parttype == "expand" {
                                tabledata.insert("Type".to_owned(), "Extended".to_owned());
                            }
                        }

                        // now pretty-format the parameters that should always be there
                        for (key, title) in parameters {
                            if let Some(value) = data.get(key) {
                                tabledata.insert(
                                    title.to_owned(),
                                    value.to_string().trim_matches('"').to_owned(),
                                );
                            }
                        }

                        // now the component specific attributes, these are in a nested array within
                        // the JSON and vary by component
                        if let Some(attributes) = data.get("attributes") {
                            if let Some(array) = attributes.as_array() {
                                for attribute in array {
                                    if let Some(name) = attribute.get("attribute_name_en") {
                                        if let Some(value) = attribute.get("attribute_value_name") {
                                            tabledata.insert(
                                                name.to_string().trim_matches('"').to_owned(),
                                                value.to_string().trim_matches('"').to_owned(),
                                            );
                                        }
                                    }
                                }
                            }
                        }

                        // here we gather metadata for the image and datasheet URLs
                        if let Some(imagelist) = data.get("imageList") {
                            if let Some(imagevec) = imagelist.as_array() {
                                for (idx, i) in imagevec.iter().enumerate() {
                                    if let Some(imageurl) = i.get("productBigImage") {
                                        tabledata.insert(
                                            format!("meta_image{}", idx),
                                            imageurl.to_string().trim_matches('"').to_owned(),
                                        );
                                    }
                                }
                            }
                        }
                        if let Some(datasheeturl) = data.get("dataManualUrl") {
                            tabledata.insert(
                                "meta_datasheeturl".to_owned(),
                                datasheeturl.to_string().trim_matches('"').to_owned(),
                            );
                        }
                        return Some(tabledata);
                    }
                }
            }
        }
        // if we fall through to here, we failed getting data somewhere along the way
        return None;
    }
}

impl eframe::App for MyApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui
        let is_web = cfg!(target_arch = "wasm32");

        // on startup the current_part IndexMap is empty even if a part is set, so we populate it
        if !self.is_init && self.current_part.is_empty() && !self.part.is_empty() {
            if let Some(tabledata) = Self::get_part(self.part.as_str()) {
                self.current_part = tabledata;
                self.search_good = true;
            } else {
                self.search_good = false;
            }
            self.is_init = true
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::menu::bar(ui, |ui| {
                // NOTE: no File->Quit on web pages!
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            if is_web {
                ui.heading("EasyEDA to KiCAD Library Converter");
            }
            let mut imagevec = vec![];

            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label("LCSC number or part URL: ");
                    ui.add(TextEdit::singleline(&mut self.part).desired_width(800.0));
                    if ui.button("Search").clicked() {
                        if let Some(tabledata) = Self::get_part(self.part.as_str()) {
                            self.current_part = tabledata;
                            self.search_good = true;
                        } else {
                            self.search_good = false;
                        }
                    }
                });
                ui.horizontal(|ui| {
                    if self.search_good {
                        ui.label(format!(
                            "Current Part: {}",
                            self.current_part
                                .get("Component Code")
                                .unwrap_or(&"".to_owned())
                        ));
                        if ui.button("Add to Library").clicked() {
                            Exec::cmd(&self.exe_path).popen();
                        }
                    } else {
                        ui.label("No such part found. Check part number or URL!");
                    }
                });
            });

            ui.separator();

            ui.vertical(|ui| {
                TableBuilder::new(ui)
                    .striped(true)
                    .resizable(true)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .column(Column::initial(170.0).at_least(90.0))
                    .column(Column::initial(400.0).at_least(170.0))
                    .header(20.0, |mut header| {
                        header.col(|ui| {
                            ui.heading("Parameter");
                        });
                        header.col(|ui| {
                            ui.heading("Value");
                        });
                    })
                    .body(|mut body| {
                        for (key, value) in &self.current_part {
                            if !key.starts_with("meta_") {
                                body.row(15.0, |mut row| {
                                    row.col(|ui| {
                                        ui.label(key);
                                    });
                                    row.col(|ui| {
                                        ui.label(value);
                                        if key == "Component Code" {
                                            ui.hyperlink_to(
                                                "LCSC",
                                                format!(
                                                    "https://www.lcsc.com/product-detail/{}.html",
                                                    value
                                                ),
                                            );
                                            ui.hyperlink_to(
                                                "JLCPCB",
                                                format!("https://jlcpcb.com/partdetail/{}", value),
                                            );
                                        }
                                    });
                                });
                            } else if key.starts_with("meta_datasheeturl") {
                                body.row(15.0, |mut row| {
                                    row.col(|ui| {
                                        ui.label("Datasheet");
                                    });
                                    row.col(|ui| {
                                        ui.hyperlink(value);
                                    });
                                });
                            } else if key.starts_with("meta_image") {
                                imagevec.push(value);
                            }
                        }
                    });

                ui.separator();

                ui.horizontal(|ui| {
                    for url in imagevec {
                        let img = ui
                            .add(egui::Image::new(url).fit_to_exact_size(Vec2::new(200.0, 200.0)));
                        if img.hovered() {
                            Window::new("")
                                .auto_sized()
                                .interactable(false)
                                .show(ctx, |ui| {
                                    ui.add(
                                        egui::Image::new(url)
                                            .fit_to_exact_size(Vec2::new(900.0, 900.0)),
                                    );
                                });
                        }
                    }
                });
            });

            ui.separator();

            ui.vertical(|ui| {
                ui.heading("Settings");
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.download_datasheet, "Download Datasheet");
                });
                ui.horizontal(|ui| {
                    ui.label("Path of JLC2KiCadLib executable:");
                    ui.add(TextEdit::singleline(&mut self.exe_path).desired_width(800.0));
                });
            });

            ui.separator();

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                powered_by(ui);
                egui::warn_if_debug_build(ui);
            });
        });
    }
}

fn powered_by(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(", ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(" and ");
        ui.hyperlink_to(
            "JLC2KiCad_lib",
            "https://github.com/TousstNicolas/JLC2KiCad_lib",
        );
        ui.label(".");
    });
}
