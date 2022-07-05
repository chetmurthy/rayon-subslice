use num::traits::Zero ;
use rayon::prelude::* ;
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

#[derive(Debug)]
pub struct SplitState<'idxs, 'data, 'slices, 'slices2, T> {
    pub idxs: &'idxs [usize],
    pub slices: &'slices [&'slices2 [T]],
    pub data: &'data mut [T],
}

impl<'idxs, 'data, 'slices, 'slices2, T> Iterator for SplitState<'idxs, 'data, 'slices, 'slices2, T> {
    type Item = (&'data mut [T], &'slices2 [T]);
    
    fn next(&mut self) -> Option<(&'data mut [T], &'slices2 [T])> {
        match *self.idxs {
            [chunk_start, chunk_end, ..] => {
                let (chunk, tail) = mem::take(&mut self.data).split_at_mut(chunk_end - chunk_start);
		let (slicehd,  slicetail) = mem::take(&mut self.slices).split_at(1) ;

                self.idxs = &self.idxs[1..];
		self.slices = slicetail ;
                self.data = tail;

                Some((chunk, slicehd[0]))
            }
            _ => None,
        }
    }
    
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.idxs.len() - 1;
        (len, Some(len))
    }
}

pub fn split_slice_mut<'idxs, 'data, 'slices, 'slices2, T>(
    idxs: &'idxs [usize],
    slices: &'slices [&'slices2 [T]],
    data: &'data mut [T],
) -> Vec<(&'data mut[T], &'slices2 [T])> {
    let sst = SplitState { idxs, data, slices } ; 
    let rv : Vec<(&'data mut[T], &'slices2 [T])> = sst.collect() ;
    rv
}

pub fn concat_slices<T>(slices : &[&[T]]) -> Vec<T>
where
    T : Copy + Zero + Sync,
for<'a, 'b> (&'a mut [T], &'b [T]) : Sync + Send
 {
    let full_length : usize =
	slices.iter()
	.map(|s| s.len())
	.sum() ;

    let mut rv : Vec<T> = Vec::with_capacity(full_length) ;
    rv.resize(full_length, T::zero()) ;

    let mut idxs = Vec::with_capacity(slices.len() + 1) ;
    let last = slices.iter()
	.fold(0, |sofar,s| {
	    idxs.push(sofar) ;
	    sofar + s.len()
	}) ;
    assert!(last == full_length) ;
    idxs.push(last) ;

    let mut chunks : Vec<(&mut[T], &[T])> = split_slice_mut(&idxs[..], slices, &mut rv[..]) ;
    assert!(chunks.iter().all(|(c,s)| c.len() == s.len())) ;
    assert!(idxs.len() == chunks.len() + 1) ;

    chunks.iter_mut()
	.for_each(|(c,s)| {
	    c.copy_from_slice(s) ;
	}) ;
    rv
}

pub fn unsafe_concat_slices<T>(slices : &[&[T]]) -> Vec<T>
where
    T : Copy + Zero + Sync,
for<'a, 'b> (&'a mut [T], &'b [T]) : Sync + Send
 {
    let full_length : usize =
	slices.iter()
	.map(|s| s.len())
	.sum() ;

    let mut rv : Vec<T> = Vec::with_capacity(full_length) ;
    unsafe { rv.set_len(full_length) ; }

    let mut idxs = Vec::with_capacity(slices.len() + 1) ;
    let last = slices.iter()
	.fold(0, |sofar,s| {
	    idxs.push(sofar) ;
	    sofar + s.len()
	}) ;
    assert!(last == full_length) ;
    idxs.push(last) ;

    let mut chunks : Vec<(&mut[T], &[T])> = split_slice_mut(&idxs[..], slices, &mut rv[..]) ;
    assert!(chunks.iter().all(|(c,s)| c.len() == s.len())) ;
    assert!(idxs.len() == chunks.len() + 1) ;

    chunks.iter_mut()
	.for_each(|(c,s)| {
	    c.copy_from_slice(s) ;
	}) ;
    rv
}

pub fn par_concat_slices<T>(slices : &[&[T]]) -> Vec<T>
where
    T : Copy + Zero + Sync,
for<'a, 'b> (&'a mut [T], &'b [T]) : Sync + Send
 {
    let full_length : usize =
	slices.iter()
	.map(|s| s.len())
	.sum() ;

    let mut rv : Vec<T> = Vec::with_capacity(full_length) ;
    rv.resize(full_length, T::zero()) ;

    let mut idxs = Vec::with_capacity(slices.len() + 1) ;
    let last = slices.iter()
	.fold(0, |sofar,s| {
	    idxs.push(sofar) ;
	    sofar + s.len()
	}) ;
    assert!(last == full_length) ;
    idxs.push(last) ;

    let mut chunks : Vec<(&mut[T], &[T])> = split_slice_mut(&idxs[..], slices, &mut rv[..]) ;
    assert!(chunks.iter().all(|(c,s)| c.len() == s.len())) ;
    assert!(idxs.len() == chunks.len() + 1) ;

    chunks.par_iter_mut()
	.for_each(|(c,s)| {
	    c.copy_from_slice(s) ;
	}) ;
    rv
}

pub fn unsafe_par_concat_slices<T>(slices : &[&[T]]) -> Vec<T>
where
    T : Copy + Zero + Sync,
for<'a, 'b> (&'a mut [T], &'b [T]) : Sync + Send
 {
    let full_length : usize =
	slices.iter()
	.map(|s| s.len())
	.sum() ;

    let mut rv : Vec<T> = Vec::with_capacity(full_length) ;
    unsafe { rv.set_len(full_length) ; }

    let mut idxs = Vec::with_capacity(slices.len() + 1) ;
    let last = slices.iter()
	.fold(0, |sofar,s| {
	    idxs.push(sofar) ;
	    sofar + s.len()
	}) ;
    assert!(last == full_length) ;
    idxs.push(last) ;

    let mut chunks : Vec<(&mut[T], &[T])> = split_slice_mut(&idxs[..], slices, &mut rv[..]) ;
    assert!(chunks.iter().all(|(c,s)| c.len() == s.len())) ;
    assert!(idxs.len() == chunks.len() + 1) ;

    chunks.par_iter_mut()
	.for_each(|(c,s)| {
	    c.copy_from_slice(s) ;
	}) ;
    rv
}

#[cfg(test)]
mod tests {
    use rayon::prelude::*;
    use crate::concat_slices;
    use crate::SplitState;
    use crate::SubSlices;

    #[test]
    fn test1() {
	let rv = concat_slices(&[ &"abc".chars().map(|c| c as u8).collect::<Vec<_>>()[..] ][..]) ;
	let rv = rv.iter().map(|c| *c as char).collect::<String>() ;
	assert_eq! (rv, "abc") ;
    }

    #[test]
    fn test3() {
	let s = "abc1234XY" ;
	let mut data = s.chars().map(|c| c as u8).collect::<Vec<_>>();
	let rv = concat_slices(&[ &data[0..3], &data[3..4], &data[4..7], &data[7..9] ][..]) ;
	let rv = rv.iter().map(|c| *c as char).collect::<String>() ;
	assert_eq!(s, rv) ;
    }

}
