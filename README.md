# list_comp

Python-style comprehension macros for Rust. Five macros sharing one grammar:

| Macro        | Returns                              |
|--------------|--------------------------------------|
| `comp!`      | `impl Iterator`                      |
| `vec_comp!`  | `Vec<_>`                             |
| `set_comp!`  | `HashSet<_>`                         |
| `map_comp!`  | `HashMap<_, _>` (uses `K => V` form) |
| `par_comp!`  | `impl rayon::iter::ParallelIterator` |

## Grammar

```text
MAPPING
    for PAT in SEQ
    [ if COND | if let PAT = EXPR ]*
    [ for PAT in SEQ [ if COND | if let PAT = EXPR ]* ]*
```

`map_comp!` uses `KEY => VALUE` in place of `MAPPING`.

Bindings introduced by `if let PAT = EXPR` are in scope for every subsequent
condition, for every nested `for` clause, and for the mapping.

## Usage

Add to `Cargo.toml`:

```toml
[dependencies]
list_comp = { version="0.1.0" }
```

For `par_comp!`, also add `rayon`:

```toml
[dependencies]
rayon = "1"
```

## Examples

### `comp!` — iterator

```rust
use list_comp::comp;

let doubled: Vec<i32> = comp!(x * 2 for x in 1..4).collect();
// [2, 4, 6]

let evens: Vec<i32> = comp!(x for x in 0..10 if x % 2 == 0).collect();
// [0, 2, 4, 6, 8]

let pairs: Vec<_> = comp!((x, y) for x in 0..2 for y in 0..2).collect();
// [(0,0), (0,1), (1,0), (1,1)]

// Flatten + filter a matrix:
let matrix = vec![vec![1, 2], vec![3, 4]];
let out: Vec<i32> = comp!(v * 10 for row in matrix for v in row if v % 2 == 0).collect();
// [20, 40]
```

### `if let` brings bindings into scope

```rust
let data = vec![Some(1), None, Some(3)];
let vs: Vec<i32> = comp!(v for x in data if let Some(v) = x).collect();
// [1, 3]
```

`v` is visible to any later conditions and to the mapping.

### `vec_comp!` / `set_comp!`

```rust
use list_comp::{set_comp, vec_comp};

let squares: Vec<i32> = vec_comp!(x * x for x in 1..=3);
// [1, 4, 9]

let mod3: std::collections::HashSet<i32> = set_comp!(x % 3 for x in 0..10);
// {0, 1, 2}
```

### `map_comp!` — `KEY => VALUE`

```rust
use list_comp::map_comp;

let squares = map_comp!(x => x * x for x in 1..=3);
// HashMap { 1: 1, 2: 4, 3: 9 }

let lengths = map_comp!(s => s.len() for s in ["a", "bb", "ccc"] if s.len() >= 2);
// HashMap { "bb": 2, "ccc": 3 }
```

### `par_comp!` — rayon parallel iterator

```rust
use list_comp::par_comp;
use rayon::iter::ParallelIterator;

let total: u64 = par_comp!(expensive(x) for x in 0..1_000_000i64).sum();
```

Grammar is identical to `comp!`; only the emitted iterator differs
(`into_par_iter` / parallel `map` / `filter_map` / `flat_map` / `flatten`).
Worth using when the mapping is expensive or the outer sequence is large.
For trivial work on small ranges, rayon's scheduling overhead makes `comp!`
the faster choice.

## Diagnostics

The parser surfaces explicit errors for missing tokens:

```text
comp!(42)                    → expected `for`
comp!(x for x 0..3)          → expected `in`
comp!(x for x in 0..3 blob)  → unexpected token
map_comp!(k v for k in 0..3) → expected `=>`
```

These are locked in by [`trybuild`](https://github.com/dtolnay/trybuild)
compile-fail tests in `tests/ui/`.

## Credit

Original idea and first implementation by
[kepler-5](https://gist.github.com/kepler-5/065346184523890310e38bcf9adff03e).
This crate adds `if let` binding propagation, the `vec_comp!` / `set_comp!` /
`map_comp!` / `par_comp!` variants, macro hygiene for the internal sequence
binding, and a stricter parser.
