//! Page layout.

use repub_geometry::{Point, Rect};
use repub_model::document::{Guide, Margins, MasterPage, Page};
use repub_model::{PageOrientation, PageSize};

/// Layout information for a page.
#[derive(Debug, Clone)]
pub struct PageLayout {
    /// Page bounds (full page)
    pub bounds: Rect,
    /// Content area (inside margins)
    pub content_area: Rect,
    /// Margin area
    pub margins: Margins,
    /// Bleed area (extends beyond page)
    pub bleed_area: Option<Rect>,
    /// Column guides (x positions)
    pub column_guides: Vec<f64>,
    /// Row guides (y positions)
    pub row_guides: Vec<f64>,
    /// All horizontal guides
    pub horizontal_guides: Vec<f64>,
    /// All vertical guides
    pub vertical_guides: Vec<f64>,
}

impl PageLayout {
    /// Create a page layout from a page.
    pub fn from_page(page: &Page, size: PageSize, orientation: PageOrientation) -> Self {
        let page_size = size.to_points();
        let (width, height) = match orientation {
            PageOrientation::Portrait => (page_size.width, page_size.height),
            PageOrientation::Landscape => (page_size.height, page_size.width),
        };

        let bounds = Rect::new(0.0, 0.0, width, height);
        let margins = page.margins;
        let content_area = Rect::new(
            margins.left,
            margins.top,
            width - margins.left - margins.right,
            height - margins.top - margins.bottom,
        );

        // Extract guides
        let (horizontal_guides, vertical_guides) = Self::extract_guides(&page.guides);

        Self {
            bounds,
            content_area,
            margins,
            bleed_area: None,
            column_guides: Vec::new(),
            row_guides: Vec::new(),
            horizontal_guides,
            vertical_guides,
        }
    }

    /// Set bleed area.
    pub fn with_bleed(mut self, bleed: f64) -> Self {
        self.bleed_area = Some(Rect::new(
            -bleed,
            -bleed,
            self.bounds.width() + bleed * 2.0,
            self.bounds.height() + bleed * 2.0,
        ));
        self
    }

    /// Add column guides.
    pub fn with_columns(mut self, count: usize, gap: f64) -> Self {
        if count <= 1 {
            return self;
        }

        let content_width = self.content_area.width();
        let total_gap = (count - 1) as f64 * gap;
        let col_width = (content_width - total_gap) / count as f64;

        self.column_guides.clear();
        for i in 1..count {
            let x = self.content_area.origin.x + (col_width + gap) * i as f64 - gap / 2.0;
            self.column_guides.push(x);
        }

        self
    }

    /// Extract guides from guide list.
    fn extract_guides(guides: &[Guide]) -> (Vec<f64>, Vec<f64>) {
        let mut horizontal = Vec::new();
        let mut vertical = Vec::new();

        for guide in guides {
            if guide.horizontal {
                horizontal.push(guide.position);
            } else {
                vertical.push(guide.position);
            }
        }

        horizontal.sort_by(|a, b| a.partial_cmp(b).unwrap());
        vertical.sort_by(|a, b| a.partial_cmp(b).unwrap());

        (horizontal, vertical)
    }

    /// Get all snap points (guides, margins, etc.).
    pub fn snap_points(&self) -> (Vec<f64>, Vec<f64>) {
        let mut h_points = self.horizontal_guides.clone();
        let mut v_points = self.vertical_guides.clone();

        // Add margin guides
        h_points.push(self.margins.top);
        h_points.push(self.bounds.height() - self.margins.bottom);
        v_points.push(self.margins.left);
        v_points.push(self.bounds.width() - self.margins.right);

        // Add column guides
        v_points.extend(&self.column_guides);

        // Sort and deduplicate
        h_points.sort_by(|a, b| a.partial_cmp(b).unwrap());
        h_points.dedup();
        v_points.sort_by(|a, b| a.partial_cmp(b).unwrap());
        v_points.dedup();

        (h_points, v_points)
    }

    /// Check if a point is within the content area.
    pub fn is_in_content_area(&self, point: Point) -> bool {
        self.content_area.contains(point)
    }

    /// Check if a point is within the bleed area.
    pub fn is_in_bleed_area(&self, point: Point) -> bool {
        if let Some(bleed) = &self.bleed_area {
            bleed.contains(point)
        } else {
            self.bounds.contains(point)
        }
    }
}

/// Mirror margins for facing pages (booklet layout).
pub fn mirror_margins(margins: &Margins, is_left_page: bool) -> Margins {
    if is_left_page {
        Margins {
            top: margins.top,
            bottom: margins.bottom,
            left: margins.right, // Swap for left page
            right: margins.left,
        }
    } else {
        *margins
    }
}

/// Apply master page content to a page.
///
/// Master layers are matched to page layers by name (creating the layer if it
/// doesn't exist), and each master shape is cloned onto the matching page
/// layer. Master guides are added if not already present.
pub fn apply_master_page(page: &mut Page, master: &MasterPage) {
    for layer in &master.layers {
        // Find or create corresponding layer on page
        let page_layer_id = page
            .layers()
            .iter()
            .find(|l| l.name == layer.name)
            .map(|l| l.id)
            .unwrap_or_else(|| page.add_layer(&layer.name));

        // Clone shapes from master to page
        for shape in layer.shapes() {
            let cloned = shape.clone();
            page.add_shape_to_layer(page_layer_id, cloned);
        }
    }

    // Apply master guides
    for guide in &master.guides {
        if !page.guides.contains(guide) {
            page.guides.push(*guide);
        }
    }
}

/// Calculate the position for objects relative to page elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelativeTo {
    /// Relative to page origin
    Page,
    /// Relative to content area (margins)
    Margins,
    /// Relative to bleed area
    Bleed,
}

/// Position an object relative to a page element.
pub fn position_relative(
    page_layout: &PageLayout,
    relative_to: RelativeTo,
    offset: Point,
) -> Point {
    match relative_to {
        RelativeTo::Page => offset,
        RelativeTo::Margins => Point::new(
            page_layout.content_area.origin.x + offset.x,
            page_layout.content_area.origin.y + offset.y,
        ),
        RelativeTo::Bleed => {
            if let Some(bleed) = &page_layout.bleed_area {
                Point::new(bleed.origin.x + offset.x, bleed.origin.y + offset.y)
            } else {
                offset
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_layout_creation() {
        let mut page = Page::new(PageSize::Letter, PageOrientation::Portrait);
        page.guides.push(Guide::horizontal(100.0));
        page.guides.push(Guide::vertical(200.0));

        let layout = PageLayout::from_page(&page, PageSize::Letter, PageOrientation::Portrait);

        // Letter is 612 x 792 points
        assert!((layout.bounds.width() - 612.0).abs() < 0.1);
        assert!((layout.bounds.height() - 792.0).abs() < 0.1);

        assert_eq!(layout.horizontal_guides.len(), 1);
        assert_eq!(layout.vertical_guides.len(), 1);
    }

    #[test]
    fn test_mirror_margins() {
        let margins = Margins {
            top: 72.0,
            bottom: 72.0,
            left: 108.0, // 1.5 inch (binding side)
            right: 72.0,
        };

        let mirrored = mirror_margins(&margins, true);

        assert_eq!(mirrored.left, 72.0);
        assert_eq!(mirrored.right, 108.0);
    }

    #[test]
    fn test_with_columns() {
        let page = Page::new(PageSize::Letter, PageOrientation::Portrait);

        let layout = PageLayout::from_page(&page, PageSize::Letter, PageOrientation::Portrait)
            .with_columns(3, 12.0);

        assert_eq!(layout.column_guides.len(), 2);
    }

    #[test]
    fn test_with_bleed() {
        let page = Page::new(PageSize::Letter, PageOrientation::Portrait);

        let layout = PageLayout::from_page(&page, PageSize::Letter, PageOrientation::Portrait)
            .with_bleed(9.0); // 1/8 inch bleed

        assert!(layout.bleed_area.is_some());
        let bleed = layout.bleed_area.unwrap();
        assert!((bleed.origin.x - (-9.0)).abs() < 0.01);
        assert!((bleed.width() - 630.0).abs() < 0.1); // 612 + 18
    }
}
