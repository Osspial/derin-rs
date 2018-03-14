//! This module is Rust at its ugliest.

use LoopFlow;
use render::RenderFrame;
use tree::{Parent, WidgetIdent, WidgetSummary, Widget, OnFocusOverflow};

use arrayvec::ArrayVec;
use std::mem;

const CHILD_BATCH_SIZE: usize = 24;

pub type ForEachSummary<'a, W> = &'a mut FnMut(ArrayVec<[WidgetSummary<W>; CHILD_BATCH_SIZE]>) -> LoopFlow<()>;
pub trait ParentDyn<A, F: RenderFrame>: Widget<A, F> {
    fn as_widget(&mut self) -> &mut Widget<A, F>;
    fn num_children(&self) -> usize;

    fn child(&self, widget_ident: WidgetIdent) -> Option<WidgetSummary<&Widget<A, F>>>;
    fn child_mut(&mut self, widget_ident: WidgetIdent) -> Option<WidgetSummary<&mut Widget<A, F>>>;

    fn child_by_index(&self, index: usize) -> Option<WidgetSummary<&Widget<A, F>>>;
    fn child_by_index_mut(&mut self, index: usize) -> Option<WidgetSummary<&mut Widget<A, F>>>;

    fn children<'a>(&'a self, for_each: ForEachSummary<&'a Widget<A, F>>);
    fn children_mut<'a>(&'a mut self, for_each: ForEachSummary<&'a mut Widget<A, F>>);

    fn update_child_layout(&mut self);

    fn on_child_focus_overflow(&self) -> OnFocusOverflow;
}

impl<A, F, P> ParentDyn<A, F> for P
    where F: RenderFrame,
          P: Parent<A, F>
{
    fn as_widget(&mut self) -> &mut Widget<A, F> {
        self as &mut Widget<A, F>
    }
    fn num_children(&self) -> usize {
        <Self as Parent<A, F>>::num_children(self)
    }

    fn child(&self, widget_ident: WidgetIdent) -> Option<WidgetSummary<&Widget<A, F>>> {
        <Self as Parent<A, F>>::child(self, widget_ident)
    }
    fn child_mut(&mut self, widget_ident: WidgetIdent) -> Option<WidgetSummary<&mut Widget<A, F>>> {
        <Self as Parent<A, F>>::child_mut(self, widget_ident)
    }

    fn child_by_index(&self, index: usize) -> Option<WidgetSummary<&Widget<A, F>>> {
        <Self as Parent<A, F>>::child_by_index(self, index)
    }
    fn child_by_index_mut(&mut self, index: usize) -> Option<WidgetSummary<&mut Widget<A, F>>> {
        <Self as Parent<A, F>>::child_by_index_mut(self, index)
    }

    fn children<'a>(&'a self, for_each: ForEachSummary<&'a Widget<A, F>>) {
        let mut child_avec: ArrayVec<[_; CHILD_BATCH_SIZE]> = ArrayVec::new();

        <Self as Parent<A, F>>::children::<_, ()>(self, |summary| {
            match child_avec.try_push(summary) {
                Ok(()) => (),
                Err(caperr) => {
                    let full_avec = mem::replace(&mut child_avec, ArrayVec::new());
                    match for_each(full_avec) {
                        LoopFlow::Break(_) => return LoopFlow::Break(()),
                        LoopFlow::Continue => ()
                    }
                    child_avec.push(caperr.element());
                }
            }

            LoopFlow::Continue
        });

        if child_avec.len() != 0 {
            let _ = for_each(child_avec);
        }
    }
    fn children_mut<'a>(&'a mut self, for_each: ForEachSummary<&'a mut Widget<A, F>>) {
        let mut child_avec: ArrayVec<[_; CHILD_BATCH_SIZE]> = ArrayVec::new();

        <Self as Parent<A, F>>::children_mut::<_, ()>(self, |summary| {
            match child_avec.try_push(summary) {
                Ok(()) => (),
                Err(caperr) => {
                    let full_avec = mem::replace(&mut child_avec, ArrayVec::new());
                    match for_each(full_avec) {
                        LoopFlow::Break(_) => return LoopFlow::Break(()),
                        LoopFlow::Continue => ()
                    }
                    child_avec.push(caperr.element());
                }
            }

            LoopFlow::Continue
        });

        if child_avec.len() != 0 {
            let _ = for_each(child_avec);
        }
    }

    fn update_child_layout(&mut self) {
        <Self as Parent<A, F>>::update_child_layout(self)
    }

    fn on_child_focus_overflow(&self) -> OnFocusOverflow {
        <Self as Parent<A, F>>::on_child_focus_overflow(self)
    }
}

impl<A, F: RenderFrame> ParentDyn<A, F> {
    #[inline]
    pub fn from_widget<W>(widget: &W) -> Option<&ParentDyn<A, F>>
        where W: Widget<A, F> + ?Sized
    {
        trait AsParent<A, F>
            where F: RenderFrame
        {
            fn as_parent_dyn(&self) -> Option<&ParentDyn<A, F>>;
        }
        impl<A, F, W> AsParent<A, F> for W
            where F: RenderFrame,
                  W: Widget<A, F> + ?Sized
        {
            #[inline(always)]
            default fn as_parent_dyn(&self) -> Option<&ParentDyn<A, F>> {
                None
            }
        }
        impl<A, F, W> AsParent<A, F> for W
            where F: RenderFrame,
                  W: ParentDyn<A, F>
        {
            #[inline(always)]
            fn as_parent_dyn(&self) -> Option<&ParentDyn<A, F>> {
                Some(self)
            }
        }

        widget.as_parent_dyn()
    }

    #[inline]
    pub fn from_widget_mut<W>(widget: &mut W) -> Option<&mut ParentDyn<A, F>>
        where W: Widget<A, F> + ?Sized
    {
        trait AsParent<A, F>
            where F: RenderFrame
        {
            fn as_parent_dyn(&mut self) -> Option<&mut ParentDyn<A, F>>;
        }
        impl<A, F, W> AsParent<A, F> for W
            where F: RenderFrame,
                  W: Widget<A, F> + ?Sized
        {
            #[inline(always)]
            default fn as_parent_dyn(&mut self) -> Option<&mut ParentDyn<A, F>> {
                None
            }
        }
        impl<A, F, W> AsParent<A, F> for W
            where F: RenderFrame,
                  W: ParentDyn<A, F>
        {
            #[inline(always)]
            fn as_parent_dyn(&mut self) -> Option<&mut ParentDyn<A, F>> {
                Some(self)
            }
        }

        widget.as_parent_dyn()
    }
}
