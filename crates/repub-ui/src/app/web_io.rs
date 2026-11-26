//! The browser's versions of the file operations. There is no file system: files are picked
//! asynchronously (the result arrives in a later frame, see [`RepubApp::process_web_events`])
//! and saved by downloading them.

use std::io::Cursor;

use base64::Engine;
use repub_model::shape::{ImageSource, PictureData};

use super::*;
use crate::platform::web::{self, Purpose};

impl RepubApp {
    pub(super) fn open_document(&mut self) {
        web::pick_file(
            Purpose::OpenDocument,
            "All Publications",
            &["repub", "repubz", "pub"],
        );
    }

    pub(super) fn insert_picture(&mut self) {
        if self.state.has_document() {
            web::pick_file(Purpose::InsertPicture, "Pictures", &["png", "jpg", "jpeg"]);
        }
    }

    pub(super) fn change_picture(&mut self) {
        let picture = self.state.selection.iter().copied().find(|id| {
            self.state
                .current_page()
                .and_then(|p| p.get_shape(*id))
                .is_some_and(|o| matches!(o.kind, ShapeKind::Picture(_)))
        });
        if let Some(id) = picture {
            web::pick_file(
                Purpose::ChangePicture(id),
                "Pictures",
                &["png", "jpg", "jpeg"],
            );
        }
    }

    /// Save by downloading the publication as a `.repub` file.
    pub(super) fn save_document(&mut self, _save_as: bool) {
        let name = format!("{}.repub", self.document_stem());
        if let Some(doc) = self.state.document.as_mut() {
            doc.metadata.modified = unix_now();
        }
        let Some(doc) = self.state.document.as_ref() else {
            return;
        };
        let result = repub_fonts::SystemFontProvider::new()
            .map_err(|e| e.to_string())
            .and_then(|fonts| {
                repub_format::save_bytes(doc, &name, &fonts).map_err(|e| e.to_string())
            })
            .and_then(|bytes| web::download(&name, crate::platform::mime_for("repub"), &bytes));
        match result {
            Ok(()) => self.state.mark_saved(PathBuf::from(&name)),
            Err(e) => self.error_alert.open(
                "Couldn't Save",
                format!("Repub couldn't save \"{name}\".\n\n{e}"),
            ),
        }
    }

    /// Print by downloading a PDF, which the viewer can print.
    pub(super) fn print(&mut self) {
        let options = crate::dialogs::export::ExportOptions {
            format: crate::dialogs::export::ExportFormat::Pdf,
            dpi: 300,
            pages: (0..self.state.page_count()).collect(),
            quality: 90,
        };
        self.export_with_options(options);
    }

    /// Export by downloading the file (or one file per page).
    pub(super) fn export_with_options(&mut self, options: crate::dialogs::export::ExportOptions) {
        let Some(doc) = self.state.document.as_ref() else {
            return;
        };
        if options.pages.is_empty() {
            self.error_alert.open(
                "Nothing to Export",
                "The page range doesn't include any pages of this publication.",
            );
            return;
        }
        let (stem, ext) = (self.document_stem(), options.format.extension());
        let result = export_files(doc, &options).and_then(|files| {
            for (page, bytes) in files {
                let name = match page {
                    Some(page) if options.pages.len() != 1 => format!("{stem}-{}.{ext}", page + 1),
                    _ => format!("{stem}.{ext}"),
                };
                web::download(&name, crate::platform::mime_for(ext), &bytes)?;
            }
            Ok(())
        });
        if let Err(e) = result {
            log::error!("Export failed: {e}");
            self.error_alert.open(
                "Couldn't Export",
                format!("Repub couldn't export the publication.\n\n{e}"),
            );
        }
    }

    /// The name of the open publication without its extension.
    fn document_stem(&self) -> String {
        self.state
            .document_path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled".to_owned())
    }

    /// Act on files the user picked since the last frame.
    pub(super) fn process_web_events(&mut self) {
        for picked in web::take_picked() {
            match picked.purpose {
                Purpose::OpenDocument => {
                    match repub_format::load_bytes(&picked.name, &picked.bytes) {
                        Ok(doc) => {
                            self.install_opened_document(doc, Some(PathBuf::from(&picked.name)))
                        }
                        Err(e) => self.error_alert.open(
                            "Couldn't Open File",
                            format!("Repub couldn't open \"{}\".\n\n{e}", picked.name),
                        ),
                    }
                }
                Purpose::InsertPicture => {
                    let result = embedded_picture(&picked.bytes)
                        .and_then(|(data, w, h)| self.place_picture_data(data, w, h));
                    if let Err(e) = result {
                        self.error_alert.open(
                            "Couldn't Insert Picture",
                            format!("Repub couldn't read \"{}\".\n\n{e}", picked.name),
                        );
                    }
                }
                Purpose::ChangePicture(id) => match embedded_picture(&picked.bytes) {
                    Ok((data, _, _)) => {
                        self.state.record_undo("Change Picture");
                        if let Some(Shape {
                            kind: ShapeKind::Picture(img),
                            ..
                        }) = self
                            .state
                            .current_page_mut()
                            .and_then(|p| p.get_shape_mut(id))
                        {
                            img.source = data.source;
                        }
                        self.state.mark_dirty();
                    }
                    Err(e) => self.error_alert.open(
                        "Couldn't Change Picture",
                        format!("Repub couldn't read \"{}\".\n\n{e}", picked.name),
                    ),
                },
            }
        }
    }
}

/// Picture data holding the image itself (a browser can't link to a file), and its size in pixels.
fn embedded_picture(bytes: &[u8]) -> Result<(PictureData, u32, u32), String> {
    let reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mime_type = match reader.format() {
        Some(image::ImageFormat::Png) => "image/png",
        Some(image::ImageFormat::Jpeg) => "image/jpeg",
        _ => return Err("only PNG and JPEG pictures can be used".into()),
    };
    let (w, h) = reader.into_dimensions().map_err(|e| e.to_string())?;
    let mut data = PictureData::new();
    data.source = ImageSource::Embedded {
        data: base64::engine::general_purpose::STANDARD.encode(bytes),
        mime_type: mime_type.to_owned(),
    };
    Ok((data, w, h))
}
