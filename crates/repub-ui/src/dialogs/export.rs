//! Export dialog.

/// Export dialog state.
#[derive(Debug)]
pub struct ExportDialog {
    /// Is the dialog open?
    pub open: bool,
    /// Selected format
    pub format: ExportFormat,
    /// DPI for raster formats
    pub dpi: u32,
    /// Page range
    pub page_range: PageRange,
    /// Custom page range string
    pub custom_range: String,
    /// JPEG quality, 1-100
    pub quality: u8,
}

/// Export formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// PDF document
    Pdf,
    /// SVG vector
    Svg,
    /// PNG image
    Png,
    /// JPEG image
    Jpeg,
    /// TIFF image
    Tiff,
    /// HTML page
    Html,
}

impl ExportFormat {
    pub const ALL: [ExportFormat; 6] = [
        ExportFormat::Pdf,
        ExportFormat::Svg,
        ExportFormat::Png,
        ExportFormat::Jpeg,
        ExportFormat::Tiff,
        ExportFormat::Html,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ExportFormat::Pdf => "PDF Document",
            ExportFormat::Svg => "SVG Vector",
            ExportFormat::Png => "PNG Image",
            ExportFormat::Jpeg => "JPEG Image",
            ExportFormat::Tiff => "TIFF Image",
            ExportFormat::Html => "HTML Page",
        }
    }

    /// File extension, without the dot.
    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Pdf => "pdf",
            ExportFormat::Svg => "svg",
            ExportFormat::Png => "png",
            ExportFormat::Jpeg => "jpg",
            ExportFormat::Tiff => "tiff",
            ExportFormat::Html => "html",
        }
    }

    /// Whether the format is a picture of each page (it has a resolution).
    pub fn is_raster(self) -> bool {
        matches!(
            self,
            ExportFormat::Png | ExportFormat::Jpeg | ExportFormat::Tiff
        )
    }

    /// Whether every chosen page goes into one file (the rest write a file per page).
    pub fn single_file(self) -> bool {
        matches!(self, ExportFormat::Pdf | ExportFormat::Html)
    }
}

/// Page range options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageRange {
    /// All pages
    All,
    /// Current page only
    Current,
    /// Custom range
    Custom,
}

impl Default for ExportDialog {
    fn default() -> Self {
        Self {
            open: false,
            format: ExportFormat::Pdf,
            dpi: 150,
            page_range: PageRange::All,
            custom_range: String::new(),
            quality: 90,
        }
    }
}

impl ExportDialog {
    /// Create a new dialog.
    pub fn new() -> Self {
        Self::default()
    }

    /// Open the dialog.
    pub fn open(&mut self) {
        self.open = true;
    }

    /// Show the dialog. Returns export options if confirmed.
    /// `current_page` is the 0-based page shown on the canvas (for "Current").
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        page_count: usize,
        current_page: usize,
    ) -> Option<ExportOptions> {
        let mut result = None;

        if !self.open {
            return None;
        }

        egui::Window::new("Export")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(300.0);

                // Format selection
                ui.horizontal(|ui| {
                    ui.label("Format:");
                    egui::ComboBox::from_id_salt("export_format")
                        .selected_text(self.format.label())
                        .show_ui(ui, |ui| {
                            for format in ExportFormat::ALL {
                                ui.selectable_value(&mut self.format, format, format.label());
                            }
                        });
                });

                // DPI (for raster formats)
                if self.format.is_raster() {
                    ui.horizontal(|ui| {
                        ui.label("DPI:");
                        egui::ComboBox::from_id_salt("export_dpi")
                            .selected_text(format!("{} DPI", self.dpi))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.dpi, 72, "72 DPI (Screen)");
                                ui.selectable_value(&mut self.dpi, 150, "150 DPI (Draft)");
                                ui.selectable_value(&mut self.dpi, 300, "300 DPI (Print)");
                                ui.selectable_value(&mut self.dpi, 600, "600 DPI (High Quality)");
                            });
                    });
                }

                if self.format == ExportFormat::Jpeg {
                    ui.horizontal(|ui| {
                        ui.label("Quality:");
                        let mut quality = u32::from(self.quality);
                        if ui.add(egui::Slider::new(&mut quality, 1..=100)).changed() {
                            self.quality = quality as u8;
                        }
                    });
                }

                // Page range
                ui.horizontal(|ui| {
                    ui.label("Pages:");
                    ui.selectable_value(&mut self.page_range, PageRange::All, "All");
                    ui.selectable_value(&mut self.page_range, PageRange::Current, "Current");
                    ui.selectable_value(&mut self.page_range, PageRange::Custom, "Custom");
                });

                if self.page_range == PageRange::Custom {
                    ui.horizontal(|ui| {
                        ui.label("Range:");
                        ui.text_edit_singleline(&mut self.custom_range);
                        ui.label("(e.g., 1-3, 5)");
                    });
                }

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);

                // Buttons
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.open = false;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Export...").clicked() {
                            result = Some(ExportOptions {
                                format: self.format,
                                dpi: self.dpi,
                                pages: self.resolve_pages(page_count, current_page),
                                quality: self.quality,
                            });
                            self.open = false;
                        }
                    });
                });
            });

        result
    }

    fn resolve_pages(&self, page_count: usize, current_page: usize) -> Vec<usize> {
        match self.page_range {
            PageRange::All => (0..page_count).collect(),
            PageRange::Current => vec![current_page.min(page_count.saturating_sub(1))],
            PageRange::Custom => parse_page_range(&self.custom_range, page_count),
        }
    }
}

fn parse_page_range(range: &str, max: usize) -> Vec<usize> {
    let mut pages = Vec::new();

    for part in range.split(',') {
        let part = part.trim();
        if part.contains('-') {
            let parts: Vec<&str> = part.split('-').collect();
            if parts.len() == 2 {
                if let (Ok(start), Ok(end)) = (
                    parts[0].trim().parse::<usize>(),
                    parts[1].trim().parse::<usize>(),
                ) {
                    for i in start.saturating_sub(1)..end.min(max) {
                        if !pages.contains(&i) {
                            pages.push(i);
                        }
                    }
                }
            }
        } else if let Ok(page) = part.parse::<usize>() {
            let idx = page.saturating_sub(1);
            if idx < max && !pages.contains(&idx) {
                pages.push(idx);
            }
        }
    }

    pages.sort();
    pages
}

/// Export options.
pub struct ExportOptions {
    /// Format
    pub format: ExportFormat,
    /// DPI
    pub dpi: u32,
    /// Pages to export (0-based indices)
    pub pages: Vec<usize>,
    /// JPEG quality, 1-100
    pub quality: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_means_the_page_on_screen() {
        let dialog = ExportDialog {
            page_range: PageRange::Current,
            ..Default::default()
        };
        assert_eq!(dialog.resolve_pages(5, 3), vec![3]);
    }

    #[test]
    fn test_parse_page_range() {
        assert_eq!(parse_page_range("1-3", 10), vec![0, 1, 2]);
        assert_eq!(parse_page_range("1, 3, 5", 10), vec![0, 2, 4]);
        assert_eq!(parse_page_range("1-3, 5", 10), vec![0, 1, 2, 4]);
    }
}
