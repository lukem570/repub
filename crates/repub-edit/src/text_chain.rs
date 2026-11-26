//! Linked text boxes: a story starts in the first box of a chain
//! (`TextFrameData::linked_frame` points to the next box) and the text that
//! doesn't fit continues in the next one. The text lives in the first box; the
//! rest of the chain is laid out from it each time it's drawn.
//!
//! Where a box's text stops fitting depends on text layout, which is a
//! rendering concern. [`layout_chains`] therefore takes an `overflow` callback
//! that, given a text frame shape, returns where its text overflows; the GUI
//! supplies its layout-based implementation.

use std::collections::HashMap;

use repub_model::id::ObjectId;
use repub_model::shape::{Shape, ShapeKind, TextFrameData, TextOverflow};
use repub_model::text::StyledText;
use repub_model::Page;

use crate::text_edit::{delete_range, end_pos, TextPos};

/// Longest chain followed (guards against link loops).
const MAX_CHAIN: usize = 64;

/// A box of a chain as drawn: the frame with the part of the story it shows,
/// and where that part starts in the story.
#[derive(Debug, Clone)]
pub struct ChainFrame {
    /// The frame shape with its shown portion of the story.
    pub frame: Shape,
    /// Where the shown portion starts in the whole story.
    pub start: TextPos,
    /// The chain's first box (the story's owner).
    pub head: ObjectId,
    /// The chain's last box (it shows the caret at the very end of the story).
    pub last: bool,
}

/// `text` from `pos` on.
pub fn text_from(text: &StyledText, pos: TextPos) -> StyledText {
    let mut t = text.clone();
    delete_range(&mut t, TextPos::new(0, 0), pos);
    t
}

/// `text` before `pos`.
pub fn text_before(text: &StyledText, pos: TextPos) -> StyledText {
    let mut t = text.clone();
    let end = end_pos(&t);
    delete_range(&mut t, pos, end);
    t
}

/// A position in a part of the story starting at `start` -> the same position
/// in the whole story.
pub fn to_story(start: TextPos, pos: TextPos) -> TextPos {
    if pos.para == 0 {
        TextPos::new(start.para, start.offset + pos.offset)
    } else {
        TextPos::new(start.para + pos.para, pos.offset)
    }
}

/// The inverse of [`to_story`]: a story position in the part starting at
/// `start`, or `None` if it comes before that part.
pub fn from_story(start: TextPos, pos: TextPos) -> Option<TextPos> {
    if pos < start {
        None
    } else if pos.para == start.para {
        Some(TextPos::new(0, pos.offset - start.offset))
    } else {
        Some(TextPos::new(pos.para - start.para, pos.offset))
    }
}

/// The text frames on `page`, by ID.
fn text_frames(page: &Page) -> HashMap<ObjectId, &Shape> {
    page.all_shapes()
        .filter(|s| matches!(s.kind, ShapeKind::TextFrame(_)))
        .map(|s| (s.id(), s))
        .collect()
}

/// The `TextFrameData` of a text-frame shape.
fn frame_data(shape: &Shape) -> Option<&TextFrameData> {
    match &shape.kind {
        ShapeKind::TextFrame(data) => Some(data),
        _ => None,
    }
}

/// The box that links to `id`, if any.
pub fn previous(page: &Page, id: ObjectId) -> Option<ObjectId> {
    text_frames(page)
        .values()
        .find(|s| frame_data(s).and_then(|d| d.linked_frame) == Some(id))
        .map(|s| s.id())
}

/// The first box of `id`'s chain (`id` itself if nothing links to it).
pub fn head(page: &Page, id: ObjectId) -> ObjectId {
    let frames = text_frames(page);
    let mut current = id;
    for _ in 0..MAX_CHAIN {
        let prev = frames.values().find(|s| frame_data(s).and_then(|d| d.linked_frame) == Some(current));
        match prev {
            Some(prev) if prev.id() != id => current = prev.id(),
            _ => break,
        }
    }
    current
}

/// The boxes of `id`'s chain, first to last.
pub fn chain(page: &Page, id: ObjectId) -> Vec<ObjectId> {
    let frames = text_frames(page);
    let mut out = vec![head(page, id)];
    while out.len() < MAX_CHAIN {
        let next = frames.get(out.last().unwrap()).and_then(|s| frame_data(s)).and_then(|d| d.linked_frame);
        match next {
            Some(n) if frames.contains_key(&n) && !out.contains(&n) => out.push(n),
            _ => break,
        }
    }
    out
}

/// Every linked box on `page` as drawn: the first box shows the story up to
/// where it stops fitting, each next box the rest from there. `overflow`
/// reports where a frame's text stops fitting (`None` when it all fits).
pub fn layout_chains(page: &Page, overflow: impl Fn(&Shape) -> Option<TextPos>) -> HashMap<ObjectId, ChainFrame> {
    let frames = text_frames(page);
    let mut out = HashMap::new();
    for shape in frames.values() {
        let id = shape.id();
        let data = frame_data(shape).unwrap();
        let starts_chain = data.linked_frame.is_some()
            && !frames.values().any(|o| frame_data(o).and_then(|d| d.linked_frame) == Some(id));
        if !starts_chain {
            continue;
        }
        let ids = chain(page, id);
        let mut story = data.content.clone();
        let mut start = TextPos::new(0, 0);
        for (i, box_id) in ids.iter().enumerate() {
            let mut frame = (*frames[box_id]).clone();
            if let ShapeKind::TextFrame(d) = &mut frame.kind {
                d.content = story.clone();
            }
            let last = i + 1 == ids.len();
            let split = if last { None } else { overflow(&frame) };
            if let Some(pos) = split {
                if let ShapeKind::TextFrame(d) = &mut frame.kind {
                    d.content = text_before(&story, pos);
                }
            }
            out.insert(*box_id, ChainFrame { frame, start, head: id, last });
            match split {
                Some(pos) => {
                    story = text_from(&story, pos);
                    start = to_story(start, pos);
                }
                None => {
                    // Everything fitted: later boxes are empty
                    start = to_story(start, end_pos(&story));
                    story = StyledText::default();
                }
            }
        }
    }
    out
}

/// Link `from` (which must have no next box) to the empty, unlinked box `to`.
/// Returns false (changing nothing) if they can't be linked.
pub fn link(page: &mut Page, from: ObjectId, to: ObjectId) -> bool {
    if from == to || chain(page, from).contains(&to) || previous(page, to).is_some() {
        return false;
    }
    let ok_to = matches!(
        page.get_shape(to).and_then(|s| frame_data(s)),
        Some(d) if d.linked_frame.is_none() && d.content.plain_text().trim().is_empty()
    );
    let ok_from = matches!(
        page.get_shape(from).and_then(|s| frame_data(s)),
        Some(d) if d.linked_frame.is_none()
    );
    if !ok_to || !ok_from {
        return false;
    }
    if let Some(ShapeKind::TextFrame(d)) = page.get_shape_mut(from).map(|s| &mut s.kind) {
        d.link_to(to);
    }
    if let Some(ShapeKind::TextFrame(d)) = page.get_shape_mut(to).map(|s| &mut s.kind) {
        d.content = StyledText::default();
        d.is_continuation = true;
    }
    true
}

/// Break the link from `from` to its next box; the text that flowed on stays
/// in the story (overflowing `from`).
pub fn break_link(page: &mut Page, from: ObjectId) {
    let next = match page.get_shape_mut(from).map(|s| &mut s.kind) {
        Some(ShapeKind::TextFrame(d)) => {
            d.overflow = TextOverflow::default();
            d.linked_frame.take()
        }
        _ => None,
    };
    if let Some(ShapeKind::TextFrame(d)) = next.and_then(|n| page.get_shape_mut(n)).map(|s| &mut s.kind) {
        d.is_continuation = false;
    }
}

/// Keep chains whole when the boxes `ids` are about to be deleted: links skip
/// deleted boxes, and a deleted first box hands the story to the next one.
pub fn before_delete(page: &mut Page, ids: &[ObjectId]) {
    for &id in ids {
        let Some(data) = page.get_shape(id).and_then(|s| frame_data(s)) else { continue };
        let (next, content) = (data.linked_frame, data.content.clone());
        match previous(page, id) {
            Some(prev) => {
                if let Some(ShapeKind::TextFrame(d)) = page.get_shape_mut(prev).map(|s| &mut s.kind) {
                    d.linked_frame = next;
                    if next.is_none() {
                        d.overflow = TextOverflow::default();
                    }
                }
            }
            None => {
                if let Some(ShapeKind::TextFrame(d)) = next.and_then(|n| page.get_shape_mut(n)).map(|s| &mut s.kind) {
                    d.content = content;
                    d.is_continuation = false;
                }
            }
        }
        if let Some(ShapeKind::TextFrame(d)) = page.get_shape_mut(id).map(|s| &mut s.kind) {
            d.linked_frame = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use repub_geometry::Rect;
    use repub_model::{Document, PageOrientation, PageSize};

    fn page_with_two_frames() -> (Page, ObjectId, ObjectId) {
        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        let page = &mut doc.pages_mut()[0];
        let a = page.add_shape(Shape::text_frame_with_text(Rect::new(0.0, 0.0, 100.0, 50.0), "Hello"));
        let b = page.add_shape(Shape::text_frame(Rect::new(0.0, 60.0, 100.0, 50.0)));
        (page.clone(), a, b)
    }

    #[test]
    fn linking_and_chain_order() {
        let (mut page, a, b) = page_with_two_frames();
        assert!(link(&mut page, a, b));
        assert_eq!(chain(&page, a), vec![a, b]);
        assert_eq!(chain(&page, b), vec![a, b]);
        assert_eq!(head(&page, b), a);
        assert_eq!(previous(&page, b), Some(a));
        // The continuation box is emptied and flagged
        match &page.get_shape(b).unwrap().kind {
            ShapeKind::TextFrame(d) => assert!(d.is_continuation),
            _ => unreachable!(),
        }
    }

    #[test]
    fn cannot_link_into_a_loop_or_non_empty_box() {
        let (mut page, a, b) = page_with_two_frames();
        assert!(link(&mut page, a, b));
        assert!(!link(&mut page, b, a), "would form a loop");
    }

    #[test]
    fn break_link_clears_the_chain() {
        let (mut page, a, b) = page_with_two_frames();
        assert!(link(&mut page, a, b));
        break_link(&mut page, a);
        assert_eq!(chain(&page, a), vec![a]);
        match &page.get_shape(a).unwrap().kind {
            ShapeKind::TextFrame(d) => assert!(d.linked_frame.is_none()),
            _ => unreachable!(),
        }
    }

    #[test]
    fn story_position_mapping_round_trips() {
        let start = TextPos::new(2, 5);
        let pos = TextPos::new(0, 3);
        let story = to_story(start, pos);
        assert_eq!(story, TextPos::new(2, 8));
        assert_eq!(from_story(start, story), Some(pos));
    }
}
