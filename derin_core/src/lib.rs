// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#![feature(nll, specialization, try_blocks, never_type)]
#![feature(core_intrinsics)] // this is used for `type_name`. Should be removed when that's stabilized

use cgmath_geometry::cgmath;
#[macro_use]
extern crate derin_common_types;

#[macro_use]
mod macros;

#[cfg(test)]
#[macro_use]
pub mod test_helpers;

pub mod timer;
#[macro_use]
pub mod event;
pub mod render;
pub mod widget;

mod mbseq;
mod offset_widget;
mod message_bus;
mod event_translator;
mod update_state;
mod widget_traverser;

use crate::cgmath::{Point2, Vector2, Bounded, EuclideanSpace};
use cgmath_geometry::{D2, rect::{DimsBox, BoundBox, GeoBox}};

use crate::{
    message_bus::{MessageBus, MessageTarget},
    event::{WidgetEvent, WidgetEventSourced},
    event_translator::EventTranslator,
    timer::{TimerTrigger, TimerTriggerTracker},
    widget::*,
    render::{DisplayEngine},
    mbseq::MouseButtonSequenceTrackPos,
    update_state::{UpdateState, UpdateStateCell},
    widget_traverser::{Relation, WidgetPath, WidgetTraverser, WidgetTraverserBase},
};
use derin_common_types::{
    buttons::{MouseButton, Key, ModifierKeys},
    cursor::CursorIcon,
    layout::SizeBounds,
};
use std::{
    rc::Rc,
    time::Instant,
};

const MAX_FRAME_UPDATE_ITERATIONS: usize = 256;

fn find_index<T: PartialEq>(s: &[T], element: &T) -> Option<usize> {
    s.iter().enumerate().find(|&(_, e)| e == element).map(|(i, _)| i)
}

fn vec_remove_element<T: PartialEq>(v: &mut Vec<T>, element: &T) -> Option<T> {
    find_index(v, element).map(|i| v.remove(i))
}

pub struct Root<N, D>
    where N: Widget + 'static,
          D: DisplayEngine
{
    // Event handing and dispatch
    event_translator: EventTranslator,

    // Input State
    input_state: InputState,

    widget_traverser_base: WidgetTraverserBase<D>,

    timer_tracker: TimerTriggerTracker,
    message_bus: MessageBus,
    update_state: Rc<UpdateStateCell>,

    // User data
    pub root_widget: N,
    pub display_engine: D,
}

struct InputState {
    mouse_pos: Option<Point2<i32>>,
    mouse_buttons_down: MouseButtonSequenceTrackPos,
    modifiers: ModifierKeys,
    keys_down: Vec<Key>,
    mouse_hover_widget: Option<WidgetId>,
    focused_widget: Option<WidgetId>
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowEvent {
    MouseMove(Point2<i32>),
    MouseEnter,
    MouseExit,
    MouseDown(MouseButton),
    MouseUp(MouseButton),
    MouseScrollLines(Vector2<i32>),
    MouseScrollPx(Vector2<i32>),
    WindowResize(DimsBox<D2, u32>),
    KeyDown(Key),
    KeyUp(Key),
    Char(char),
    Timer,
    Redraw
}

/// Whether to continue or abort a loop.
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopFlow {
    /// Continue the loop.
    Continue,
    /// Abort the loop.
    Break
}

#[must_use]
pub struct FrameEventProcessor<'a, D>
    where D: DisplayEngine + 'static
{
    input_state: &'a mut InputState,
    event_translator: &'a mut EventTranslator,
    timer_tracker: &'a mut TimerTriggerTracker,
    message_bus: &'a mut MessageBus,
    update_state: Rc<UpdateStateCell>,
    widget_traverser: WidgetTraverser<'a, D>,
}

#[must_use]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventLoopResult {
    pub next_timer: Option<Instant>,
    pub set_cursor_pos: Option<Point2<i32>>,
    pub set_cursor_icon: Option<CursorIcon>,
}

impl InputState {
    fn new() -> InputState {
        InputState {
            mouse_pos: None,
            mouse_buttons_down: MouseButtonSequenceTrackPos::new(),
            modifiers: ModifierKeys::empty(),
            keys_down: Vec::new(),
            mouse_hover_widget: None,
            focused_widget: None
        }
    }
}

impl<N, D> Root<N, D>
    where N: Widget,
          D: DisplayEngine
{
    #[inline]
    pub fn new(mut root_widget: N, display_engine: D, dims: DimsBox<D2, u32>) -> Root<N, D> {
        // TODO: DRAW ROOT AND DO INITIAL LAYOUT
        *root_widget.rect_mut() = dims.cast().unwrap_or(DimsBox::max_value()).into();
        let message_bus = MessageBus::new();
        Root {
            event_translator: EventTranslator::new(),

            input_state: InputState::new(),

            widget_traverser_base: WidgetTraverserBase::new(root_widget.widget_id()),

            timer_tracker: TimerTriggerTracker::new(),
            update_state: UpdateState::new(&message_bus),
            message_bus,

            root_widget, display_engine,
        }
    }

    pub fn start_frame(&mut self) -> FrameEventProcessor<'_, D> {
        FrameEventProcessor {
            input_state: &mut self.input_state,
            event_translator: &mut self.event_translator,
            timer_tracker: &mut self.timer_tracker,
            message_bus: &mut self.message_bus,
            update_state: self.update_state.clone(),
            widget_traverser: self.widget_traverser_base.with_root_ref(&mut self.root_widget, self.update_state.clone())
        }
    }

    pub fn relayout(&mut self) -> SizeBounds {
        let mut widget_traverser = self.widget_traverser_base.with_root_ref(&mut self.root_widget, self.update_state.clone());

        let mut relayout_widgets = Vec::new();

        let mut iter_num = 0;
        let global_update = self.update_state.borrow().global_update;

        while global_update || self.update_state.borrow().relayout.len() > 0 {
            match global_update {
                false => relayout_widgets.extend(self.update_state.borrow_mut().relayout.drain()),
                true => {
                    self.update_state.borrow_mut().relayout.clear();
                    relayout_widgets.extend(widget_traverser.all_widgets());
                }
            }

            let valid_len = widget_traverser.sort_widgets_by_depth(&mut relayout_widgets).len();
            relayout_widgets.truncate(valid_len);

            for i in 0..valid_len {
                let widget_id = relayout_widgets[i];

                let WidgetPath{mut widget, ..} = match widget_traverser.get_widget(widget_id) {
                    Some(widget) => widget,
                    None => continue
                };

                let old_widget_rect = widget.rect();
                widget.update_layout(self.display_engine.layout(widget_id, old_widget_rect.dims()));
                let size_bounds = widget.size_bounds();
                let new_widget_rect = widget.rect();
                let widget_dims = new_widget_rect.dims();
                widget.cancel_scan();

                let dims_bounded = size_bounds.bound_rect(widget_dims);

                // If we're doing a global update, all widgets are in the relayout list so we don't
                // need to queue the parent for relayout. Otherwise, queue the parent for relayout if
                // the widget's rect has changed or the widget's dimensions no longer fall in its size
                // bounds.
                let parent_needs_relayout =
                    dims_bounded != widget_dims ||
                    old_widget_rect != new_widget_rect;

                if !global_update && parent_needs_relayout {
                    drop(widget);
                    if let Some(WidgetPath{widget_id: parent_id, ..}) = widget_traverser.get_widget_relation(widget_id, Relation::Parent) {
                        if !relayout_widgets.contains(&parent_id) {
                            relayout_widgets.push(parent_id);
                        }
                        continue;
                    } /*else*/ { // Ideally this would be an else block but lifetimes.
                        // If there's no parent, we must be on the root widget. So, just resize the
                        // widget to what it expects.
                        let mut widget = widget_traverser.get_widget(widget_id).unwrap().widget;
                        widget.set_rect(dims_bounded.into());
                        widget.cancel_scan();
                    }
                }
            }

            // Remove all re-layed-out widgets from the list.
            relayout_widgets.drain(..valid_len);

            if global_update {
                break;
            }

            iter_num += 1;
            if iter_num > MAX_FRAME_UPDATE_ITERATIONS {
                // TODO: CHANGE TO LOG WARN
                println!("WARNING: layout iterations happened unreasonable number of times");
                break;
            }
        }

        let root_id = widget_traverser.root_id();
        let root_widget = widget_traverser.get_widget(root_id).unwrap().widget;
        root_widget.size_bounds()
    }

    pub fn redraw(&mut self) {
        let root_rect = self.root_widget.rect();
        let new_dims = root_rect.dims().cast::<u32>().unwrap_or(DimsBox::new2(0, 0));
        if new_dims != self.display_engine.dims() {
            self.display_engine.resized(new_dims);
        }

        let Root {
            ref update_state,
            ref mut widget_traverser_base,
            ref mut root_widget,
            ref mut display_engine,
            ..
        } = *self;

        let mut update_state_ref = update_state.borrow_mut();
        if update_state_ref.global_update || update_state_ref.redraw.len() > 0 {
            // We should probably support incremental redraw at some point but not doing that is
            // soooo much easier.
            update_state_ref.redraw.clear();
            update_state_ref.reset_global_update();
            drop(update_state_ref);

            display_engine.start_frame();
            let window_rect = display_engine.dims();
            let window_rect = BoundBox::new2(0, 0, window_rect.width() as i32, window_rect.height() as i32);

            let mut widget_traverser = widget_traverser_base.with_root_ref(root_widget, update_state.clone());
            widget_traverser.crawl_widgets(|mut path| {
                let renderer = display_engine.render(
                    path.widget.widget_id(),
                    path.widget.rect(),
                    path.widget.clip().unwrap_or(window_rect),
                );

                path.widget.render(renderer);
            });
            display_engine.finish_frame();
        }
    }
}

impl<D> FrameEventProcessor<'_, D>
    where D: DisplayEngine
{
    pub fn process_event(
        &mut self,
        event: WindowEvent,
    ) {
        let FrameEventProcessor {
            ref mut input_state,
            ref mut event_translator,
            ref update_state,
            ref mut widget_traverser,
            timer_tracker: _,
            message_bus: _,
        } = *self;

        event_translator
            .with_data(
                widget_traverser,
                input_state,
                update_state.clone(),
            )
            .translate_window_event(event);
    }

    pub fn set_modifiers(&mut self, modifiers: ModifierKeys) {
        self.input_state.modifiers = modifiers;
    }

    pub fn finish(mut self) -> EventLoopResult {
        {
            let mut update_state = self.update_state.borrow_mut();

            for remove_id in update_state.remove_from_tree.drain() {
                self.widget_traverser.remove_widget(remove_id);
                self.message_bus.remove_widget(remove_id);
            }

            for widget_id in update_state.update_timers.drain() {
                let widget = match self.widget_traverser.get_widget(widget_id) {
                    Some(wpath) => wpath.widget,
                    None => continue
                };

                for (&timer_id, timer) in &widget.widget_tag().timers {
                    let trigger_time = timer.next_trigger();
                    let trigger = TimerTrigger::new(trigger_time, timer_id, widget_id);
                    self.timer_tracker.queue_trigger(trigger);
                }
            }

            for widget_id in update_state.update_messages.drain() {
                let widget = match self.widget_traverser.get_widget(widget_id) {
                    Some(wpath) => wpath.widget,
                    None => continue
                };

                let widget_tag = widget.widget_tag();
                for message_type in widget_tag.message_types() {
                    self.message_bus.register_widget_message_type(message_type, widget_tag.widget_id);
                }
            }
        }

        while let Some((message, widgets)) = self.message_bus.next_message() {
            for message_target in widgets {
                match message_target {
                    MessageTarget::Widget(widget_id) => {
                        match self.widget_traverser.get_widget(widget_id) {
                            Some(mut wpath) => wpath.widget.inner_mut().dispatch_message(&*message),
                            None => continue
                        }
                    },
                    MessageTarget::ParentOf(widget_id) => {
                        match self.widget_traverser.get_widget_relation(widget_id, Relation::Parent) {
                            Some(mut wpath) => wpath.widget.inner_mut().dispatch_message(&*message),
                            None => continue
                        }
                    },
                    MessageTarget::ChildrenOf(widget_id) => {
                        self.widget_traverser.crawl_widget_children(widget_id, |mut wpath| {
                            wpath.widget.inner_mut().dispatch_message(&*message)
                        });
                    }
                }
            }
        }

        // Send timer events
        let timers_triggered = self.timer_tracker.timers_triggered().collect::<Vec<_>>();
        for timer_trigger in timers_triggered {let _: Option<_> = try {
            let mut widget = self.widget_traverser.get_widget(timer_trigger.widget_id)?.widget;

            // Dispatch the widget event.
            let timer = widget.widget_tag().timers.get(&timer_trigger.timer_id)?;
            let event = WidgetEvent::Timer {
                timer_id: timer_trigger.timer_id,
                start_time: timer.start_time(),
                last_triggered: timer.last_triggered(),
                frequency: timer.frequency,
                times_triggered: timer.times_triggered(),
            };
            let trigger_time = Instant::now();
            // TODO: HANDLE OPS
            widget.on_widget_event(WidgetEventSourced::This(event), self.input_state);


            // Update the timer's internal info values.
            let timer = widget.widget_tag().timers.get(&timer_trigger.timer_id)?;
            timer.times_triggered.set(timer.times_triggered.get() + 1);
            timer.last_triggered.set(Some(trigger_time));

            // Queue the next timer trigger.
            self.timer_tracker.queue_trigger(TimerTrigger {
                instant: timer.next_trigger(),
                ..timer_trigger
            });
        };}

        let mut update_state = self.update_state.borrow_mut();
        let widget_traverser = &mut self.widget_traverser;
        let set_cursor_icon = update_state.set_cursor_icon.take();

        // The cursor position stored in `UpdateState.set_cursor_pos` is relative to the requesting
        // widget's origin. This translates it into window-space.
        let set_cursor_pos = update_state.set_cursor_pos.take()
            .and_then(|(widget_id, offset_pos)|
                widget_traverser.get_widget(widget_id)
                    .map(|wpath| wpath.widget.rect().min + offset_pos.to_vec())
            );


        EventLoopResult {
            next_timer: self.timer_tracker.next_trigger(),
            set_cursor_pos,
            set_cursor_icon,
        }
    }
}
