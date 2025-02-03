use std::collections::VecDeque;

pub struct OverlappingWindowIter<I>
where
    I: Iterator,
{
    iterator: I,
    buffer: VecDeque<I::Item>,
    window_size: usize,
    step_size: usize,
    filled: bool,
}

impl<I: Iterator> OverlappingWindowIter<I> {
    pub fn new(iterator: I, window_size: usize, step_size: usize) -> Self {
        Self {
            iterator,
            buffer: VecDeque::new(),
            window_size,
            step_size,
            filled: false,
        }
    }
}

impl<I: Iterator> Iterator for OverlappingWindowIter<I>
where
    I::Item: Clone,
{
    type Item = Vec<I::Item>; // TODO

    fn next(&mut self) -> Option<Self::Item> {
        if !self.filled {
            while self.buffer.len() < self.window_size {
                if let Some(next) = self.iterator.next() {
                    self.buffer.push_back(next)
                } else {
                    return None;
                }
            }

            self.filled = true;
            return Some(self.buffer.iter().cloned().collect());
        }

        let mut step = Vec::new();
        for _ in 0..self.step_size {
            if let Some(next) = self.iterator.next() {
                step.push(next);
            } else {
                return None;
            }
        }

        self.buffer.extend(step);
        while self.buffer.len() > self.window_size {
            let _ = self.buffer.pop_front();
        }

        Some(self.buffer.iter().cloned().collect())
    }
}

impl<I: ExactSizeIterator> ExactSizeIterator for OverlappingWindowIter<I>
where
    I::Item: Clone,
{
    fn len(&self) -> usize {
        (self.iterator.len() - self.window_size) / self.step_size
    }
}

pub trait IterExt<I: Iterator> {
    fn overlapping_windows(self, window_size: usize, overlap: usize) -> OverlappingWindowIter<I>;
}

impl<I> IterExt<I> for I
where
    I: Iterator + Sized,
{
    fn overlapping_windows(self, window_size: usize, overlap: usize) -> OverlappingWindowIter<I> {
        OverlappingWindowIter::new(self, window_size, overlap)
    }
}
