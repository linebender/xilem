// Copyright 2023 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

pub(crate) struct VecSplice<'v, 's, T> {
    v: &'v mut Vec<T>,
    scratch: &'s mut Vec<T>,
    ix: usize,
}

impl<'v, 's, T> VecSplice<'v, 's, T> {
    pub fn new(v: &'v mut Vec<T>, scratch: &'s mut Vec<T>) -> Self {
        Self { v, scratch, ix: 0 }
    }

    pub fn skip(&mut self, n: usize) {
        let v_len = self.v.len();
        let new_ix = self.ix + n;
        // 2 < 3
        // ix: 0, n = 3
        if v_len < new_ix {
            let s_len = self.scratch.len();
            if v_len + s_len < new_ix {
                unreachable!("This is a bug, please report an issue about `ElementSplice::skip`");
            }
            let new_scratch_len = s_len - (new_ix - v_len);
            self.v.extend(self.scratch.splice(new_scratch_len.., []));
        }
        self.ix += n;
    }

    pub fn delete_next(&mut self) -> T {
        self.clear_tail();
        self.scratch
            .pop()
            .expect("This is a bug, please report an issue about `ElementSplice::delete`")
    }

    pub fn insert(&mut self, value: T) {
        self.clear_tail();
        self.v.push(value);
        self.ix += 1;
    }

    pub fn next_mut(&mut self) -> Option<&mut T> {
        self.v
            .get_mut(self.ix + 1)
            .or_else(|| self.scratch.last_mut())
    }

    pub fn mutate(&mut self) -> &mut T {
        if self.v.len() == self.ix {
            self.v.push(self.scratch.pop().unwrap());
        }
        let ix = self.ix;
        self.ix += 1;
        &mut self.v[ix]
    }

    fn clear_tail(&mut self) {
        if self.v.len() > self.ix {
            self.scratch.extend(self.v.splice(self.ix.., []).rev());
        }
    }
}
