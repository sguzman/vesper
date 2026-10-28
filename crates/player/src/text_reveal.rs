//! Frame-rate-independent, grapheme-safe text reveal primitives.
//!
//! Build the boundary table once for each newly displayed dialogue line.
//! Per-frame work only advances a counter and borrows an existing UTF-8 prefix.
use unicode_segmentation::UnicodeSegmentation;
pub use vesper_game::TextRevealSpeed as RevealSpeed;

pub const MAX_ANIMATED_BYTES: usize = 32 * 1024;

#[derive(Debug, Clone)]
pub struct RevealBuffer {
    full: String,
    ends: Vec<usize>,
    visible: usize,
    elapsed: f64,
    speed: RevealSpeed,
}

impl RevealBuffer {
    pub fn new(full: String, speed: RevealSpeed) -> Self {
        let animate = speed.rate().is_some() && full.len() <= MAX_ANIMATED_BYTES;
        let ends = if animate {
            UnicodeSegmentation::grapheme_indices(full.as_str(), true)
                .map(|(start, grapheme)| start + grapheme.len())
                .collect()
        } else {
            Vec::new()
        };
        let visible = if animate { 0 } else { ends.len() };
        Self { full, ends, visible, elapsed: 0.0, speed }
    }

    pub fn is_complete(&self) -> bool {
        self.ends.is_empty() || self.visible == self.ends.len()
    }

    pub fn visible_text(&self) -> &str {
        if self.is_complete() {
            &self.full
        } else if self.visible == 0 {
            ""
        } else {
            &self.full[..self.ends[self.visible - 1]]
        }
    }

    pub fn complete_now(&mut self) -> bool {
        if self.is_complete() { return false; }
        self.visible = self.ends.len();
        true
    }

    /// Elapsed wall time determines the number revealed, not frames or frame
    /// count. Large pauses/NaN/negative values do not corrupt the buffer.
    pub fn advance(&mut self, seconds: f32) -> bool {
        if self.is_complete() || !seconds.is_finite() || seconds <= 0.0 {
            return false;
        }
        let Some(rate) = self.speed.rate() else { return false; };
        self.elapsed = (self.elapsed + f64::from(seconds)).min(86_400.0);
        // f32 frame deltas can land a few ULPs below an exact boundary.
        let next = (self.elapsed.mul_add(rate, 1e-6)).floor() as usize;
        let next = next.min(self.ends.len());
        if next == self.visible { return false; }
        self.visible = next;
        true
    }

    pub fn set_speed(&mut self, speed: RevealSpeed) {
        if speed == RevealSpeed::Instant {
            self.complete_now();
        } else if self.speed != speed && !self.is_complete() {
            // Preserve the exact already-visible grapheme prefix.
            let count = self.visible;
            self.elapsed = count as f64 / speed.rate().unwrap_or(1.0);
        }
        self.speed = speed;
    }

    pub fn full_text(&self) -> &str { &self.full }

    pub fn remaining_seconds(&self) -> f32 {
        if self.is_complete() { return 0.0; }
        match self.speed.rate() {
            Some(rate) => ((self.ends.len() as f64 / rate) - self.elapsed)
                .max(0.0).min(86_400.0) as f32,
            None => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unicode_clusters_never_split_combining_marks_flags_or_emoji_families() {
        let message = "e\u{301}👨‍👩‍👧‍👦🇲🇽नमस्ते".to_owned();
        let mut buffer = RevealBuffer::new(message.clone(), RevealSpeed::Normal);
        assert_eq!(buffer.visible_text(), "");
        assert!(buffer.advance(1.0 / 34.0));
        assert_eq!(buffer.visible_text(), "e\u{301}");
        assert!(buffer.advance(1.0 / 34.0));
        assert_eq!(buffer.visible_text(), "e\u{301}👨‍👩‍👧‍👦");
        assert!(buffer.advance(1.0 / 34.0));
        assert_eq!(buffer.visible_text(), "e\u{301}👨‍👩‍👧‍👦🇲🇽");
        buffer.complete_now();
        assert_eq!(buffer.visible_text(), message);
    }

    #[test]
    fn progression_is_independent_of_frame_chunking() {
        let text = "A lantern rises above the starlight ✨".to_string();
        let mut frames = RevealBuffer::new(text.clone(), RevealSpeed::Slow);
        let mut one_frame = RevealBuffer::new(text, RevealSpeed::Slow);
        for _ in 0..25 { frames.advance(0.04); }
        one_frame.advance(1.0);
        assert_eq!(frames.visible_text(), one_frame.visible_text());
        assert_eq!(frames.is_complete(), one_frame.is_complete());
    }

    #[test]
    fn first_skip_finishes_current_line_without_advancing_machine() {
        let mut line = RevealBuffer::new("One 🌙 gentle line".into(), RevealSpeed::Normal);
        assert!(line.complete_now());
        assert!(line.is_complete());
        assert_eq!(line.visible_text(), "One 🌙 gentle line");
        assert!(!line.complete_now());
    }

    #[test]
    fn large_or_empty_lines_are_instant_and_safe() {
        let huge = "é".repeat(MAX_ANIMATED_BYTES);
        let huge_buffer = RevealBuffer::new(huge.clone(), RevealSpeed::Slow);
        assert!(huge_buffer.is_complete());
        assert_eq!(huge_buffer.visible_text(), huge);
        assert!(RevealBuffer::new(String::new(), RevealSpeed::Slow).is_complete());
        let instant = RevealBuffer::new("👍🏽".into(), RevealSpeed::Instant);
        assert_eq!(instant.visible_text(), "👍🏽");
    }

    #[test]
    fn instant_and_speed_changes_preserve_grapheme_boundary() {
        let mut buf = RevealBuffer::new("abcd".into(), RevealSpeed::Slow);
        buf.advance(1.0 / 14.0);
        assert_eq!(buf.visible_text(), "a");
        buf.set_speed(RevealSpeed::Fast);
        assert_eq!(buf.visible_text(), "a");
        assert!(buf.advance(1.0 / 80.0));
        assert_eq!(buf.visible_text(), "ab");
        buf.set_speed(RevealSpeed::Instant);
        assert_eq!(buf.visible_text(), "abcd");
        assert!(buf.is_complete());
    }

    #[test]
    fn broken_elapsed_time_cannot_corrupt_unicode() {
        let mut buf = RevealBuffer::new("e\u{301}👩‍💻".into(), RevealSpeed::Fast);
        assert!(!buf.advance(f32::NAN));
        assert!(!buf.advance(-1.0));
        buf.advance(1000.0);
        assert_eq!(buf.visible_text(), "e\u{301}👩‍💻");
        assert_eq!(buf.remaining_seconds(), 0.0);
    }
}
