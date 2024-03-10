use indexmap::indexmap;
use regex::Regex;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct MyApp {
    part: String,
    current_part: String,
    download_datasheet: bool,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            part: "C11702".to_owned(),
            current_part: "".to_owned(),
            download_datasheet: true,
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

    fn get_part(search_term: &str) -> Option<&str> {
        let term = search_term.trim();
        let re_jlc = Regex::new(r"/(C\d+)$").unwrap();
        let re_lcsc = Regex::new(r"_(C\d+)[^/]*\.html$").unwrap();
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
        } else if term.starts_with("C") {
            lcscnumber = term;
        }
        if !lcscnumber.is_empty() {
            let client = reqwest::blocking::Client::new();
            let res = client
                .get(format!("https://cart.jlcpcb.com/shoppingCart/smtGood/getComponentDetail?componentCode={}", lcscnumber))
                .header(reqwest::header::ACCEPT, "application/json")
                .send()
                .expect("Issue running the GET request to JLCPCB.");
            let res_status = res.status();
            if res_status.is_success() {
                let res_text = res
                    .text()
                    .expect("Issue decoding received response from JLCPCB.");
                let json: serde_json::Value =
                    serde_json::from_str(&res_text).expect("Issue parsing search result JSON.");
                println!("{}", json);
                let parameters = indexmap! {
                    "componentCode" => "Component code",
                    "firstTypeNameEn" => "Primary category",
                    "secondTypeNameEn" => "Secondary category",
                    "componentBrandEn" => "Brand",
                    "componentName" => "Full name",
                    "componentDesignator" => "Designator",
                    "componentModelEn" => "Model",
                    "componentSpecificationEn" => "Specification",
                    "describe" => "Description",
                    "matchedPartDetail" => "Details",
                    "stockCount" => "Stock",
                    "leastNumber" => "Minimal Quantity",
                    "leastNumberPrice" => "Minimum price",
                };
            }
            return Some(lcscnumber);
        } else {
            return None;
        }
        //
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

            ui.horizontal(|ui| {
                ui.label("LCSC number or part URL: ");
                ui.text_edit_singleline(&mut self.part);
                if ui.button("Search").clicked() {
                    if let Some(lcscnumber) = Self::get_part(self.part.as_str()) {
                        self.current_part = lcscnumber.to_string();
                    }
                }
                ui.label(format!("Current Part: {}", self.current_part.as_str()));
            });

            ui.separator();

            //ui.add();

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
