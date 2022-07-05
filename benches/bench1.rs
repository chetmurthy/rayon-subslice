use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::mem::size_of ;
use rayon::prelude::*;
use rayon_subslice::{ concat_slices, par_concat_slices, unsafe_concat_slices, unsafe_par_concat_slices };
use num_complex::Complex64;
use num_traits::identities::Zero;
use si_scale::helpers::{bibytes1};
use conv::prelude::* ;

fn f_nbytes(nslices : usize, slicelen : usize) -> f64 {
    let n = nslices * slicelen * size_of::<Complex64>() ;
    f64::value_from(n).unwrap()
}

struct Setup<T> {
    nslices : usize,
    slicelen : usize,
    vecs : Vec<Vec<T>>,
}

impl<T : Zero + Clone> Setup<T> {

    fn label(&self) -> String {
	format!("{} x {} ({})",
		self.nslices, self.slicelen,
		bibytes1(f_nbytes(self.nslices, self.slicelen)))
    }

    fn new<'a>(nslices : usize, slicelen : usize) -> Self {
	let vecs : Vec<Vec<T>> = (0..nslices).into_iter()
	    .map(|_| vec![T::zero(); slicelen])
	    .collect() ;

	let it = Setup::<T> { nslices, slicelen, vecs } ;
	it
    }

    fn slices<'a>(&'a self) -> Vec<&'a [T]> {
	let slices : Vec<&[T]> = self.vecs.iter()
	    .map(|v| &v[..])
	    .collect() ;
	slices
    }
}

fn serial_bench<T : Zero + Clone + Copy + Sync + Send>(it : &Setup<T>) {
    let slices = it.slices() ;
    concat_slices(&slices[..]);
}

fn unsafe_serial_bench<T : Zero + Clone + Copy + Sync + Send>(it : &Setup<T>) {
    let slices = it.slices() ;
    unsafe_concat_slices(&slices[..]);
}

fn parallel_bench<T : Zero + Clone + Copy + Sync + Send>(it : &Setup<T>) {
    let slices = it.slices() ;
    par_concat_slices(&slices[..]);
}

fn unsafe_parallel_bench<T : Zero + Clone + Copy + Sync + Send>(it : &Setup<T>) {
    let slices = it.slices() ;
    unsafe_par_concat_slices(&slices[..]);
}

pub fn criterion_benchmark(c: &mut Criterion) {
    let it = Setup::<Complex64>::new(1<<10, 1<<20) ;
    let mut group = c.benchmark_group("serial");
    group.sample_size(10);
    group.bench_function(it.label(),
			 |b| b.iter(|| serial_bench(&it)));
    group.finish() ;
    let mut group = c.benchmark_group("serial+unsafe");
    group.sample_size(10);
    group.bench_function(it.label(),
			 |b| b.iter(|| unsafe_serial_bench(&it)));
    group.finish() ;
    let mut group = c.benchmark_group("parallel");
    group.sample_size(10);
    group.bench_function(it.label(),
			 |b| b.iter(|| parallel_bench(&it)));
    group.finish() ;
    let mut group = c.benchmark_group("parallel+unsafe");
    group.sample_size(10);
    group.bench_function(it.label(),
			 |b| b.iter(|| unsafe_parallel_bench(&it)));
    group.finish() ;
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
