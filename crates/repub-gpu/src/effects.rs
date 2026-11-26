//! Realization status of [`SceneEffect`](repub_scene::SceneEffect)s.
//!
//! Effects are expensive to port faithfully (true Gaussian blur needs a
//! multi-pass ping-pong pipeline). This pass deliberately keeps the base
//! primitives correct and realizes only a cheap approximation of drop shadows,
//! deferring everything else. Deferred effects are **dropped** (the primitive
//! is drawn without them), never faked.
//!
//! Realized:
//! - [`SceneEffect::DropShadow`] — approximated as a flat, **unblurred** copy
//!   of the primitive's own geometry, offset by `(offset_x, offset_y)` and
//!   tinted with the shadow color, drawn behind the primitive. `blur_radius`
//!   and `spread` are ignored; a true blurred shadow is deferred.
//!
//! Deferred (drawn as if absent): [`SceneEffect::InnerShadow`],
//! [`SceneEffect::OuterGlow`], [`SceneEffect::InnerGlow`],
//! [`SceneEffect::Reflection`], [`SceneEffect::SoftEdges`] and
//! [`SceneEffect::Bevel`].

use repub_color::Color;
use repub_scene::SceneEffect;

/// A realized drop-shadow approximation: a solid color and a page-space offset.
pub struct ShadowApprox {
    /// Shadow tint color.
    pub color: Color,
    /// Horizontal offset in points.
    pub offset_x: f64,
    /// Vertical offset in points.
    pub offset_y: f64,
}

/// Extract the drop-shadow approximation for a primitive, if any.
///
/// Returns the first [`SceneEffect::DropShadow`] in `effects`. All other
/// effects are deferred and ignored here.
pub fn drop_shadow(effects: &[SceneEffect]) -> Option<ShadowApprox> {
    effects.iter().find_map(|e| match e {
        SceneEffect::DropShadow {
            color,
            offset_x,
            offset_y,
            ..
        } => Some(ShadowApprox {
            color: *color,
            offset_x: *offset_x,
            offset_y: *offset_y,
        }),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_drop_shadow() {
        let effects = vec![
            SceneEffect::SoftEdges { radius: 2.0 },
            SceneEffect::DropShadow {
                color: Color::BLACK,
                offset_x: 3.0,
                offset_y: 4.0,
                blur_radius: 5.0,
                spread: 0.0,
            },
        ];
        let s = drop_shadow(&effects).unwrap();
        assert_eq!((s.offset_x, s.offset_y), (3.0, 4.0));
    }

    #[test]
    fn no_shadow_when_absent() {
        assert!(drop_shadow(&[SceneEffect::SoftEdges { radius: 1.0 }]).is_none());
    }
}
