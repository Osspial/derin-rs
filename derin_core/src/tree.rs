use std::cell::Cell;

use LoopFlow;
use cgmath::Point2;
use cgmath_geometry::BoundBox;

use mbseq::MouseButtonSequence;
use dct::buttons::MouseButton;
use event::{NodeEvent, EventOps};
use render::{RenderFrame, FrameRectStack};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeIdent {
    Str(&'static str),
    Num(u32),
    StrCollection(&'static str, u32),
    NumCollection(u32, u32)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Update {
    pub render_self: bool,
    pub update_child: bool,
    pub update_layout: bool
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum MouseState {
    /// The mouse is hovering over the given node.
    Hovering(Point2<i32>, MouseButtonSequence),
    /// The mouse isn't hovering over the node, but the node is still receiving mouse events.
    Tracking(Point2<i32>, MouseButtonSequence),
    /// The node is not aware of the current mouse position and is receiving no events.
    Untracked
}

#[derive(Debug, Clone)]
pub struct UpdateTag {
    last_root: Cell<u32>,
    pub(crate) last_event_stamp: Cell<u32>,
    pub(crate) mouse_state: Cell<MouseState>,
    pub(crate) child_event_recv: Cell<ChildEventRecv>
}

impl MouseState {
    #[inline]
    pub fn mouse_button_sequence(&self) -> MouseButtonSequence {
        match *self {
            MouseState::Untracked => MouseButtonSequence::new(),
            MouseState::Hovering(_, mbseq) |
            MouseState::Tracking(_, mbseq) => mbseq
        }
    }
}

bitflags! {
    #[doc(hidden)]
    pub struct ChildEventRecv: u8 {
        const MOUSE_L     = 1 << 0;
        const MOUSE_R     = 1 << 1;
        const MOUSE_M     = 1 << 2;
        const MOUSE_X1    = 1 << 3;
        const MOUSE_X2    = 1 << 4;
        const MOUSE_HOVER = 1 << 5;
        // const KEYS        = 1 << 6;

        const MOUSE_BUTTONS =
            Self::MOUSE_L.bits  |
            Self::MOUSE_R.bits  |
            Self::MOUSE_M.bits  |
            Self::MOUSE_X1.bits |
            Self::MOUSE_X2.bits;
    }
}

impl ChildEventRecv {
    #[inline]
    pub(crate) fn mouse_button_mask(button: MouseButton) -> ChildEventRecv {
        ChildEventRecv::from_bits_truncate(1 << (u8::from(button) - 1))
    }
}

impl From<MouseButtonSequence> for ChildEventRecv {
    #[inline]
    fn from(mbseq: MouseButtonSequence) -> ChildEventRecv {
        mbseq.into_iter().fold(ChildEventRecv::empty(), |child_event_recv, mb| child_event_recv | ChildEventRecv::mouse_button_mask(mb))
    }
}

impl<'a> From<&'a UpdateTag> for ChildEventRecv {
    #[inline]
    fn from(update_tag: &'a UpdateTag) -> ChildEventRecv {
        let node_mb_flags = ChildEventRecv::from(update_tag.mouse_state.get().mouse_button_sequence());

        node_mb_flags |
        match update_tag.mouse_state.get() {
            MouseState::Hovering(_, _) => ChildEventRecv::MOUSE_HOVER,
            MouseState::Tracking(_, _)  |
            MouseState::Untracked   => ChildEventRecv::empty()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RootID(u32);

macro_rules! subtrait_enums {
    (subtraits {
        $( $Variant:ident($SubTrait:ty) ),+
    }) => {
        pub enum NodeSubtrait<'a, A: 'a, F: 'a + RenderFrame> {
            $( $Variant(&'a $SubTrait) ),+
        }

        pub enum NodeSubtraitMut<'a, A: 'a, F: 'a + RenderFrame> {
            $( $Variant(&'a mut $SubTrait) ),+
        }
    }
}

subtrait_enums! {subtraits {
    Parent(Parent<A, F>),
    Node(Node<A, F>)
}}

pub trait Node<A, F: RenderFrame> {
    fn update_tag(&self) -> &UpdateTag;
    fn bounds(&self) -> BoundBox<Point2<u32>>;
    fn bounds_mut(&mut self) -> &mut BoundBox<Point2<u32>>;
    fn render(&self, frame: &mut FrameRectStack<F>);
    fn on_node_event(&mut self, event: NodeEvent, source_child: &[NodeIdent]) -> EventOps<A>;
    fn subtrait(&self) -> NodeSubtrait<A, F>;
    fn subtrait_mut(&mut self) -> NodeSubtraitMut<A, F>;
}

#[derive(Debug, Clone)]
pub struct NodeSummary<N> {
    pub node: N,
    pub ident: NodeIdent,
    pub rect: BoundBox<Point2<u32>>,
    pub update_tag: UpdateTag
}

pub trait Parent<A, F: RenderFrame>: Node<A, F> {
    fn child(&self, node_ident: NodeIdent) -> Option<NodeSummary<&Node<A, F>>>;
    fn child_mut(&mut self, node_ident: NodeIdent) -> Option<NodeSummary<&mut Node<A, F>>>;

    fn children<'a>(&'a self, for_each: &mut FnMut(&[NodeSummary<&'a Node<A, F>>]) -> LoopFlow<()>);
    fn children_mut<'a>(&'a mut self, for_each: &mut FnMut(&mut [NodeSummary<&'a mut Node<A, F>>]) -> LoopFlow<()>);

    fn update_child_layout(&mut self);

    fn child_by_point(&self, point: Point2<u32>) -> Option<NodeSummary<&Node<A, F>>>;
    fn child_by_point_mut(&mut self, point: Point2<u32>) -> Option<NodeSummary<&mut Node<A, F>>>;
}

const RENDER_SELF: u32 = 1 << 31;
const UPDATE_CHILD: u32 = 1 << 30;
const RENDER_ALL: u32 = RENDER_SELF | UPDATE_CHILD;
const UPDATE_LAYOUT: u32 = (1 << 29);

const UPDATE_MASK: u32 = RENDER_SELF | UPDATE_CHILD | RENDER_ALL | UPDATE_LAYOUT;

impl UpdateTag {
    #[inline]
    pub fn new() -> UpdateTag {
        UpdateTag {
            last_root: Cell::new(UPDATE_MASK),
            last_event_stamp: Cell::new(0),
            mouse_state: Cell::new(MouseState::Untracked),
            child_event_recv: Cell::new(ChildEventRecv::empty())
        }
    }

    #[inline]
    pub fn mark_render_self(&mut self) -> &mut UpdateTag {
        self.last_root.set(self.last_root.get() | RENDER_SELF);
        self
    }

    #[inline]
    pub fn mark_update_child(&mut self) -> &mut UpdateTag {
        self.last_root.set(self.last_root.get() | UPDATE_CHILD);
        self
    }

    #[inline]
    pub fn mark_update_layout(&mut self) -> &mut UpdateTag {
        self.last_root.set(self.last_root.get() | UPDATE_LAYOUT);
        self
    }

    #[inline]
    pub(crate) fn mark_updated(&self, root_id: RootID) {
        self.last_root.set(root_id.0);
    }

    #[inline]
    pub(crate) fn unmark_update_layout(&self) {
        self.last_root.set(self.last_root.get() & !UPDATE_LAYOUT);
    }

    pub(crate) fn mark_update_child_immutable(&self) {
        self.last_root.set(self.last_root.get() | UPDATE_CHILD);
    }

    #[inline]
    pub(crate) fn needs_update(&self, root_id: RootID) -> Update {
        match self.last_root.get() {
            r if r == root_id.0 => Update {
                render_self: false,
                update_child: false,
                update_layout: false
            },
            r => Update {
                render_self: r & UPDATE_MASK & RENDER_SELF != 0,
                update_child: r & UPDATE_MASK & UPDATE_CHILD != 0,
                update_layout: r & UPDATE_MASK & UPDATE_LAYOUT != 0
            },
        }
    }
}

impl RootID {
    #[inline]
    pub fn new() -> RootID {
        use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};

        static ID_COUNTER: AtomicUsize = ATOMIC_USIZE_INIT;
        let id = ID_COUNTER.fetch_add(1, Ordering::SeqCst) as u32;
        assert!(id < UPDATE_MASK);

        RootID(id as u32)
    }
}
