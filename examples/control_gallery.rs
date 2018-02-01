extern crate derin;
#[macro_use]
extern crate derin_macros;
extern crate glutin;
extern crate png;

extern crate gl_raii;

use derin::dct::buttons::{MouseButton, Key};
use derin::dct::hints::{WidgetPos, NodeSpan, GridSize, Margins, Align, Align2};
use derin::{ButtonHandler, NodeLayout, Button, Group};
use derin::gl_render::GLRenderer;
use derin::core::{LoopFlow, Root, WindowEvent};
use derin::core::tree::NodeIdent;
use derin::theme::{ThemeText, ThemeFace, RescaleRules};
use derin::geometry::{Point2, DimsBox, GeoBox};

use gl_raii::colors::Rgba;
use gl_raii::glsl::Nu8;

use std::rc::Rc;

use glutin::{Event, ControlFlow, WindowEvent as GWindowEvent, MouseButton as GMouseButton, ElementState, VirtualKeyCode};


enum GalleryEvent {}

#[derive(NodeContainer)]
#[derin(action = "GalleryEvent")]
struct BasicContainer {
    button: Button<BasicHandler>,
    nested: Group<NestedContainer, BasicLayoutVertical>
}

#[derive(NodeContainer)]
#[derin(action = "GalleryEvent")]
struct NestedContainer {
    button0: Button<BasicHandler>,
    button1: Button<BasicHandler>
}

struct BasicHandler;
struct BasicLayout;
struct BasicLayoutVertical;

impl ButtonHandler for BasicHandler {
    type Action = GalleryEvent;

    fn on_click(&mut self) -> Option<GalleryEvent> {
        println!("clicked!");
        None
    }
}

impl NodeLayout for BasicLayout {
    fn hints(&self, node_ident: NodeIdent) -> Option<WidgetPos> {
        match node_ident {
            NodeIdent::Str("button") => Some(WidgetPos {
                node_span: NodeSpan::new(0, 0),
                margins: Margins::new(16, 100, 16, 16),
                ..WidgetPos::default()
            }),
            NodeIdent::Str("nested") => Some(WidgetPos {
                node_span: NodeSpan::new(1, 0),
                margins: Margins::new(16, 16, 16, 100),
                ..WidgetPos::default()
            }),
            _ => None
        }
    }
    fn grid_size(&self) -> GridSize {
        GridSize::new(2, 1)
    }
}

impl NodeLayout for BasicLayoutVertical {
    fn hints(&self, node_ident: NodeIdent) -> Option<WidgetPos> {
        match node_ident {
            NodeIdent::Str("button0") => Some(WidgetPos {
                node_span: NodeSpan::new(0, 0),
                margins: Margins::new(16, 16, 16, 16),
                ..WidgetPos::default()
            }),
            NodeIdent::Str("button1") => Some(WidgetPos {
                node_span: NodeSpan::new(0, 1),
                margins: Margins::new(16, 16, 16, 16),
                ..WidgetPos::default()
            }),
            _ => None
        }
    }
    fn grid_size(&self) -> GridSize {
        GridSize::new(1, 2)
    }
}

fn main() {
    let group = Group::new(
        BasicContainer {
            button: Button::new("good day\tgood day good day good day good day \nHello Hello".to_string(), BasicHandler),
            nested: Group::new(NestedContainer {
                button0: Button::new("tr tr".to_string(), BasicHandler),
                button1: Button::new("br".to_string(), BasicHandler)
            }, BasicLayoutVertical)
        },
        BasicLayout
    );

    let dims = DimsBox::new2(512, 512);
    let mut events_loop = glutin::EventsLoop::new();
    let window_builder = glutin::WindowBuilder::new()
        .with_dimensions(dims.width(), dims.height())
        .with_title("Derin Control Gallery");

    let mut renderer = unsafe{ GLRenderer::new(&events_loop, window_builder).unwrap() };

    let mut theme = derin::theme::Theme::new();

    macro_rules! upload_image {
        ($name:expr, $path:expr, $dims:expr, $border:expr) => {{
            let image_png = png::Decoder::new(std::io::Cursor::new(&include_bytes!($path)[..]));
            let (info, mut reader) = image_png.read_info().unwrap();
            // Allocate the output buffer.
            let mut image = vec![0; info.buffer_size()];
            // Read the next frame. Currently this function should only called once.
            // The default options
            reader.next_frame(&mut image).unwrap();
            theme.insert_node(
                $name.to_string(),
                derin::theme::ThemeNode {
                    text: Some(ThemeText {
                        face: ThemeFace::new("./tests/DejaVuSans.ttf", 0).unwrap(),
                        color: Rgba::new(Nu8(255), Nu8(100), Nu8(100), Nu8(255)),
                        face_size: 16 * 64,
                        tab_size: 8,
                        justify: Align2::new(Align::Stretch, Align::Center),
                    }),
                    icon: Some(Rc::new(derin::theme::Image {
                        pixels: unsafe {
                            Vec::from_raw_parts(
                                image.as_mut_ptr() as *mut _,
                                image.len() / 4,
                                image.capacity() / 4
                            )
                        },
                        dims: DimsBox::new2($dims, $dims),
                        rescale: RescaleRules::Slice(Margins::new($border, $border, $border, $border))
                    }))
                }
            );

            ::std::mem::forget(image);
        }}
    }

    upload_image!("Group", "../group.png", 3, 1);
    upload_image!("Button::Normal", "../button.normal.png", 32, 4);
    upload_image!("Button::Hover", "../button.hover.png", 32, 4);
    upload_image!("Button::Clicked", "../button.clicked.png", 32, 4);


    let mut root = Root::new(group, theme, dims);
    root.run_forever(|for_each_event| {
        let mut ret: Option<()> = None;
        events_loop.run_forever(|glutin_event| {
            match glutin_event {
                Event::WindowEvent{event, ..} => {
                    let derin_event_opt: Option<WindowEvent> = match event {
                        GWindowEvent::CursorMoved{position, ..} => Some(WindowEvent::MouseMove(Point2::new(position.0 as i32, position.1 as i32))),
                        GWindowEvent::CursorEntered{..} => Some(WindowEvent::MouseEnter(Point2::new(0, 0))),
                        GWindowEvent::CursorLeft{..} => Some(WindowEvent::MouseExit(Point2::new(0, 0))),
                        GWindowEvent::MouseInput{state, button: g_button, ..} => {
                            let button = match g_button {
                                GMouseButton::Left => Some(MouseButton::Left),
                                GMouseButton::Right => Some(MouseButton::Right),
                                GMouseButton::Middle => Some(MouseButton::Middle),
                                GMouseButton::Other(1) => Some(MouseButton::X1),
                                GMouseButton::Other(2) => Some(MouseButton::X2),
                                GMouseButton::Other(_) => None
                            };
                            button.map(|b| match state {
                                ElementState::Pressed => WindowEvent::MouseDown(b),
                                ElementState::Released => WindowEvent::MouseUp(b)
                            })
                        }
                        GWindowEvent::Resized(width, height) => Some(WindowEvent::WindowResize(DimsBox::new2(width, height))),
                        GWindowEvent::ReceivedCharacter(c) => Some(WindowEvent::Char(c)),
                        GWindowEvent::KeyboardInput{ input, .. } => {
                            if let Some(key) = input.virtual_keycode.and_then(map_key) {
                                match input.state {
                                    ElementState::Pressed => Some(WindowEvent::KeyDown(key)),
                                    ElementState::Released => Some(WindowEvent::KeyUp(key))
                                }
                            } else {
                                None
                            }
                        }
                        GWindowEvent::Closed => return ControlFlow::Break,
                        _ => None
                    };

                    if let Some(derin_event) = derin_event_opt {
                        match for_each_event(derin_event) {
                            LoopFlow::Break(b) => {
                                ret = Some(b);
                                return ControlFlow::Break;
                            },
                            LoopFlow::Continue => ()
                        }
                    }
                },
                Event::Awakened |
                Event::Suspended(..) |
                Event::DeviceEvent{..} => ()
            }

            ControlFlow::Continue
        });

        ret
    }, |_, _, _| {LoopFlow::Continue}, &mut renderer);
}

fn map_key(k: VirtualKeyCode) -> Option<Key> {
    match k {
        VirtualKeyCode::Back => Some(Key::Back),
        VirtualKeyCode::Tab => Some(Key::Tab),
        // VirtualKeyCode::Clear => Some(Key::Clear),
        VirtualKeyCode::Return => Some(Key::Enter),
        // VirtualKeyCode::Pause => Some(Key::Pause),
        VirtualKeyCode::Escape => Some(Key::Escape),
        VirtualKeyCode::Space => Some(Key::Space),
        VirtualKeyCode::PageUp => Some(Key::PageUp),
        VirtualKeyCode::PageDown => Some(Key::PageDown),
        VirtualKeyCode::End => Some(Key::End),
        VirtualKeyCode::Home => Some(Key::Home),
        // VirtualKeyCode::Select => Some(Key::Select),
        // VirtualKeyCode::Print => Some(Key::Print),
        // VirtualKeyCode::Execute => Some(Key::Execute),
        VirtualKeyCode::Snapshot => Some(Key::PrntScr),
        VirtualKeyCode::Insert => Some(Key::Insert),
        VirtualKeyCode::Delete => Some(Key::Delete),
        // VirtualKeyCode::Help => Some(Key::Help),
        VirtualKeyCode::Key0 => Some(Key::Key0),
        VirtualKeyCode::Key1 => Some(Key::Key1),
        VirtualKeyCode::Key2 => Some(Key::Key2),
        VirtualKeyCode::Key3 => Some(Key::Key3),
        VirtualKeyCode::Key4 => Some(Key::Key4),
        VirtualKeyCode::Key5 => Some(Key::Key5),
        VirtualKeyCode::Key6 => Some(Key::Key6),
        VirtualKeyCode::Key7 => Some(Key::Key7),
        VirtualKeyCode::Key8 => Some(Key::Key8),
        VirtualKeyCode::Key9 => Some(Key::Key9),
        VirtualKeyCode::A => Some(Key::A),
        VirtualKeyCode::B => Some(Key::B),
        VirtualKeyCode::C => Some(Key::C),
        VirtualKeyCode::D => Some(Key::D),
        VirtualKeyCode::E => Some(Key::E),
        VirtualKeyCode::F => Some(Key::F),
        VirtualKeyCode::G => Some(Key::G),
        VirtualKeyCode::H => Some(Key::H),
        VirtualKeyCode::I => Some(Key::I),
        VirtualKeyCode::J => Some(Key::J),
        VirtualKeyCode::K => Some(Key::K),
        VirtualKeyCode::L => Some(Key::L),
        VirtualKeyCode::M => Some(Key::M),
        VirtualKeyCode::N => Some(Key::N),
        VirtualKeyCode::O => Some(Key::O),
        VirtualKeyCode::P => Some(Key::P),
        VirtualKeyCode::Q => Some(Key::Q),
        VirtualKeyCode::R => Some(Key::R),
        VirtualKeyCode::S => Some(Key::S),
        VirtualKeyCode::T => Some(Key::T),
        VirtualKeyCode::U => Some(Key::U),
        VirtualKeyCode::V => Some(Key::V),
        VirtualKeyCode::W => Some(Key::W),
        VirtualKeyCode::X => Some(Key::X),
        VirtualKeyCode::Y => Some(Key::Y),
        VirtualKeyCode::Z => Some(Key::Z),
        VirtualKeyCode::Semicolon => Some(Key::Semicolon),
        VirtualKeyCode::Equals => Some(Key::Equals),
        VirtualKeyCode::Comma => Some(Key::Comma),
        VirtualKeyCode::Minus => Some(Key::Minus),
        VirtualKeyCode::Period => Some(Key::Period),
        VirtualKeyCode::Slash => Some(Key::Slash),
        VirtualKeyCode::Grave => Some(Key::Accent),
        VirtualKeyCode::LBracket => Some(Key::LBracket),
        VirtualKeyCode::RBracket => Some(Key::RBracket),
        VirtualKeyCode::Backslash => Some(Key::Backslash),
        VirtualKeyCode::Apostrophe => Some(Key::Apostrophe),
        VirtualKeyCode::Sleep => Some(Key::Sleep),
        VirtualKeyCode::Numpad0 => Some(Key::Num0),
        VirtualKeyCode::Numpad1 => Some(Key::Num1),
        VirtualKeyCode::Numpad2 => Some(Key::Num2),
        VirtualKeyCode::Numpad3 => Some(Key::Num3),
        VirtualKeyCode::Numpad4 => Some(Key::Num4),
        VirtualKeyCode::Numpad5 => Some(Key::Num5),
        VirtualKeyCode::Numpad6 => Some(Key::Num6),
        VirtualKeyCode::Numpad7 => Some(Key::Num7),
        VirtualKeyCode::Numpad8 => Some(Key::Num8),
        VirtualKeyCode::Numpad9 => Some(Key::Num9),
        VirtualKeyCode::Multiply => Some(Key::NumStar),
        VirtualKeyCode::Add => Some(Key::NumPlus),
        VirtualKeyCode::Subtract => Some(Key::NumSub),
        VirtualKeyCode::Decimal => Some(Key::NumDot),
        VirtualKeyCode::Divide => Some(Key::NumSlash),
        VirtualKeyCode::F1 => Some(Key::F1),
        VirtualKeyCode::F2 => Some(Key::F2),
        VirtualKeyCode::F3 => Some(Key::F3),
        VirtualKeyCode::F4 => Some(Key::F4),
        VirtualKeyCode::F5 => Some(Key::F5),
        VirtualKeyCode::F6 => Some(Key::F6),
        VirtualKeyCode::F7 => Some(Key::F7),
        VirtualKeyCode::F8 => Some(Key::F8),
        VirtualKeyCode::F9 => Some(Key::F9),
        VirtualKeyCode::F10 => Some(Key::F10),
        VirtualKeyCode::F11 => Some(Key::F11),
        VirtualKeyCode::F12 => Some(Key::F12),
        VirtualKeyCode::F13 => Some(Key::F13),
        VirtualKeyCode::F14 => Some(Key::F14),
        VirtualKeyCode::F15 => Some(Key::F15),
        // VirtualKeyCode::F16 => Some(Key::F16),
        // VirtualKeyCode::F17 => Some(Key::F17),
        // VirtualKeyCode::F18 => Some(Key::F18),
        // VirtualKeyCode::F19 => Some(Key::F19),
        // VirtualKeyCode::F20 => Some(Key::F20),
        // VirtualKeyCode::F21 => Some(Key::F21),
        // VirtualKeyCode::F22 => Some(Key::F22),
        // VirtualKeyCode::F23 => Some(Key::F23),
        // VirtualKeyCode::F24 => Some(Key::F24),
        VirtualKeyCode::Numlock => Some(Key::NumLock),
        // VirtualKeyCode::Caps => Some(Key::Caps),
        VirtualKeyCode::Scroll => Some(Key::ScrollLock),
        VirtualKeyCode::LShift => Some(Key::LShift),
        VirtualKeyCode::RShift => Some(Key::RShift),
        VirtualKeyCode::LControl => Some(Key::LCtrl),
        VirtualKeyCode::RControl => Some(Key::RCtrl),
        VirtualKeyCode::LAlt => Some(Key::LAlt),
        VirtualKeyCode::RAlt => Some(Key::RAlt),
        VirtualKeyCode::NavigateBackward => Some(Key::BrowserBack),
        VirtualKeyCode::NavigateForward => Some(Key::BrowserFwd),
        VirtualKeyCode::WebRefresh => Some(Key::BrowserRef),
        VirtualKeyCode::WebStop => Some(Key::BrowserStop),
        VirtualKeyCode::WebSearch => Some(Key::BrowserSearch),
        VirtualKeyCode::WebFavorites => Some(Key::BrowserFav),
        VirtualKeyCode::WebHome => Some(Key::BrowserHome),
        VirtualKeyCode::NextTrack => Some(Key::MediaNextTrack),
        VirtualKeyCode::PrevTrack => Some(Key::MediaPrevTrack),
        VirtualKeyCode::Stop => Some(Key::MediaStop),
        VirtualKeyCode::Pause => Some(Key::Pause),
        VirtualKeyCode::Left => Some(Key::LArrow),
        VirtualKeyCode::Up => Some(Key::UArrow),
        VirtualKeyCode::Right => Some(Key::RArrow),
        VirtualKeyCode::Down => Some(Key::DArrow),
        VirtualKeyCode::Kana => Some(Key::Kana),
        // VirtualKeyCode::Junja => Some(Key::Junja),
        // VirtualKeyCode::Final => Some(Key::Final),
        VirtualKeyCode::Kanji => Some(Key::Kanji),
        VirtualKeyCode::Convert => Some(Key::Convert),
        // VirtualKeyCode::Nonconvert => Some(Key::Nonconvert),
        // VirtualKeyCode::Accept => Some(Key::Accept),
        // VirtualKeyCode::ModeChange => Some(Key::ModeChange),
        // VirtualKeyCode::Process => Some(Key::Process),
        // VirtualKeyCode::LShift => Some(Key::Shift),
        // VirtualKeyCode::Control => Some(Key::Control),
        // VirtualKeyCode::Menu => Some(Key::Menu),
        VirtualKeyCode::Compose |
        VirtualKeyCode::AbntC1 |
        VirtualKeyCode::AbntC2 |
        VirtualKeyCode::Apps |
        VirtualKeyCode::At |
        VirtualKeyCode::Ax |
        VirtualKeyCode::Calculator |
        VirtualKeyCode::Capital |
        VirtualKeyCode::Colon |
        VirtualKeyCode::LMenu |
        VirtualKeyCode::LWin |
        VirtualKeyCode::Mail |
        VirtualKeyCode::MediaSelect |
        VirtualKeyCode::MediaStop |
        VirtualKeyCode::Mute |
        VirtualKeyCode::MyComputer |
        VirtualKeyCode::NoConvert |
        VirtualKeyCode::NumpadComma |
        VirtualKeyCode::NumpadEnter |
        VirtualKeyCode::NumpadEquals |
        VirtualKeyCode::OEM102 |
        VirtualKeyCode::PlayPause |
        VirtualKeyCode::Power |
        VirtualKeyCode::RMenu |
        VirtualKeyCode::RWin |
        VirtualKeyCode::Sysrq |
        VirtualKeyCode::Underline |
        VirtualKeyCode::Unlabeled |
        VirtualKeyCode::VolumeDown |
        VirtualKeyCode::VolumeUp |
        VirtualKeyCode::Wake |
        VirtualKeyCode::WebBack |
        VirtualKeyCode::WebForward |
        VirtualKeyCode::Yen => None
    }
}
