// Copyright 2023 the Druid Authors.
// SPDX-License-Identifier: Apache-2.0

use masonry::widget::WidgetMut;

use crate::ElementSplice;

pub struct VecSplice<'a, 'b, T> {
    v: &'a mut Vec<T>,
    scratch: &'b mut Vec<T>,
    ix: usize,
}

impl<'a, 'b, T> VecSplice<'a, 'b, T> {
    pub fn new(v: &'a mut Vec<T>, scratch: &'b mut Vec<T>) -> Self {
        let ix = 0;
        VecSplice { v, scratch, ix }
    }

    pub fn skip(&mut self, n: usize) {
        if self.v.len() < self.ix + n {
            let l = self.scratch.len();
            self.v.extend(self.scratch.splice(l - n.., []));
            self.v[self.ix..].reverse();
        }
        self.ix += n;
    }

    pub fn delete(&mut self, n: usize) {
        if self.v.len() < self.ix + n {
            self.scratch.truncate(self.scratch.len() - n);
        } else {
            if self.v.len() > self.ix + n {
                let removed = self.v.splice(self.ix + n.., []).rev();
                self.scratch.extend(removed);
            }
            self.v.truncate(self.ix);
        }
    }

    pub fn push(&mut self, value: T) {
        self.clear_tail();
        self.v.push(value);
        self.ix += 1;
    }

    pub fn mutate(&mut self) -> &mut T {
        if self.v.len() == self.ix {
            self.v.push(self.scratch.pop().unwrap());
        }
        let ix = self.ix;
        self.ix += 1;
        &mut self.v[ix]
    }

    pub fn last_mutated(&self) -> Option<&T> {
        if self.ix == 0 {
            None
        } else {
            self.v.get(self.ix - 1)
        }
    }

    pub fn last_mutated_mut(&mut self) -> Option<&mut T> {
        if self.ix == 0 {
            None
        } else {
            self.v.get_mut(self.ix - 1)
        }
    }

    pub fn len(&self) -> usize {
        self.ix
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_vec<R, F: FnOnce(&mut Vec<T>) -> R>(&mut self, f: F) -> R {
        self.clear_tail();
        let ret = f(self.v);
        self.ix = self.v.len();
        ret
    }

    fn clear_tail(&mut self) {
        if self.v.len() > self.ix {
            let removed = self.v.splice(self.ix.., []).rev();
            self.scratch.extend(removed);
        }
    }
}

impl ElementSplice for VecSplice<'_, '_, masonry::WidgetPod<Box<dyn masonry::Widget>>> {
    fn push(&mut self, element: masonry::WidgetPod<Box<dyn masonry::Widget>>) {
        self.push(element);
    }

    fn mutate(&mut self) -> WidgetMut<Box<dyn masonry::Widget>> {
        unreachable!("VecSplice can only be used for `build`, not rebuild")
    }

    fn delete(&mut self, n: usize) {
        self.delete(n);
    }

    fn len(&self) -> usize {
        self.len()
    }
}
