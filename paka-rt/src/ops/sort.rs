/// M2-14 section 1.13's merge sort of positions `order`. `before(right, left)` says whether the
/// right item must come first; the left one is taken on `false`. Sorting positions, not items, lets
/// each back end move its own items once at the end while making the same calls in the same order.
pub fn merge_sort<E>(
    order: &mut [usize],
    scratch: &mut Vec<usize>,
    before: &mut impl FnMut(usize, usize) -> Result<bool, E>,
) -> Result<(), E> {
    if order.len() < 2 {
        return Ok(());
    }
    let mid = order.len() / 2;
    merge_sort(&mut order[..mid], scratch, before)?;
    merge_sort(&mut order[mid..], scratch, before)?;
    let (left, right) = order.split_at(mid);
    let (mut i, mut j) = (0, 0);
    while i < left.len() && j < right.len() {
        if before(right[j], left[i])? {
            scratch.push(right[j]);
            j += 1;
        } else {
            scratch.push(left[i]);
            i += 1;
        }
    }
    scratch.extend(&left[i..]);
    scratch.extend(&right[j..]);
    order.copy_from_slice(scratch);
    scratch.clear();
    Ok(())
}

/// The positions `0..n` in sorted order.
pub fn sorted<E>(
    n: usize,
    mut before: impl FnMut(usize, usize) -> Result<bool, E>,
) -> Result<Vec<usize>, E> {
    let mut order: Vec<usize> = (0..n).collect();
    merge_sort(&mut order, &mut Vec::with_capacity(n), &mut before)?;
    Ok(order)
}
