use cgmath_geometry::{
    D2,
    cgmath::Point2,
    rect::BoundBox,
};
use crate::rect_layout::{
    Rect, RectFill,
    theme::{Color, ImageId},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextureVertex {
    pub position: Point2<f32>,
    pub texture_coordinate: Point2<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorVertex {
    pub position: Point2<f32>,
    pub color: Color,
}

/// A rect comprised of individual vertices.
///
/// Vertices are laid out as follows:
/// ```text
/// 0---1
/// |   |
/// 2---3
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VertexRect {
    Color([ColorVertex; 4]),
    Texture {
        image_id: ImageId,
        vertices: [TextureVertex; 4]
    },
}

impl VertexRect {
    // 0-1
    // |/|
    // 2-3
    pub const INDICES_CLOCKWISE: [u16; 6] = [0, 1, 2, 3, 2, 1];
    pub const INDICES_COUNTERCLOCKWISE: [u16; 6] = [0, 2, 1, 3, 1, 2];

    pub fn new_fill(rect: BoundBox<D2, f32>, fill: RectFill) -> VertexRect {
        let rect_points = [
            Point2::new(rect.min.x, rect.min.y),
            Point2::new(rect.max.x, rect.min.y),
            Point2::new(rect.min.x, rect.max.y),
            Point2::new(rect.max.x, rect.max.y),
        ];
        match fill {
            RectFill::Color(color) => VertexRect::Color([
                ColorVertex::new(rect_points[0], color),
                ColorVertex::new(rect_points[1], color),
                ColorVertex::new(rect_points[2], color),
                ColorVertex::new(rect_points[3], color),
            ]),
            RectFill::Image{image_id, subrect} => {
                let texture_points = [
                    Point2::new(subrect.min.x, subrect.min.y),
                    Point2::new(subrect.max.x, subrect.min.y),
                    Point2::new(subrect.min.x, subrect.max.y),
                    Point2::new(subrect.max.x, subrect.max.y),
                ];
                VertexRect::Texture {
                    image_id,
                    vertices: [
                        TextureVertex::new(rect_points[0], texture_points[0]),
                        TextureVertex::new(rect_points[1], texture_points[1]),
                        TextureVertex::new(rect_points[2], texture_points[2]),
                        TextureVertex::new(rect_points[3], texture_points[3]),
                    ],
                }
            },
        }

    }

    pub fn indices_clockwise_offset(offset: u16) -> [u16; 6] {
        array_offset(offset, Self::INDICES_CLOCKWISE)
    }

    pub fn indices_counterclockwise_offset(offset: u16) -> [u16; 6] {
        array_offset(offset, Self::INDICES_COUNTERCLOCKWISE)
    }

    pub fn image_id(self) -> Option<ImageId> {
        match self {
            VertexRect::Color(_) => None,
            VertexRect::Texture{image_id, ..} => Some(image_id),
        }
    }

    pub fn map_color<V>(self, mut map_color: impl FnMut(ColorVertex) -> V) -> Result<[V; 4], VertexRect> {
        match self {
            VertexRect::Color(color_verts) => Ok([
                map_color(color_verts[0]),
                map_color(color_verts[1]),
                map_color(color_verts[2]),
                map_color(color_verts[3]),
            ]),
            VertexRect::Texture{..} => Err(self)
        }
    }

    pub fn map_texture<V>(self, mut map_texture: impl FnMut(ImageId, TextureVertex) -> V) -> Result<[V; 4], VertexRect> {
        match self {
            VertexRect::Color(_) => Err(self),
            VertexRect::Texture{image_id, vertices} => Ok([
                map_texture(image_id, vertices[1]),
                map_texture(image_id, vertices[0]),
                map_texture(image_id, vertices[2]),
                map_texture(image_id, vertices[3]),
            ]),
        }
    }

    pub fn map_unify<V>(
        self,
        mut map_color: impl FnMut(ColorVertex) -> V,
        mut map_texture: impl FnMut(ImageId, TextureVertex) -> V,
    ) -> [V; 4] {
        match self {
            VertexRect::Color(color_verts) => [
                map_color(color_verts[0]),
                map_color(color_verts[1]),
                map_color(color_verts[2]),
                map_color(color_verts[3]),
            ],
            VertexRect::Texture{image_id, vertices} => [
                map_texture(image_id, vertices[1]),
                map_texture(image_id, vertices[0]),
                map_texture(image_id, vertices[2]),
                map_texture(image_id, vertices[3]),
            ],
        }
    }
}

impl TextureVertex {
    pub fn new(position: Point2<f32>, texture_coordinate: Point2<i32>) -> TextureVertex {
        TextureVertex{ position, texture_coordinate }
    }
}

impl ColorVertex {
    pub fn new(position: Point2<f32>, color: Color) -> ColorVertex {
        ColorVertex{ position, color }
    }
}

impl From<Rect> for VertexRect {
    fn from(rect: Rect) -> VertexRect {
        VertexRect::new_fill(rect.rect.cast().unwrap(), rect.fill)
    }
}

#[inline(always)]
fn array_offset(offset: u16, mut array: [u16; 6]) -> [u16; 6] {
    for i in &mut array {
        *i += offset;
    }

    array
}
