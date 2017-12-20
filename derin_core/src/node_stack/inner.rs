use std::mem;
use render::RenderFrame;
use tree::{Node, NodeIdent};

use cgmath::{EuclideanSpace, Vector2};
use cgmath_geometry::{BoundRect, Rectangle};

// TODO: GET CODE REVIEWED FOR SAFETY

struct StackElement<'a, A, F: RenderFrame> {
    node: *mut (Node<A, F> + 'a),
    bounds: BoundRect<u32>,
    ident: NodeIdent
}

pub struct NRAllocCache<A, F: RenderFrame> {
    vec: Vec<StackElement<'static, A, F>>
}

pub struct NRVec<'a, A: 'a, F: 'a + RenderFrame> {
    cache: &'a mut Vec<StackElement<'static, A, F>>,
    vec: Vec<StackElement<'a, A, F>>,
    top_parent_offset: Vector2<u32>
}

impl<A, F: RenderFrame> NRAllocCache<A, F> {
    pub fn new() -> NRAllocCache<A, F> {
        NRAllocCache {
            vec: vec![]
        }
    }

    pub fn use_cache<'a>(&'a mut self, node: &mut (Node<A, F> + 'a)) -> NRVec<'a, A, F> {
        let mut cache_swap = Vec::new();
        mem::swap(&mut cache_swap, &mut self.vec);

        let mut vec = unsafe {
            let (ptr, len, cap) = (cache_swap.as_ptr(), cache_swap.len(), cache_swap.capacity());
            mem::forget(cache_swap);
            Vec::from_raw_parts(mem::transmute::<_, *mut StackElement<A, F>>(ptr), len, cap)
        };

        vec.push(StackElement {
            node: node,
            bounds: BoundRect::new(0xDEADBEEF, 0xDEADBEEF, 0xDEADBEEF, 0xDEADBEEF),
            ident: NodeIdent::Num(0)
        });

        NRVec {
            cache: &mut self.vec,
            vec,
            top_parent_offset: Vector2::new(0, 0)
        }
    }
}

impl<'a, A, F: RenderFrame> NRVec<'a, A, F> {
    #[inline]
    pub fn top(&self) -> &(Node<A, F> + 'a) {
        self.vec.last().map(|n| unsafe{ &*n.node }).unwrap()
    }

    #[inline]
    pub fn top_mut(&mut self) -> &mut (Node<A, F> + 'a) {
        self.vec.last_mut().map(|n| unsafe{ &mut *n.node }).unwrap()
    }

    #[inline]
    pub fn top_ident(&self) -> NodeIdent {
        self.vec.last().unwrap().ident
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.vec.len()
    }

    #[inline]
    pub fn truncate(&mut self, len: usize) {
        assert_ne!(0, len);
        self.vec.truncate(len);

        self.top_parent_offset = Vector2::new(0, 0);
        for bounds in self.vec[..len-1].iter().map(|n| n.bounds) {
            self.top_parent_offset += bounds.min().to_vec();
        }
    }

    #[inline]
    pub fn top_parent_offset(&self) -> Vector2<u32> {
        self.top_parent_offset
    }

    #[inline]
    pub fn top_bounds_offset(&self) -> BoundRect<u32> {
        self.top().bounds() + self.top_parent_offset
    }

    #[inline]
    pub fn nodes<'b>(&'b self) -> impl 'b + Iterator<Item=&'a Node<A, F>> + DoubleEndedIterator + ExactSizeIterator {
        self.vec.iter().map(|n| unsafe{ &*n.node })
    }

    #[inline]
    pub fn try_push<G>(&mut self, with_top: G)
        where G: FnOnce(&'a mut Node<A, F>) -> Option<(&'a mut Node<A, F>, NodeIdent)>
    {
        let new_top_opt = with_top(unsafe{ mem::transmute(self.top_mut()) } );
        if let Some((new_top, new_top_ident)) = new_top_opt {
            assert_ne!(new_top as *mut Node<A, F>, self.top_mut() as *mut _);
            {
                let cur_top = self.vec.last_mut().unwrap();

                cur_top.bounds = unsafe{ &*cur_top.node }.bounds();
                self.top_parent_offset += cur_top.bounds.min().to_vec();
            }

            self.vec.push(StackElement {
                node: new_top,
                bounds: BoundRect::new(0xDEADBEEF, 0xDEADBEEF, 0xDEADBEEF, 0xDEADBEEF),
                ident: new_top_ident
            });
        }
    }

    #[inline]
    pub fn pop(&mut self) -> Option<&'a mut Node<A, F>> {
        // Ensure the base is never popped
        if self.vec.len() == 1 {
            return None;
        }

        let popped = self.vec.pop().map(|n| unsafe{ &mut *n.node });
        if let Some(last_mut) = self.vec.last_mut() {
            self.top_parent_offset -= last_mut.bounds.min().to_vec();
            last_mut.bounds = BoundRect::new(0xDEADBEEF, 0xDEADBEEF, 0xDEADBEEF, 0xDEADBEEF);
        }

        popped
    }
}

impl<'a, A, F: RenderFrame> Drop for NRVec<'a, A, F> {
    fn drop(&mut self) {
        self.vec.clear();
        let mut vec = unsafe {
            let (ptr, len, cap) = (self.vec.as_ptr(), self.vec.len(), self.vec.capacity());
            Vec::from_raw_parts(mem::transmute::<_, *mut StackElement<'static, A, F>>(ptr), len, cap)
        };
        let mut empty_vec = unsafe {
            let (ptr, len, cap) = (self.cache.as_ptr(), self.cache.len(), self.cache.capacity());
            Vec::from_raw_parts(mem::transmute::<_, *mut StackElement<'a, A, F>>(ptr), len, cap)
        };

        mem::swap(self.cache, &mut vec);
        mem::swap(&mut self.vec, &mut empty_vec);

        mem::forget(vec);
        mem::forget(empty_vec);
    }
}

