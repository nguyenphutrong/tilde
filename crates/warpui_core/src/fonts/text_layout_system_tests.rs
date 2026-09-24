use super::*;
use crate::fonts::{FamilyId, Properties};
use crate::text_layout::TextStyle;

struct CheckingLayoutSystem;

impl platform::TextLayoutSystem for CheckingLayoutSystem {
    fn layout_line(
        &self,
        _text: &str,
        _line_style: LineStyle,
        _style_runs: &[(Range<usize>, StyleAndFont)],
        _max_width: f32,
        _clip_config: ClipConfig,
    ) -> Line {
        unreachable!("text frames should not use the line API")
    }

    fn layout_text(
        &self,
        text: &str,
        line_style: LineStyle,
        style_runs: &[(Range<usize>, StyleAndFont)],
        max_width: f32,
        max_height: f32,
        alignment: TextAlignment,
        first_line_head_indent: Option<f32>,
    ) -> TextFrame {
        assert_eq!(text, "ab");
        assert_eq!(style_runs.len(), 1);
        assert_eq!(style_runs[0].0, 0..2);
        assert_eq!(max_width, 71.0);
        assert_eq!(max_height, 29.0);
        assert_eq!(alignment, TextAlignment::Left);
        assert_eq!(first_line_head_indent, Some(3.0));
        TextFrame::empty(line_style.font_size, line_style.line_height_ratio)
    }
}

#[test]
fn uncached_layout_strips_bom_and_adjusts_styles_before_platform_shaping() {
    let cache = FontFallbackCache::default();
    let system = TextLayoutSystem {
        platform: &CheckingLayoutSystem,
        cache: &cache,
    };
    let styles = [(
        0..3,
        StyleAndFont::new(FamilyId(0), Properties::default(), TextStyle::default()),
    )];
    let frame = system.layout_text_uncached(
        "\u{FEFF}ab",
        LineStyle {
            font_size: 12.0,
            line_height_ratio: 1.0,
            baseline_ratio: 0.8,
            fixed_width_tab_size: None,
        },
        &styles,
        71.0,
        29.0,
        TextAlignment::Left,
        Some(3.0),
    );
    assert_eq!(frame.height(), 12.0);
}
