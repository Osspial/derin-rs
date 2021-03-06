// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

extern crate derin;
#[macro_use]
extern crate derin_macros;

use derin::{Window, WindowConfig};
use derin::layout::{Margins, LayoutHorizontal, LayoutVertical};
use derin::widgets::*;
use derin::geometry::rect::DimsBox;

#[derive(Debug, Clone, Copy, PartialEq)]
enum GalleryEvent {
    NewButton,
    Checked,
    SliderMove(f32)
}

#[derive(WidgetContainer)]
struct BasicContainer {
    button: Button<Option<GalleryEvent>>,
    nested: ScrollBox<Group<NestedContainer, LayoutVertical>>,
    tabs: TabList<Button<Option<GalleryEvent>>>
}

#[derive(WidgetContainer)]
struct NestedContainer {
    label: Label,
    edit_box: LineBox,
    progress_bar: ProgressBar,
    slider: Slider<SliderH>,
    check_box: CheckBox<Option<GalleryEvent>>,
    radio_buttons: RadioButtonList<Vec<RadioButton>, LayoutVertical>,
    #[derin(collection = "Button<Option<GalleryEvent>>")]
    buttons: Vec<Button<Option<GalleryEvent>>>
}

struct SliderH;
impl SliderHandler for SliderH {
    type Action = GalleryEvent;
    fn on_move(&mut self, _: f32, new_value: f32) -> Option<GalleryEvent> {
        Some(GalleryEvent::SliderMove(new_value))
    }
}

fn main() {
    let group = Group::new(
        BasicContainer {
            button: Button::new(Contents::Text("New Button".to_string()), Some(GalleryEvent::NewButton)),
            nested: ScrollBox::new(Group::new(
                NestedContainer {
                    label: Label::new(Contents::Text("Nested Container".to_string())),
                    slider: Slider::new(0.0, 1.0, 0.0, 100.0, SliderH),
                    progress_bar: ProgressBar::new(0.0, 0.0, 100.0),
                    check_box: CheckBox::new(true, Contents::Text("Checkable".to_string()), Some(GalleryEvent::Checked)),
                    radio_buttons: RadioButtonList::new(
                        vec![
                            RadioButton::new(true, Contents::Text("Radio 1".to_string())),
                            RadioButton::new(false, Contents::Text("Radio 2".to_string()))
                        ],
                        LayoutVertical::new(Margins::new(0, 2, 0, 8), Default::default())
                    ),
                    edit_box: LineBox::new("Edit Me!".to_string()),
                    buttons: Vec::new(),
                },
                LayoutVertical::new(Margins::new(8, 8, 8, 8), Default::default())
            )),
            tabs: TabList::new(vec![
                TabPage::new("Tab 1".to_string(), Button::new(Contents::Text("Tab 1".to_string()), None)),
                TabPage::new("Tab No.2".to_string(), Button::new(Contents::Text("Tab 2".to_string()), None)),
            ])
        },
        LayoutHorizontal::new(Margins::new(8, 8, 8, 8), Default::default())
    );
    let theme = derin::theme::Theme::default();

    let window_config = WindowConfig {
        dimensions: Some(DimsBox::new2(512, 512)),
        title: "Derin Control Gallery".to_string(),
        ..WindowConfig::default()
    };

    let mut window = unsafe{ Window::new(window_config, group, theme).unwrap() };
    window.run_forever();
}
