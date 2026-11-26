//! Repub Desktop Application
//!
//! Launch the Repub GUI application.

use repub_ui::RepubApp;

fn main() -> eframe::Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn,wgpu_core=off,wgpu_hal=off,naga=off")).init();
    
    log::info!("Starting Repub Desktop");

    // Configure native options
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([1280.0, 800.0])
        .with_min_inner_size([800.0, 600.0])
        .with_title("Repub")
        .with_icon(load_icon());

    // macOS: draw our own Publisher-style title row (Quick Access Toolbar + centered
    // title) under a transparent title bar; the traffic lights stay native.
    #[cfg(target_os = "macos")]
    let viewport = viewport
        .with_fullsize_content_view(true)
        .with_titlebar_shown(false)
        .with_title_shown(false);

    let native_options = eframe::NativeOptions {
        viewport,
        // wgpu backend: canvas text is drawn by a WGSL SDF shader (see sdf_text)
        renderer: eframe::Renderer::Wgpu,
        // 4x MSAA for smooth edges on WordArt shapes and strokes
        // WebGPU spec guarantees [1, 4] sample counts for Bgra8Unorm
        multisampling: 4,
        ..Default::default()
    };

    eframe::run_native(
        "Repub",
        native_options,
        Box::new(|cc| {
            // Create native menu bar
            #[cfg(not(target_arch = "wasm32"))]
            let (menu, menu_rx) = repub_ui::native_menu::create_menu();
            
            #[cfg(target_os = "macos")]
            repub_ui::native_menu::init_menu_for_nsapp(&menu);
            repub_ui::native_menu::force_light_appearance();
            
            // Create the app with native menu support
            #[cfg(not(target_arch = "wasm32"))]
            let mut app = RepubApp::with_native_menu(cc, menu, menu_rx);
            
            #[cfg(target_arch = "wasm32")]
            let mut app = RepubApp::new(cc);
            
            // Open a document passed on the command line: `repub-gui file.pub` or `repub-gui file.repub`
            if let Some(path) = std::env::args_os().nth(1) {
                app.open_path(path.into());
            }
            
            Ok(Box::new(app))
        }),
    )
}

/// Load the application icon (teal Publisher-style badge, see `assets/make_icon.swift`).
fn load_icon() -> egui::IconData {
    let image = image::load_from_memory_with_format(
        include_bytes!("../../repub-ui/assets/icon.png"),
        image::ImageFormat::Png,
    )
    .expect("embedded icon.png is a valid PNG")
    .into_rgba8();
    let (width, height) = image.dimensions();
    egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}
