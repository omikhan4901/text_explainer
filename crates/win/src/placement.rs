//! Where the reading card goes: next to the selection, never over it, always fully on
//! the monitor the selection is on. Pure geometry so it's tested on every platform.

/// A rectangle in physical screen pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Rect {
            left,
            top,
            right,
            bottom,
        }
    }
    pub fn point(x: i32, y: i32) -> Self {
        Rect {
            left: x,
            top: y,
            right: x,
            bottom: y,
        }
    }
    pub fn width(&self) -> i32 {
        self.right - self.left
    }
    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }
    pub fn is_empty(&self) -> bool {
        self.width() <= 0 && self.height() <= 0
    }
    /// The smallest rectangle containing both.
    pub fn union(&self, other: &Rect) -> Rect {
        Rect {
            left: self.left.min(other.left),
            top: self.top.min(other.top),
            right: self.right.max(other.right),
            bottom: self.bottom.max(other.bottom),
        }
    }
}

/// Top-left corner for a card of `size` (width, height) near `anchor` inside `work`.
///
/// Prefers just below the anchor, left-aligned with it; then just above; otherwise it
/// sits at the bottom of the screen. A selection taller than half the screen (a whole
/// page) puts the card beside the cursor instead, which `anchor` should then be.
pub fn place(anchor: Rect, size: (i32, i32), work: Rect, gap: i32) -> (i32, i32) {
    let (w, h) = size;
    let w = w.min(work.width());
    let h = h.min(work.height());

    let x = anchor
        .left
        .clamp(work.left, (work.right - w).max(work.left));
    let below = anchor.bottom + gap;
    let above = anchor.top - gap - h;
    let y = if below + h <= work.bottom {
        below
    } else if above >= work.top {
        above
    } else {
        // Neither fits: overlap as little as possible, at the bottom of the screen.
        work.bottom - h
    };
    (x, y.clamp(work.top, (work.bottom - h).max(work.top)))
}

/// Which side of the selection the card sits on. Chosen once when the card opens (for
/// its largest size) so it doesn't jump as text streams in and it grows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// Below the selection; the card grows downwards.
    Below,
    /// Above the selection; the card grows upwards.
    Above,
    /// Neither fits: pinned to the bottom of the screen.
    Bottom,
}

pub fn choose_side(anchor: Rect, max_height: i32, work: Rect, gap: i32) -> Side {
    let h = max_height.min(work.height());
    if anchor.bottom + gap + h <= work.bottom {
        Side::Below
    } else if anchor.top - gap - h >= work.top {
        Side::Above
    } else {
        Side::Bottom
    }
}

/// Top-left corner for a card of `size` on a fixed `side` of `anchor`.
pub fn place_on_side(
    anchor: Rect,
    size: (i32, i32),
    work: Rect,
    gap: i32,
    side: Side,
) -> (i32, i32) {
    let (w, h) = (size.0.min(work.width()), size.1.min(work.height()));
    let x = anchor
        .left
        .clamp(work.left, (work.right - w).max(work.left));
    let y = match side {
        Side::Below => anchor.bottom + gap,
        Side::Above => anchor.top - gap - h,
        Side::Bottom => work.bottom - h,
    };
    (x, y.clamp(work.top, (work.bottom - h).max(work.top)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORK: Rect = Rect {
        left: 0,
        top: 0,
        right: 1920,
        bottom: 1040,
    };

    #[test]
    fn goes_below_the_selection() {
        let sel = Rect::new(300, 200, 700, 260);
        assert_eq!(place(sel, (440, 300), WORK, 8), (300, 268));
    }

    #[test]
    fn goes_above_when_there_is_no_room_below() {
        let sel = Rect::new(300, 900, 700, 960);
        assert_eq!(place(sel, (440, 300), WORK, 8), (300, 592));
    }

    #[test]
    fn stays_on_screen_horizontally() {
        let sel = Rect::new(1800, 200, 1900, 220);
        let (x, _) = place(sel, (440, 300), WORK, 8);
        assert_eq!(x, 1920 - 440);
        let sel = Rect::new(-50, 200, 10, 220);
        assert_eq!(place(sel, (440, 300), WORK, 8).0, 0);
    }

    #[test]
    fn huge_selection_falls_back_to_the_bottom() {
        let sel = Rect::new(100, 50, 1800, 1000);
        assert_eq!(place(sel, (440, 300), WORK, 8), (100, 740));
    }

    #[test]
    fn respects_monitors_with_offsets_and_taskbars() {
        // A second monitor to the left, with the taskbar on top.
        let work = Rect::new(-1600, 40, 0, 900);
        let sel = Rect::point(-1590, 45);
        assert_eq!(place(sel, (440, 300), work, 8), (-1590, 53));
    }

    #[test]
    fn card_larger_than_the_screen_is_shrunk_to_fit() {
        let work = Rect::new(0, 0, 400, 300);
        let (x, y) = place(Rect::point(10, 10), (440, 500), work, 8);
        assert_eq!((x, y), (0, 0));
    }

    #[test]
    fn sides_are_kept_while_the_card_grows() {
        let sel = Rect::new(300, 700, 700, 720);
        // Max card 400 tall doesn't fit below (720+8+400 > 1040) but fits above.
        let side = choose_side(sel, 400, WORK, 8);
        assert_eq!(side, Side::Above);
        // Small at first, then taller: the bottom edge stays put above the selection.
        let (_, y1) = place_on_side(sel, (440, 120), WORK, 8, side);
        let (_, y2) = place_on_side(sel, (440, 300), WORK, 8, side);
        assert_eq!(y1 + 120, 692);
        assert_eq!(y2 + 300, 692);
        let below = choose_side(Rect::new(0, 100, 10, 120), 400, WORK, 8);
        assert_eq!(below, Side::Below);
        assert_eq!(
            place_on_side(Rect::new(0, 100, 10, 120), (440, 200), WORK, 8, below).1,
            128
        );
    }

    #[test]
    fn full_screen_selection_uses_the_bottom() {
        let sel = Rect::new(0, 10, 1900, 1030);
        assert_eq!(choose_side(sel, 400, WORK, 8), Side::Bottom);
        assert_eq!(place_on_side(sel, (440, 200), WORK, 8, Side::Bottom).1, 840);
    }

    #[test]
    fn rect_helpers() {
        let r = Rect::new(0, 0, 10, 10);
        assert!(r.contains(5, 5));
        assert!(!r.contains(10, 5));
        assert_eq!(r.union(&Rect::new(5, 5, 20, 30)), Rect::new(0, 0, 20, 30));
        assert!(Rect::point(3, 3).is_empty());
    }
}
