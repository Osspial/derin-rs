extern crate tint;
extern crate glutin;

use tint::draw::*;
use tint::draw::primitives::*;
use tint::draw::gl::{Facade, ShaderDataCollector};
use tint::draw::font::{Font, FontInfo};

struct CompositeRects {
    rect: Rect,
    outer_color: ColorRect,
    inner_color: ColorRect,
    text: TextBox<&'static str>
}

impl Shadable for CompositeRects {
    fn shader_data(&self, mut data: ShaderDataCollector) {
        data.with_rect(self.rect);
        data.with_mask(&[
                Complex::new_rat(-1.0, 0.0),
                Complex::new_rat(1.0, -1.0),
                Complex::new_rat(-1.0, 1.0),
                Complex::new_rat(0.7, 0.7)
            ],
            &[[0, 1, 2], [2, 3, 1]]);

        self.outer_color.shader_data(data.take());
        self.inner_color.shader_data(data.take());
        self.text.shader_data(data.take());
    }
}


fn main() {
    let window = glutin::WindowBuilder::new()
        .with_dimensions(500, 500)
        .with_pixel_format(24, 8)
        .with_depth_buffer(24)
        .with_multisampling(4)
        .build().unwrap();

    unsafe{ window.make_current().unwrap() };

    let mut display = Facade::new(|s| window.get_proc_address(s) as *const _);
    let font = Font::new(&FontInfo {
        regular: "./tests/DejaVuSans.ttf".into(),
        italic: None,
        bold: None,
        bold_italic: None
    });

    let mut rect = Widget::new(LinearGradient::new(
            Rect::new(
                Complex::new_rat(-0.5, -0.5),
                Complex::new_rat( 0.5,  0.5)
            ),
            vec![
                GradientNode::new(-0.5, Color::new(255, 255, 255, 255)),
                GradientNode::new( 0.0, Color::new(0, 255, 0, 255)),
                GradientNode::new( 1.0, Color::new(255, 0, 0, 255)),
            ],
            0.0
        )
    );

    let rad_grad = Widget::new(RadialGradient {
        rect: Rect::new(
                Complex::new_rat(-1.0, -1.0),
                Complex::new_rat( 0.0,  0.0)
            ),
        nodes: vec![
            GradientNode::new(0.0, Color::new(255, 255, 255, 255)),
            GradientNode::new(0.2, Color::new(255, 0, 0, 255)),
            GradientNode::new(1.0, Color::new(0, 255, 0, 255))
        ],
        subdivs: None
    });

    let ellipse = Widget::new(ColorEllipse {
        rect: Rect::new(
                Complex::new_rat(-0.5, -1.0),
                Complex::new_rat( 0.5,  0.0)
            ),
        color: Color::new(0, 0, 255, 128),
        subdivs: None
    });

    let composite = Widget::new(CompositeRects {
        rect: Rect::new(
                Complex::new_rat(-1.0, 0.0),
                Complex::new_rat( 0.0, 1.0)
            ),
        outer_color: ColorRect::new(
                Rect::new(
                    Complex::new_rat(-1.0, -1.0),
                    Complex::new_rat( 1.0,  1.0)
                ),
                Color::new(255, 0, 0, 255)
            ),
        inner_color: ColorRect::new(
                Rect::new(
                    Complex::new(-1.0, -1.0,  12.0,  12.0),
                    Complex::new( 1.0,  1.0, -12.0, -12.0)
                ),
                Color::new(255, 255, 0, 255)
            ),
        text: TextBox::new(
                Rect::new(
                    Complex::new(-1.0, -1.0,  12.0,  12.0),
                    Complex::new( 1.0,  1.0, -12.0, -12.0)
                ),
                Color::new(0, 127, 255, 255),
                "Greetings, you glorious bastards. Word wrapping works fine, and so d\no ne\nwlines",
                font,
                16
            )
    });

    'main: loop {
        for event in window.poll_events() {
            use glutin::Event::*;

            match event {
                Closed => break 'main,
                Resized(x, y) => display.resize(x, y),
                _ => ()
            }
        }

        let mut surface = display.surface();
        surface.draw(&rect);
        surface.draw(&composite);
        surface.draw(&ellipse);
        surface.draw(&rad_grad);

        window.swap_buffers().unwrap();
        rect.angle += 1.0;
    }
}