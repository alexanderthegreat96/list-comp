use list_comp::{map_comp, set_comp, vec_comp};
use std::collections::{HashMap, HashSet};

#[test]
fn vec_comp_basic() {
    let v = vec_comp!(x * x for x in 1..=3);
    assert_eq!(v, vec![1, 4, 9]);
}

#[test]
fn vec_comp_with_filter_and_if_let() {
    let data = vec![Some(1), None, Some(2), None, Some(3)];
    let v = vec_comp!(n * 10 for x in data if let Some(n) = x if n != 2);
    assert_eq!(v, vec![10, 30]);
}

#[test]
fn set_comp_dedups() {
    let s: HashSet<i32> = set_comp!(x % 3 for x in 0..10);
    let expected: HashSet<i32> = [0, 1, 2].into_iter().collect();
    assert_eq!(s, expected);
}

#[test]
fn map_comp_squares() {
    let m: HashMap<i32, i32> = map_comp!(x => x * x for x in 1..=4);
    assert_eq!(m.get(&1), Some(&1));
    assert_eq!(m.get(&2), Some(&4));
    assert_eq!(m.get(&3), Some(&9));
    assert_eq!(m.get(&4), Some(&16));
    assert_eq!(m.len(), 4);
}

#[test]
fn map_comp_with_filter() {
    let m: HashMap<&'static str, usize> =
        map_comp!(name => name.len() for name in ["a", "bb", "ccc"] if name.len() >= 2);
    assert_eq!(m.len(), 2);
    assert_eq!(m.get("bb"), Some(&2));
    assert_eq!(m.get("ccc"), Some(&3));
}

// Verifies the hygienic `__seq` ident: a caller-scope `__seq` must not be
// shadowed by the macro's internal binding.
#[test]
fn caller_can_use_seq_name() {
    let __seq = vec![1, 2, 3];
    let v = vec_comp!(x * 2 for x in __seq);
    assert_eq!(v, vec![2, 4, 6]);
}
