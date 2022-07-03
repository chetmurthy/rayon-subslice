use rayon::prelude::*;
use rayon_subslice::SubSlices;

fn main() {
    let mut data = "abc1234XY".chars().collect::<Vec<_>>();
    let idxs = [0, 3, 4, 7, 9];
    
    // Note: not using .collect() method syntax to make sure
    // we're actually using parallel, not regular, iterator :)
    let chunks: Vec<_> = ParallelIterator::collect(
        SubSlices {
            idxs: &idxs,
            data: &mut data,
        }
    );
    dbg!(chunks);
}
