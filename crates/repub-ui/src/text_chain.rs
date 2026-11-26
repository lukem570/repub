//! Linked text boxes: a story starts in the first box of a chain
//! (`TextFrame::linked_frame` points to the next box) and the text that
//! doesn't fit continues in the next one. The text lives in the first box;
//! the rest of the chain is laid out from it each time it's drawn.

use std::collections::HashMap;

use repub_model::Page;
use repub_model::ObjectId;
use repub_model::shape::{Shape, ShapeKind};
use repub_model::text::StyledText;

use crate::text_edit::{delete_range, end_pos, TextPos};

/// Longest chain followed (guards against link loops).
const MAX_CHAIN: usize = 64;

/// A box of a chain as drawn: its frame with the part of the story it shows,
/// and where that part starts in the story.
#[derive(Debug, Clone)]
pub struct ChainFrame {
    pub frame: Shape,
    pub start: TextPos,
    pub head: ObjectId,
    /// The chain's last box (it shows the caret at the very end of the story).
    pub last: bool,
}

/// The [`repub_model::shape::TextFrameData`] payload of a text-frame shape.
fn data(shape: &Shape) -> Option<&repub_model::shape::TextFrameData> {
    match &shape.kind {
        ShapeKind::TextFrame(tf) => Some(tf),
        _ => None,
    }
}

/// Where `tf`'s text stops fitting: the start of its first line below the
/// text area.
pub fn overflow_pos(tf: &Shape) -> Option<TextPos> {
    let layout = crate::sdf_text::layout_frame(tf);
    let margins = data(tf)?.effective_margins();
    let b = tf.common.bounds;
    let bottom = (b.origin.y + b.size.height - margins.bottom) as f32;
    layout.lines.iter().find(|l| l.bottom > bottom + 0.5).map(|l| TextPos::new(l.para, l.start))
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

fn text_frames(page: &Page) -> HashMap<ObjectId, &Shape> {
    page.layers()
        .iter()
        .flat_map(|l| l.shapes())
        .filter_map(|o| match &o.kind {
            ShapeKind::TextFrame(_) => Some((o.common.id, o)),
            _ => None,
        })
        .collect()
}

/// The box that links to `id`, if any.
pub fn previous(page: &Page, id: ObjectId) -> Option<ObjectId> {
    text_frames(page).values().find(|tf| data(tf).and_then(|d| d.linked_frame) == Some(id)).map(|tf| tf.common.id)
}

/// The first box of `id`'s chain (`id` itself if nothing links to it).
pub fn head(page: &Page, id: ObjectId) -> ObjectId {
    let frames = text_frames(page);
    let mut current = id;
    for _ in 0..MAX_CHAIN {
        match frames.values().find(|tf| data(tf).and_then(|d| d.linked_frame) == Some(current)) {
            Some(prev) if prev.common.id != id => current = prev.common.id,
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
        let next = frames.get(out.last().unwrap()).and_then(|tf| data(tf).and_then(|d| d.linked_frame));
        match next {
            Some(n) if frames.contains_key(&n) && !out.contains(&n) => out.push(n),
            _ => break,
        }
    }
    out
}

/// Every linked box on `page` as drawn: the first box shows the story up to
/// where it stops fitting, each next box the rest from there.
pub fn layout_chains(page: &Page) -> HashMap<ObjectId, ChainFrame> {
    let frames = text_frames(page);
    let mut out = HashMap::new();
    for tf in frames.values() {
        let tf_linked = data(tf).and_then(|d| d.linked_frame);
        let starts_chain = tf_linked.is_some() && !frames.values().any(|o| data(o).and_then(|d| d.linked_frame) == Some(tf.common.id));
        if !starts_chain {
            continue;
        }
        let ids = chain(page, tf.common.id);
        let mut story = data(tf).map(|d| d.content.clone()).unwrap_or_default();
        let mut start = TextPos::new(0, 0);
        for (i, id) in ids.iter().enumerate() {
            let mut frame = (*frames[id]).clone();
            if let ShapeKind::TextFrame(d) = &mut frame.kind {
                d.content = story.clone();
            }
            let last = i + 1 == ids.len();
            let split = if last { None } else { overflow_pos(&frame) };
            if let Some(pos) = split {
                if let ShapeKind::TextFrame(d) = &mut frame.kind {
                    d.content = text_before(&story, pos);
                }
            }
            out.insert(*id, ChainFrame { frame, start, head: tf.common.id, last });
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
    let ok_to = matches!(page.get_shape(to).map(|s| &s.kind), Some(ShapeKind::TextFrame(t)) if t.linked_frame.is_none() && t.content.plain_text().trim().is_empty());
    let ok_from = matches!(page.get_shape(from).map(|s| &s.kind), Some(ShapeKind::TextFrame(t)) if t.linked_frame.is_none());
    if !ok_to || !ok_from {
        return false;
    }
    if let Some(ShapeKind::TextFrame(t)) = page.get_shape_mut(from).map(|s| &mut s.kind) {
        t.link_to(to);
    }
    if let Some(ShapeKind::TextFrame(t)) = page.get_shape_mut(to).map(|s| &mut s.kind) {
        t.content = StyledText::default();
        t.is_continuation = true;
    }
    true
}

/// Break the link from `from` to its next box; the text that flowed on stays
/// in the story (overflowing `from`).
pub fn break_link(page: &mut Page, from: ObjectId) {
    let next = match page.get_shape_mut(from).map(|s| &mut s.kind) {
        Some(ShapeKind::TextFrame(t)) => {
            t.overflow = Default::default();
            t.linked_frame.take()
        }
        _ => None,
    };
    if let Some(ShapeKind::TextFrame(t)) = next.and_then(|n| page.get_shape_mut(n)).map(|s| &mut s.kind) {
        t.is_continuation = false;
    }
}

/// Keep chains whole when the boxes `ids` are about to be deleted: links skip
/// deleted boxes, and a deleted first box hands the story to the next one.
pub fn before_delete(page: &mut Page, ids: &[ObjectId]) {
    for &id in ids {
        let Some(ShapeKind::TextFrame(tf)) = page.get_shape(id).map(|s| &s.kind) else { continue };
        let (next, content) = (tf.linked_frame, tf.content.clone());
        match previous(page, id) {
            Some(prev) => {
                if let Some(ShapeKind::TextFrame(p)) = page.get_shape_mut(prev).map(|s| &mut s.kind) {
                    p.linked_frame = next;
                    if next.is_none() {
                        p.overflow = Default::default();
                    }
                }
            }
            None => {
                if let Some(ShapeKind::TextFrame(n)) = next.and_then(|n| page.get_shape_mut(n)).map(|s| &mut s.kind) {
                    n.content = content;
                    n.is_continuation = false;
                }
            }
        }
        if let Some(ShapeKind::TextFrame(tf)) = page.get_shape_mut(id).map(|s| &mut s.kind) {
            tf.linked_frame = None;
        }
    }
}
