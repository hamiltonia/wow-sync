//! Small widget helpers. Every size goes through `u` so `--zoom 1.5` simulates 150 % app scale.

use crate::model::Icon;
use gpui::{prelude::*, px, rgb, svg, Pixels, Svg};
use std::sync::atomic::{AtomicU32, Ordering};

static ZOOM_BITS: AtomicU32 = AtomicU32::new(0x3f80_0000);

pub fn set_zoom(z: f32) {
    ZOOM_BITS.store(z.clamp(0.5, 3.0).to_bits(), Ordering::Relaxed);
}

pub fn zoom() -> f32 {
    f32::from_bits(ZOOM_BITS.load(Ordering::Relaxed))
}

pub fn u(n: f32) -> Pixels {
    px(n * zoom())
}

pub fn icon(i: Icon, size: f32, color: u32) -> Svg {
    svg()
        .path(i.path())
        .size(u(size))
        .flex_none()
        .text_color(rgb(color))
}
