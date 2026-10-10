//! Loaded framework comparison introsort, including swaps of equal keys.
//! Rust's stable or unstable slice sort has different tie partition behavior.
use std::cmp::Ordering;

pub fn comparison_sort<T: Clone>(rows: &mut [T], compare: impl Fn(&T, &T) -> Ordering) {
    fn swap_if_greater<T>(
        rows: &mut [T],
        i: usize,
        j: usize,
        compare: &impl Fn(&T, &T) -> Ordering,
    ) {
        if i != j && compare(&rows[i], &rows[j]) == Ordering::Greater {
            rows.swap(i, j);
        }
    }
    fn down_heap<T: Clone>(
        rows: &mut [T],
        mut i: usize,
        size: usize,
        lo: usize,
        compare: &impl Fn(&T, &T) -> Ordering,
    ) {
        let value = rows[lo + i - 1].clone();
        while i <= size / 2 {
            let mut child = 2 * i;
            if child < size && compare(&rows[lo + child - 1], &rows[lo + child]) == Ordering::Less {
                child += 1;
            }
            if compare(&value, &rows[lo + child - 1]) != Ordering::Less {
                break;
            }
            rows[lo + i - 1] = rows[lo + child - 1].clone();
            i = child;
        }
        rows[lo + i - 1] = value;
    }
    fn sort<T: Clone>(
        rows: &mut [T],
        lo: usize,
        mut hi: usize,
        mut depth: u32,
        compare: &impl Fn(&T, &T) -> Ordering,
    ) {
        while hi > lo {
            let size = hi - lo + 1;
            if size <= 16 {
                if size == 2 {
                    swap_if_greater(rows, lo, hi, compare);
                } else if size == 3 {
                    swap_if_greater(rows, lo, hi - 1, compare);
                    swap_if_greater(rows, lo, hi, compare);
                    swap_if_greater(rows, hi - 1, hi, compare);
                } else {
                    for i in lo..hi {
                        let value = rows[i + 1].clone();
                        let mut j = i as isize;
                        while j >= lo as isize
                            && compare(&value, &rows[j as usize]) == Ordering::Less
                        {
                            rows[j as usize + 1] = rows[j as usize].clone();
                            j -= 1;
                        }
                        rows[(j + 1) as usize] = value;
                    }
                }
                return;
            }
            if depth == 0 {
                for i in (1..=size / 2).rev() {
                    down_heap(rows, i, size, lo, compare);
                }
                for i in (2..=size).rev() {
                    rows.swap(lo, lo + i - 1);
                    down_heap(rows, 1, i - 1, lo, compare);
                }
                return;
            }
            depth -= 1;
            let middle = lo + (hi - lo) / 2;
            swap_if_greater(rows, lo, middle, compare);
            swap_if_greater(rows, lo, hi, compare);
            swap_if_greater(rows, middle, hi, compare);
            let pivot = rows[middle].clone();
            rows.swap(middle, hi - 1);
            let (mut left, mut right) = (lo, hi - 1);
            while left < right {
                left += 1;
                while compare(&rows[left], &pivot) == Ordering::Less {
                    left += 1;
                }
                right -= 1;
                while compare(&pivot, &rows[right]) == Ordering::Less {
                    right -= 1;
                }
                if left >= right {
                    break;
                }
                rows.swap(left, right);
            }
            rows.swap(left, hi - 1);
            sort(rows, left + 1, hi, depth, compare);
            hi = left - 1;
        }
    }
    if rows.len() > 1 {
        sort(
            rows,
            0,
            rows.len() - 1,
            2 * (usize::BITS - rows.len().leading_zeros()),
            &compare,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn comparison_sort_port_matches_python_partition_vectors() {
        let fixture = crate::json::parse(include_str!("../tests/fixtures/retail_physics_port.json"))
        .unwrap();
        for row in fixture.get_array("sort_vectors").unwrap() {
            let mut input: Vec<(i32, i32)> = row
                .get_array("input")
                .unwrap()
                .iter()
                .map(|v| {
                    let pair = v.as_array().unwrap();
                    (
                        pair[0].as_i64().unwrap() as i32,
                        pair[1].as_i64().unwrap() as i32,
                    )
                })
                .collect();
            comparison_sort(&mut input, |a, b| a.0.cmp(&b.0));
            let actual: Vec<_> = input.iter().map(|v| v.1 as i64).collect();
            let expected: Vec<_> = row
                .get_array("expected_order")
                .unwrap()
                .iter()
                .map(|v| v.as_i64().unwrap())
                .collect();
            assert_eq!(actual, expected);
        }
    }
}
