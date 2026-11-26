//! Snapping system for precise object placement.

use repub_geometry::{Point, Rect};

/// Snap distance in pixels.
pub const SNAP_DISTANCE: f64 = 8.0;

/// Snap targets for alignment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SnapTarget {
    /// Page edge
    PageEdge,
    /// Page margin
    PageMargin,
    /// Guide line
    Guide,
    /// Grid line
    Grid,
    /// Another object's edge
    ObjectEdge,
    /// Another object's center
    ObjectCenter,
}

/// Snap axis (horizontal or vertical).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SnapAxis {
    /// Horizontal snap (affects X position, draws vertical line)
    Horizontal,
    /// Vertical snap (affects Y position, draws horizontal line)
    Vertical,
}

/// Snap result for a single axis.
#[derive(Debug, Clone)]
pub struct SnapResult {
    /// Snapped position
    pub position: f64,
    /// What it snapped to
    pub target: SnapTarget,
    /// Guide line position for visualization
    pub guide_pos: f64,
    /// Which axis this snap applies to
    pub axis: SnapAxis,
}

/// Snap configuration.
#[derive(Debug, Clone)]
pub struct SnapConfig {
    /// Snap to page edges
    pub page_edges: bool,
    /// Snap to page margins
    pub margins: bool,
    /// Snap to guides
    pub guides: bool,
    /// Snap to grid
    pub grid: bool,
    /// Snap to other objects
    pub objects: bool,
    /// Grid spacing
    pub grid_spacing: f64,
    /// Margin size
    pub margin_size: f64,
}

impl Default for SnapConfig {
    fn default() -> Self {
        Self {
            page_edges: true,
            margins: true,
            guides: true,
            grid: false,
            objects: true,
            grid_spacing: 10.0,
            margin_size: 36.0, // 0.5 inch
        }
    }
}

/// Snapping engine.
pub struct SnapEngine {
    config: SnapConfig,
    /// Page bounds
    page_bounds: Rect,
    /// Guide positions (horizontal)
    h_guides: Vec<f64>,
    /// Guide positions (vertical)
    v_guides: Vec<f64>,
    /// Other object bounds (excluding the one being moved)
    object_bounds: Vec<Rect>,
}

impl SnapEngine {
    /// Create a new snap engine.
    pub fn new(config: SnapConfig, page_bounds: Rect) -> Self {
        Self {
            config,
            page_bounds,
            h_guides: Vec::new(),
            v_guides: Vec::new(),
            object_bounds: Vec::new(),
        }
    }

    /// Set horizontal guides.
    pub fn set_h_guides(&mut self, guides: Vec<f64>) {
        self.h_guides = guides;
    }

    /// Set vertical guides.
    pub fn set_v_guides(&mut self, guides: Vec<f64>) {
        self.v_guides = guides;
    }

    /// Set other object bounds for snapping.
    pub fn set_object_bounds(&mut self, bounds: Vec<Rect>) {
        self.object_bounds = bounds;
    }

    /// Snap a point.
    pub fn snap_point(&self, point: Point) -> (Point, Option<SnapResult>, Option<SnapResult>) {
        let (snapped_x, snap_h) = self.snap_x(point.x);
        let (snapped_y, snap_v) = self.snap_y(point.y);
        (Point::new(snapped_x, snapped_y), snap_h, snap_v)
    }

    /// Snap a rectangle being moved.
    pub fn snap_move(&self, bounds: &Rect) -> (Point, Vec<SnapResult>) {
        let mut snaps = Vec::new();
        let mut dx = 0.0;
        let mut dy = 0.0;

        // Try snapping left edge
        if let (_, Some(snap)) = self.snap_x(bounds.origin.x) {
            if (snap.position - bounds.origin.x).abs() <= SNAP_DISTANCE {
                dx = snap.position - bounds.origin.x;
                snaps.push(snap);
            }
        }

        // Try snapping right edge
        if dx == 0.0 {
            if let (_, Some(snap)) = self.snap_x(bounds.right()) {
                if (snap.position - bounds.right()).abs() <= SNAP_DISTANCE {
                    dx = snap.position - bounds.right();
                    snaps.push(snap);
                }
            }
        }

        // Try snapping center X
        if dx == 0.0 {
            if let (_, Some(snap)) = self.snap_x(bounds.center().x) {
                if (snap.position - bounds.center().x).abs() <= SNAP_DISTANCE {
                    dx = snap.position - bounds.center().x;
                    snaps.push(snap);
                }
            }
        }

        // Try snapping top edge
        if let (_, Some(snap)) = self.snap_y(bounds.origin.y) {
            if (snap.position - bounds.origin.y).abs() <= SNAP_DISTANCE {
                dy = snap.position - bounds.origin.y;
                snaps.push(snap);
            }
        }

        // Try snapping bottom edge
        if dy == 0.0 {
            if let (_, Some(snap)) = self.snap_y(bounds.bottom()) {
                if (snap.position - bounds.bottom()).abs() <= SNAP_DISTANCE {
                    dy = snap.position - bounds.bottom();
                    snaps.push(snap);
                }
            }
        }

        // Try snapping center Y
        if dy == 0.0 {
            if let (_, Some(snap)) = self.snap_y(bounds.center().y) {
                if (snap.position - bounds.center().y).abs() <= SNAP_DISTANCE {
                    dy = snap.position - bounds.center().y;
                    snaps.push(snap);
                }
            }
        }

        (Point::new(bounds.origin.x + dx, bounds.origin.y + dy), snaps)
    }

    fn snap_x(&self, x: f64) -> (f64, Option<SnapResult>) {
        let mut best_snap: Option<(f64, SnapTarget, f64)> = None;
        let mut best_dist = SNAP_DISTANCE;

        // Page edges
        if self.config.page_edges {
            let left = self.page_bounds.origin.x;
            let right = self.page_bounds.right();
            let center = self.page_bounds.center().x;

            if (x - left).abs() < best_dist {
                best_dist = (x - left).abs();
                best_snap = Some((left, SnapTarget::PageEdge, left));
            }
            if (x - right).abs() < best_dist {
                best_dist = (x - right).abs();
                best_snap = Some((right, SnapTarget::PageEdge, right));
            }
            if (x - center).abs() < best_dist {
                best_dist = (x - center).abs();
                best_snap = Some((center, SnapTarget::PageEdge, center));
            }
        }

        // Margins
        if self.config.margins {
            let left = self.page_bounds.origin.x + self.config.margin_size;
            let right = self.page_bounds.right() - self.config.margin_size;

            if (x - left).abs() < best_dist {
                best_dist = (x - left).abs();
                best_snap = Some((left, SnapTarget::PageMargin, left));
            }
            if (x - right).abs() < best_dist {
                best_dist = (x - right).abs();
                best_snap = Some((right, SnapTarget::PageMargin, right));
            }
        }

        // Guides
        if self.config.guides {
            for &guide in &self.v_guides {
                if (x - guide).abs() < best_dist {
                    best_dist = (x - guide).abs();
                    best_snap = Some((guide, SnapTarget::Guide, guide));
                }
            }
        }

        // Grid
        if self.config.grid {
            let spacing = self.config.grid_spacing;
            let snapped = (x / spacing).round() * spacing;
            if (x - snapped).abs() < best_dist {
                best_dist = (x - snapped).abs();
                best_snap = Some((snapped, SnapTarget::Grid, snapped));
            }
        }

        // Other objects
        if self.config.objects {
            for obj in &self.object_bounds {
                for edge in [obj.origin.x, obj.right(), obj.center().x] {
                    if (x - edge).abs() < best_dist {
                        best_dist = (x - edge).abs();
                        let target = if (edge - obj.center().x).abs() < 0.01 {
                            SnapTarget::ObjectCenter
                        } else {
                            SnapTarget::ObjectEdge
                        };
                        best_snap = Some((edge, target, edge));
                    }
                }
            }
        }

        match best_snap {
            Some((pos, target, guide_pos)) => (pos, Some(SnapResult { position: pos, target, guide_pos, axis: SnapAxis::Horizontal })),
            None => (x, None),
        }
    }

    fn snap_y(&self, y: f64) -> (f64, Option<SnapResult>) {
        let mut best_snap: Option<(f64, SnapTarget, f64)> = None;
        let mut best_dist = SNAP_DISTANCE;

        // Page edges
        if self.config.page_edges {
            let top = self.page_bounds.origin.y;
            let bottom = self.page_bounds.bottom();
            let center = self.page_bounds.center().y;

            if (y - top).abs() < best_dist {
                best_dist = (y - top).abs();
                best_snap = Some((top, SnapTarget::PageEdge, top));
            }
            if (y - bottom).abs() < best_dist {
                best_dist = (y - bottom).abs();
                best_snap = Some((bottom, SnapTarget::PageEdge, bottom));
            }
            if (y - center).abs() < best_dist {
                best_dist = (y - center).abs();
                best_snap = Some((center, SnapTarget::PageEdge, center));
            }
        }

        // Margins
        if self.config.margins {
            let top = self.page_bounds.origin.y + self.config.margin_size;
            let bottom = self.page_bounds.bottom() - self.config.margin_size;

            if (y - top).abs() < best_dist {
                best_dist = (y - top).abs();
                best_snap = Some((top, SnapTarget::PageMargin, top));
            }
            if (y - bottom).abs() < best_dist {
                best_dist = (y - bottom).abs();
                best_snap = Some((bottom, SnapTarget::PageMargin, bottom));
            }
        }

        // Guides
        if self.config.guides {
            for &guide in &self.h_guides {
                if (y - guide).abs() < best_dist {
                    best_dist = (y - guide).abs();
                    best_snap = Some((guide, SnapTarget::Guide, guide));
                }
            }
        }

        // Grid
        if self.config.grid {
            let spacing = self.config.grid_spacing;
            let snapped = (y / spacing).round() * spacing;
            if (y - snapped).abs() < best_dist {
                best_dist = (y - snapped).abs();
                best_snap = Some((snapped, SnapTarget::Grid, snapped));
            }
        }

        // Other objects
        if self.config.objects {
            for obj in &self.object_bounds {
                for edge in [obj.origin.y, obj.bottom(), obj.center().y] {
                    if (y - edge).abs() < best_dist {
                        best_dist = (y - edge).abs();
                        let target = if (edge - obj.center().y).abs() < 0.01 {
                            SnapTarget::ObjectCenter
                        } else {
                            SnapTarget::ObjectEdge
                        };
                        best_snap = Some((edge, target, edge));
                    }
                }
            }
        }

        match best_snap {
            Some((pos, target, guide_pos)) => (pos, Some(SnapResult { position: pos, target, guide_pos, axis: SnapAxis::Vertical })),
            None => (y, None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snap_to_page_edge() {
        let page = Rect::new(0.0, 0.0, 612.0, 792.0);
        let engine = SnapEngine::new(SnapConfig::default(), page);
        
        // Near left edge
        let (snapped, h, _v) = engine.snap_point(Point::new(3.0, 100.0));
        assert_eq!(snapped.x, 0.0);
        assert!(h.is_some());
        assert_eq!(h.unwrap().target, SnapTarget::PageEdge);
    }

    #[test]
    fn test_snap_to_object() {
        let page = Rect::new(0.0, 0.0, 612.0, 792.0);
        let mut engine = SnapEngine::new(SnapConfig::default(), page);
        engine.set_object_bounds(vec![Rect::new(100.0, 100.0, 50.0, 50.0)]);
        
        // Near object right edge (150)
        let (snapped, h, _v) = engine.snap_point(Point::new(152.0, 200.0));
        assert_eq!(snapped.x, 150.0);
        assert!(h.is_some());
        assert_eq!(h.unwrap().target, SnapTarget::ObjectEdge);
    }

    #[test]
    fn test_snap_move() {
        let page = Rect::new(0.0, 0.0, 612.0, 792.0);
        let engine = SnapEngine::new(SnapConfig::default(), page);
        
        // Moving a rect near the page edge
        let bounds = Rect::new(3.0, 5.0, 100.0, 50.0);
        let (new_pos, snaps) = engine.snap_move(&bounds);
        
        // Should snap to left and top edges
        assert_eq!(new_pos.x, 0.0);
        assert_eq!(new_pos.y, 0.0);
        assert!(!snaps.is_empty());
    }

    #[test]
    fn test_snap_to_grid() {
        let page = Rect::new(0.0, 0.0, 612.0, 792.0);
        let mut config = SnapConfig::default();
        config.grid = true;
        config.grid_spacing = 9.0; // 1/8 inch
        config.page_edges = false; // Disable page edges to test grid alone
        config.margins = false;
        config.objects = false;
        
        let engine = SnapEngine::new(config, page);
        
        // Near grid line at 18 (2 * 9)
        let (snapped, h, _v) = engine.snap_point(Point::new(16.0, 100.0));
        assert_eq!(snapped.x, 18.0);
        assert!(h.is_some());
        assert_eq!(h.unwrap().target, SnapTarget::Grid);
        
        // Near grid line at 27 (3 * 9)
        let (snapped, _h, v) = engine.snap_point(Point::new(100.0, 25.0));
        assert_eq!(snapped.y, 27.0);
        assert!(v.is_some());
        assert_eq!(v.unwrap().target, SnapTarget::Grid);
    }
}
