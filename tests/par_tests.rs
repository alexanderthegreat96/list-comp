use list_comp::par_comp;
use rayon::iter::ParallelIterator;

#[test]
fn par_simple_map() {
    let mut v: Vec<i32> = par_comp!(x * 2 for x in 0..1000i32).collect();
    v.sort();
    let expected: Vec<i32> = (0..1000).map(|x| x * 2).collect();
    assert_eq!(v, expected);
}

#[test]
fn par_with_boolean_filter() {
    let sum: i64 = par_comp!(x as i64 for x in 0..1000i32 if x % 2 == 0).sum();
    let expected: i64 = (0..1000i32).filter(|x| x % 2 == 0).map(|x| x as i64).sum();
    assert_eq!(sum, expected);
}

#[test]
fn par_with_if_let_binding() {
    let data: Vec<Option<i32>> = (0..100)
        .map(|x| if x % 3 == 0 { Some(x) } else { None })
        .collect();
    let sum: i64 = par_comp!(v as i64 for x in data if let Some(v) = x).sum();
    let expected: i64 = (0..100i32).filter(|x| x % 3 == 0).map(|x| x as i64).sum();
    assert_eq!(sum, expected);
}

#[test]
fn par_nested_loops() {
    let count = par_comp!((x, y) for x in 0..10i32 for y in 0..10i32 if (x + y) % 2 == 0).count();
    assert_eq!(count, 50);
}
