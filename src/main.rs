#![warn(clippy::all, rust_2018_idioms)]
#![allow(clippy::collapsible_if)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 1000.0])
            .with_min_inner_size([300.0, 220.0])
            .with_title("EasyEDA to KiCAD Library UI")
            .with_icon(load_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "EasyEDA_to_KiCAD_Lib_UI",
        native_options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::new(easyeda_to_kicad_lib_ui::MyApp::new(cc)))
        }),
    )
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "the_canvas_id", // hardcode it
                web_options,
                Box::new(|cc| Box::new(easyeda_to_kicad_lib_ui::MyApp::new(cc))),
            )
            .await
            .expect("failed to start eframe");
    });
}

// Function to load the icon (supports both Windows and others)
fn load_icon() -> egui::viewport::IconData {
    #[cfg(target_os = "windows")]
    {
        // Embed ICO for Windows
        let ico_bytes = include_bytes!("../assets/cpu.ico");
        let image = image::load_from_memory(ico_bytes)
            .expect("Failed to load icon from memory")
            .to_rgba8();
        let (width, height) = image.dimensions();
        egui::viewport::IconData {
            rgba: image.into_raw(),
            width,
            height,
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        // Keep PNG for macOS/Linux
        eframe::icon_data::from_png_bytes(&include_bytes!("../assets/cpu.png")[..])
            .expect("Failed to load icon")
    }
}
