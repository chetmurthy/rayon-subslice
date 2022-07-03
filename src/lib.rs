use rayon::iter::plumbing;
use rayon::iter::{ParallelIterator, IndexedParallelIterator};

use std::mem;
use std::iter;

#[derive(Debug)]
pub struct SubSlices<'idxs, 'data, T> {
    pub idxs: &'idxs [usize],
    pub data: &'data mut [T],
}

impl<'idxs, 'data, T> Iterator for SubSlices<'idxs, 'data, T> {
    type Item = &'data mut [T];
    
    fn next(&mut self) -> Option<&'data mut [T]> {
        match *self.idxs {
            [chunk_start, chunk_end, ..] => {
                let (chunk, tail) = mem::take(&mut self.data).split_at_mut(chunk_end - chunk_start);

                self.idxs = &self.idxs[1..];
                self.data = tail;

                Some(chunk)
            }
            _ => None,
        }
    }
    
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.idxs.len() - 1;
        (len, Some(len))
    }
}

impl<'idxs, 'data, T> iter::ExactSizeIterator for SubSlices<'idxs, 'data, T> {}

impl<'idxs, 'data, T> iter::DoubleEndedIterator for SubSlices<'idxs, 'data, T> {
    fn next_back(&mut self) -> Option<&'data mut [T]> { todo!() }
}


impl<'idxs, 'data, T: Send> plumbing::Producer for SubSlices<'idxs, 'data, T> {
    type Item = &'data mut [T];
    type IntoIter = Self;

    fn into_iter(self) -> Self { self }
    
    fn split_at(self, mid: usize) -> (Self, Self) {
        assert!(mid != 0);
        assert!(mid < self.idxs.len() - 1);
        
        dbg!(mid);

        let (idxs_r, idxs_l) = (&self.idxs[0..=mid], &self.idxs[mid..]);
        let (data_r, data_l) = self.data.split_at_mut(idxs_l[0] - idxs_r[0]);
        (
            Self {
                idxs: idxs_r,
                data: data_r,
            },
            Self {
                idxs: idxs_l,
                data: data_l,
            },
        )
    }
}

impl<'idxs, 'data, T: Send> ParallelIterator for SubSlices<'idxs, 'data, T> {
    type Item = &'data mut [T];

    fn opt_len(&self) -> Option<usize> { Some(IndexedParallelIterator::len(self)) }
    
    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: plumbing::UnindexedConsumer<Self::Item>,
    {
        self.drive(consumer)
    }

}

impl<'idxs, 'data, T: Send> IndexedParallelIterator for SubSlices<'idxs, 'data, T> {
    fn len(&self) -> usize { iter::ExactSizeIterator::len(self) }
    
    fn drive<C>(self, consumer: C) -> C::Result
    where
        C: plumbing::Consumer<Self::Item>,
    {
        plumbing::bridge(self, consumer)
    }

    // ???
    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: plumbing::ProducerCallback<Self::Item>,
    {
        callback.callback(self)
    }
}
