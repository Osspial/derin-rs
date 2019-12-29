// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

mod shape_glyphs;

use shape_glyphs::{GlyphIterBuilder, RenderGlyph};
use crate::{
    GraphemeCluster,
    HasLifetimeIterator,
    rect_layout::{
        Rect, RectFill, ImageRectFill,
        theme::{TextLayoutStyle, TextRenderStyle, LineWrap, FontFaceId, ImageId, Color},
    },
};

use crate::cgmath::{EuclideanSpace, Point2, Vector2};
use cgmath_geometry::{D2, rect::{BoundBox, DimsBox, GeoBox}};

use derin_common_types::layout::Align;

use std::ops::Range;
use xi_unicode::LineBreakIterator;
use itertools::Itertools;


// you can think of this as a secretarial version of koh the face stealer.
pub trait FaceManager: 'static + for<'a> HasLifetimeIterator<'a, Glyph> {
    type FaceQuery;

    fn query_face(&mut self, face_query: Self::FaceQuery) -> Option<FontFaceId>;
    fn query_face_best_match(&mut self, face_query: &[Self::FaceQuery]) -> Option<FontFaceId>;
    fn face_metrics(&mut self, face_id: FontFaceId, face_size: u32, dpi: u32) -> FaceMetrics;
    fn glyph_image(&mut self, face: FontFaceId, face_size: u32, dpi: u32, glyph_index: u32, color: Color) -> (ImageId, BoundBox<D2, i32>);
    fn shape_text<'a>(&'a mut self, face_id: FontFaceId, face_size: u32, dpi: u32, text: &'a str) -> <Self as HasLifetimeIterator<'a, Glyph>>::Iter;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Glyph {
    pub glyph_index: u32,
    pub advance: Vector2<i32>,
    pub pos: Point2<i32>,
    pub str_index: usize,
}

/// A face's metrics.
///
/// All values are in 16.16 format.
pub struct FaceMetrics {
    pub line_height: i32,
    pub ascender: i32,
    pub descender: i32,
    pub space_advance: i32,
    pub cursor_width: i32,
}

pub struct TextToRects<'a, G: FaceManager> {
    glyph_slice_index: usize,
    layout_data: &'a StringLayoutData,
    highlight_range: Range<usize>,
    cursor_pos: Option<usize>,

    font_ascender: i32,
    font_descender: i32,
    cursor_width: i32,

    render_style: TextRenderStyle,

    face_manager: &'a mut G,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StringLayoutData {
    shaped_glyphs: Vec<RenderGlyph>,
    dpi: u32,
    layout_style: TextLayoutStyle,
    source_dims: DimsBox<D2, i32>,
    text_rect: BoundBox<D2, i32>,
}

impl StringLayoutData {
    pub fn shape<F>(
        text: &str,
        dims: DimsBox<D2, i32>,
        dpi: u32,
        layout_style: TextLayoutStyle,
        face_manager: &mut F,
    ) -> StringLayoutData
        where F: FaceManager
    {
        let face_metrics = face_manager.face_metrics(layout_style.face, layout_style.face_size, dpi);
        let mut iter_builder = GlyphIterBuilder::new(dims, layout_style, face_metrics);
        let mut last_index = 0;
        for (break_index, hard_break) in LineBreakIterator::new(text) {
            let s = &text[last_index..break_index];
            let glyphs = face_manager.shape_text(layout_style.face, dpi, layout_style.face_size, s);
            iter_builder.add_segment(s, last_index, hard_break, glyphs);
            last_index = break_index;
        }

        let mut render_glyph_iter = iter_builder.build();
        let shaped_glyphs = render_glyph_iter.by_ref().collect::<Vec<_>>();
        let shaped_data = render_glyph_iter.shaped_data();

        StringLayoutData {
            shaped_glyphs,
            layout_style,
            dpi,
            source_dims: dims,
            text_rect: shaped_data.text_rect,
        }
    }

    pub fn min_size(&self) -> Option<DimsBox<D2, i32>> {
        match self.layout_style.line_wrap {
            LineWrap::None |
            LineWrap::Explicit => Some(self.text_rect.dims()),
            LineWrap::Normal => None,
        }
    }

    pub fn grapheme_clusters(&self) -> impl '_ + Iterator<Item=GraphemeCluster> {
        glyphs_to_grapheme_clusters(self.shaped_glyphs.iter().cloned())
    }
}

fn glyphs_to_grapheme_clusters(iter: impl Iterator<Item=RenderGlyph>) -> impl Iterator<Item=GraphemeCluster> {
    iter.map(|glyph| GraphemeCluster {
            range: glyph.str_index..glyph.str_index + glyph.grapheme_len,
            selection_rect: glyph.highlight_rect
        })
        .coalesce(|acc, glyph1| match acc.range == glyph1.range {
            true => Ok(GraphemeCluster {
                range: acc.range,
                selection_rect: BoundBox::new(acc.selection_rect.min, glyph1.selection_rect.max),
            }),
            false => Err((acc, glyph1)),
        })
}

impl<'a, G: FaceManager> TextToRects<'a, G> {
    pub fn new(
        layout_data: &'a StringLayoutData,
        highlight_range: Range<usize>,
        cursor_pos: Option<usize>,
        render_style: TextRenderStyle,

        face_manager: &'a mut G,
    ) -> TextToRects<'a, G>
    {
        let face_metrics = face_manager.face_metrics(layout_data.layout_style.face, layout_data.layout_style.face_size, layout_data.dpi);

        TextToRects {
            glyph_slice_index: 0,
            layout_data,

            highlight_range,
            cursor_pos,

            font_ascender: face_metrics.ascender,
            font_descender: face_metrics.descender,
            cursor_width: face_metrics.cursor_width,

            render_style,

            face_manager,
        }
    }
}

impl<'a, G: FaceManager> Iterator for TextToRects<'a, G> {
    type Item = Rect;

    fn next(&mut self) -> Option<Rect> {
        let TextToRects {
            ref mut glyph_slice_index,
            layout_data,
            ref mut highlight_range,
            ref mut cursor_pos,
            font_ascender,
            font_descender,
            cursor_width,
            render_style,
            ref mut face_manager,
        } = *self;

        macro_rules! get_glyph_slice {
            (range $i:expr) => {{layout_data.shaped_glyphs.get($i).iter().flat_map(|g| g.iter()).cloned()}};
            ($i:expr) => {{layout_data.shaped_glyphs.get($i).cloned()}};
        }
        let next_glyph_opt = get_glyph_slice!(*glyph_slice_index);

        let cursor_rect_opt = try {
            let cursor_pos = (*cursor_pos)?;

            let rect_tl_opt = match next_glyph_opt {
                Some(next_glyph) => {
                    let highlight_rect = next_glyph.highlight_rect;

                    if cursor_pos == next_glyph.str_index {
                        Some(highlight_rect.min())
                    } else if cursor_pos == next_glyph.str_index + next_glyph.grapheme_len {
                        Some(Point2::new(highlight_rect.max().x, highlight_rect.min().y))
                    } else {
                        None
                    }
                },
                None if cursor_pos == 0 => Some(
                    Point2 {
                        x: match layout_data.layout_style.justify.x {
                            Align::Start |
                            Align::Stretch => 0,
                            Align::Center => layout_data.source_dims.width() as i32 / 2,
                            Align::End => layout_data.source_dims.width()
                        },
                        y: match layout_data.layout_style.justify.y {
                            Align::Start => -font_descender,
                            Align::Stretch => layout_data.source_dims.height() as i32 / 2 - font_ascender,
                            Align::Center => (layout_data.source_dims.height() as i32 - font_ascender - font_descender) / 2,
                            Align::End => layout_data.source_dims.height() as i32 - font_ascender,
                        }
                    }
                ),
                None => None
            };

            let rect_tl = rect_tl_opt?;
            Rect {
                rect: BoundBox::new(rect_tl, rect_tl + Vector2::new(cursor_width, font_ascender - font_descender)),
                fill: RectFill::Color(render_style.color),
            }
        };

        if let Some(cursor_rect) = cursor_rect_opt {
            *cursor_pos = None;
            return Some(cursor_rect);
        }


        let next_glyph = next_glyph_opt?;

        let is_highlighted = highlight_range.contains(&next_glyph.str_index);
        let starts_highlight_rect = highlight_range.start == next_glyph.str_index && highlight_range.len() > 0;

        let highlight_rect_opt = match starts_highlight_rect {
            true => {
                // Find the end of this highlight rect. That can occur if either:
                // a) The end of the current line is reached.
                // b) The highlight range ends.
                let rect_end_glyph = get_glyph_slice!(range *glyph_slice_index..)
                    .take_while(|g| g.highlight_rect.min().y == next_glyph.highlight_rect.min().y) // case a
                    .take_while(|g| g.str_index < highlight_range.end) // case b
                    .last().unwrap_or(next_glyph);
                let highlight_rect_end = rect_end_glyph.highlight_rect.max().x;

                // In case a, this moves the start of the highlight range to the start of the next line.
                // This has no effect on the rendering process in case b.
                highlight_range.start = rect_end_glyph.str_index + rect_end_glyph.grapheme_len;

                let mut highlight_rect = next_glyph.highlight_rect;
                highlight_rect.max.x = highlight_rect_end;

                Some(Rect {
                    rect: highlight_rect,
                    fill: RectFill::Color(render_style.highlight_bg_color),
                })
            },
            false => None
        };

        if let Some(highlight_rect) = highlight_rect_opt {
            return Some(highlight_rect);
        }

        let glyph_color = match is_highlighted {
            true => render_style.color,
            false => render_style.highlight_text_color,
        };
        let glyph_image = next_glyph.glyph_index.map(|glyph_index|
            face_manager.glyph_image(
                layout_data.layout_style.face,
                layout_data.layout_style.face_size,
                layout_data.dpi,
                glyph_index,
                glyph_color,
            )
        );
        let glyph_rect_opt = glyph_image.map(|(image, rect)|
            Rect {
                rect: rect + next_glyph.pos.to_vec(),
                fill: RectFill::Image(ImageRectFill {
                    image_id: image,
                    subrect: rect.dims().into(),
                })
            }
        );

        *glyph_slice_index += 1;
        if let Some(glyph_rect) = glyph_rect_opt {
            return Some(glyph_rect);
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::rects_from_string;
    use std::char;

    /// Tests unification of individual glyphs into sets of grapheme clusters.
    #[test]
    fn test_glyphs_to_grapheme_clusters() {
        let clusters = "
            a-+b--+c-Ad---+
            +-aA--b+-c+---d
                e-+f--+
                +-e+--f
        ";
        let clusters = rects_from_string(clusters, true);
        let cluster_ranges = [
            0..1,
            1..3,
            3..4,
            4..5,
            5..6,
        ];
        cluster_ranges.iter().cloned()
            .tuple_windows()
            .for_each(|(a, b)| assert_eq!(a.end, b.start));

        let gen_glyph = |i: usize, c: char| {
            RenderGlyph {
                pos: clusters[&c].min,
                highlight_rect: clusters[&c],
                str_index: cluster_ranges[i].start,
                grapheme_len: cluster_ranges[i].len(),
                glyph_index: None,
            }
        };

        let glyphs = vec![
            gen_glyph(0, 'a'),
            gen_glyph(1, 'b'),
            gen_glyph(1, 'c'),
            gen_glyph(2, 'd'),
            gen_glyph(3, 'e'),
            gen_glyph(4, 'f'),
        ];

        let gen_cluster = |i: usize, c: char| {
            GraphemeCluster {
                range: cluster_ranges[i].clone(),
                selection_rect: clusters[&c],
            }
        };

        assert_eq!(
            vec![
                gen_cluster(0, 'a'),
                gen_cluster(1, 'A'),
                gen_cluster(2, 'd'),
                gen_cluster(3, 'e'),
                gen_cluster(4, 'f'),
            ],
            glyphs_to_grapheme_clusters(glyphs.into_iter()).collect::<Vec<_>>()
        );
    }
}