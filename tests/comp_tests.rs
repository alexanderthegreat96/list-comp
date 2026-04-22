use list_comp::comp;

#[test]
fn test_simple_comprehension() {
    let result: Vec<_> = comp!(x * 2 for x in 1..4).collect();
    assert_eq!(result, vec![2, 4, 6]);
}

#[test]
fn test_boolean_filter() {
    let result: Vec<_> = comp!(x for x in 0..10 if x % 2 == 0).collect();
    assert_eq!(result, vec![0, 2, 4, 6, 8]);
}

#[test]
fn test_nested_loops() {
    let result: Vec<_> = comp!((x, y) for x in 0..2 for y in 0..2).collect();
    assert_eq!(result, vec![(0, 0), (0, 1), (1, 0), (1, 1)]);
}

#[test]
fn test_if_let_filter() {
    let data = vec![Some(1), None, Some(3), None];
    // This tests the new 'if let' support
    let result: Vec<_> = comp!(v for x in data if let Some(v) = x).collect();
    assert_eq!(result, vec![1, 3]);
}

#[test]
fn test_multiple_filters() {
    let result: Vec<_> = comp!(
        x for x in 0..20
        if x % 2 == 0
        if x % 3 == 0
    )
    .collect();
    assert_eq!(result, vec![0, 6, 12, 18]);
}

#[test]
fn test_destructuring_patterns() {
    let pairs = vec![(1, 10), (2, 20), (3, 30)];
    let result: Vec<_> = comp!(a + b for (a, b) in pairs).collect();
    assert_eq!(result, vec![11, 22, 33]);
}

#[test]
fn test_complex_nesting_with_filters() {
    let matrix = vec![vec![1, 2], vec![3, 4]];
    let result: Vec<_> = comp!(
        val * 10
        for row in matrix
        for val in row
        if val % 2 == 0
    )
    .collect();
    assert_eq!(result, vec![20, 40]);
}
