//! Export tools.

use std::path::{Path, PathBuf};

use repub_export_raster::{export_image, export_image_bytes, ExportFormat, RasterExportOptions};
use repub_export_vector::{export_pdf, export_svg, PdfOptions, SvgOptions};
use repub_fonts::SystemFontProvider;
use serde::Deserialize;
use serde_json::{json, Value};

use super::Tool;
use crate::error::{McpError, McpResult};
use crate::protocol::{ToolDefinition, ToolResult};
use crate::session::Session;

/// Export document to file.
pub struct ExportDocument;

#[derive(Debug, Deserialize)]
struct ExportDocumentArgs {
    path: String,
    format: String,
    pages: Option<String>,
    dpi: Option<f64>,
}

impl Tool for ExportDocument {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "repub_export".to_string(),
            description: "Export the document to a file".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Output file path"
                    },
                    "format": {
                        "type": "string",
                        "enum": ["pdf", "svg", "png", "jpg"],
                        "description": "Export format"
                    },
                    "pages": {
                        "type": "string",
                        "description": "Page range (e.g., '1-3', '1,3,5', 'all')",
                        "default": "all"
                    },
                    "dpi": {
                        "type": "number",
                        "description": "DPI for raster formats",
                        "default": 150
                    }
                },
                "required": ["path", "format"]
            }),
        }
    }

    fn execute(&self, session: &mut Session, args: Value) -> McpResult<ToolResult> {
        let args: ExportDocumentArgs =
            serde_json::from_value(args).map_err(|e| McpError::InvalidArgument(e.to_string()))?;

        let doc = session.document()?;
        let path = PathBuf::from(&args.path);

        // The scene-driven exporters resolve and measure fonts through a provider.
        let fonts = SystemFontProvider::new()
            .map_err(|e| McpError::Export(format!("Failed to load system fonts: {}", e)))?;

        match args.format.to_lowercase().as_str() {
            "pdf" => {
                let opts = PdfOptions::default();
                export_pdf(doc, &path, &opts, &fonts)
                    .map_err(|e| McpError::Export(e.to_string()))?;

                Ok(ToolResult::json(json!({
                    "success": true,
                    "message": format!("Exported PDF to {}", path.display()),
                    "path": args.path,
                    "format": "pdf"
                })))
            }

            "svg" => {
                let opts = SvgOptions::default();
                let pages = args.pages.as_deref().unwrap_or("all");

                if pages == "all" && doc.pages().len() > 1 {
                    // Export all pages with numbered filenames
                    for i in 0..doc.pages().len() {
                        let page_path = add_page_suffix(&path, i + 1);
                        export_svg(doc, i, &page_path, &opts, &fonts)
                            .map_err(|e| McpError::Export(e.to_string()))?;
                    }
                    Ok(ToolResult::json(json!({
                        "success": true,
                        "message": format!("Exported {} SVG files", doc.pages().len()),
                        "format": "svg",
                        "pages": doc.pages().len()
                    })))
                } else {
                    // Export single page or first page
                    export_svg(doc, 0, &path, &opts, &fonts)
                        .map_err(|e| McpError::Export(e.to_string()))?;
                    Ok(ToolResult::json(json!({
                        "success": true,
                        "message": format!("Exported SVG to {}", path.display()),
                        "path": args.path,
                        "format": "svg"
                    })))
                }
            }

            "png" | "jpg" | "jpeg" => {
                let dpi = args.dpi.unwrap_or(150.0);
                let format = if args.format.to_lowercase() == "png" {
                    ExportFormat::Png
                } else {
                    ExportFormat::Jpeg
                };
                let opts = RasterExportOptions {
                    format,
                    dpi,
                    ..RasterExportOptions::default()
                };
                export_image(doc, 0, &path, &opts, &fonts)
                    .map_err(|e| McpError::Export(e.to_string()))?;

                Ok(ToolResult::json(json!({
                    "success": true,
                    "message": format!("Exported image to {}", path.display()),
                    "path": args.path,
                    "format": args.format
                })))
            }

            _ => Err(McpError::InvalidArgument(format!(
                "Unknown format: {}",
                args.format
            ))),
        }
    }
}

/// Render a page preview.
pub struct RenderPreview;

#[derive(Debug, Deserialize)]
struct RenderPreviewArgs {
    page: Option<usize>,
    width: Option<u32>,
}

impl Tool for RenderPreview {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "repub_render_preview".to_string(),
            description: "Render a page preview as base64 PNG".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "page": {
                        "type": "integer",
                        "description": "Page index (0-based, default: 0)"
                    },
                    "width": {
                        "type": "integer",
                        "description": "Preview width in pixels",
                        "default": 400
                    }
                }
            }),
        }
    }

    fn execute(&self, session: &mut Session, args: Value) -> McpResult<ToolResult> {
        let args: RenderPreviewArgs =
            serde_json::from_value(args).map_err(|e| McpError::InvalidArgument(e.to_string()))?;

        let doc = session.document()?;
        let page_idx = args.page.unwrap_or(0);
        let target_width = args.width.unwrap_or(400);

        if page_idx >= doc.pages().len() {
            return Err(McpError::InvalidPage(page_idx, doc.pages().len()));
        }

        let page = &doc.pages()[page_idx];

        // Calculate DPI to achieve target width
        // At 72 DPI, width = page.size.width pixels
        // To get target_width pixels, DPI = 72 * target_width / page.size.width
        let dpi = 72.0 * target_width as f64 / page.size.width;
        let target_height = (page.size.height * dpi / 72.0) as u32;

        // Compose and rasterize the page to encoded PNG bytes.
        let fonts = SystemFontProvider::new()
            .map_err(|e| McpError::Export(format!("Failed to load system fonts: {}", e)))?;
        let opts = RasterExportOptions::png(dpi);
        let png_data = export_image_bytes(doc, page_idx, &opts, &fonts)
            .map_err(|e| McpError::Export(e.to_string()))?;

        // Return as base64
        let base64_data =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &png_data);

        Ok(ToolResult::json(json!({
            "success": true,
            "page": page_idx,
            "width": target_width,
            "height": target_height,
            "image": {
                "type": "base64",
                "mime_type": "image/png",
                "data": base64_data
            }
        })))
    }
}

/// Add a page number suffix to a file path.
fn add_page_suffix(path: &Path, page_num: usize) -> PathBuf {
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let ext = path.extension().unwrap_or_default().to_string_lossy();
    let parent = path.parent().unwrap_or(std::path::Path::new("."));
    parent.join(format!("{}-page{}.{}", stem, page_num, ext))
}

#[cfg(test)]
mod tests {
    use super::*;
    use repub_model::{Document, PageOrientation, PageSize};

    fn setup_session() -> Session {
        let mut session = Session::new();
        let doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        session.create_document(doc);
        session
    }

    #[test]
    fn test_render_preview() {
        let tool = RenderPreview;
        let mut session = setup_session();

        let result = tool
            .execute(
                &mut session,
                json!({
                    "page": 0,
                    "width": 200
                }),
            )
            .unwrap();

        assert!(!result.is_error);
    }
}
