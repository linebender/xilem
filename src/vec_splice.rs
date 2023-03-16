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
                let l = self.scratch.len();
                self.scratch.extend(self.v.splice(self.ix + n.., []));
                self.scratch[l..].reverse();
            }
            self.v.truncate(self.ix);
        }
    }

    pub fn push(&mut self, value: T) {
        if self.v.len() > self.ix {
            let l = self.scratch.len();
            self.scratch.extend(self.v.splice(self.ix.., []));
            self.scratch[l..].reverse();
        }
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

    pub fn len(&self) -> usize {
        self.ix
    }
}