//! Python-style comprehension macros for Rust.
//!
//! - [`comp!`]      — returns an `Iterator`.
//! - [`vec_comp!`]  — collects into a `Vec`.
//! - [`set_comp!`]  — collects into a `HashSet`.
//! - [`map_comp!`]  — `k => v` syntax; collects into a `HashMap`.
//! - [`par_comp!`]  — returns a `rayon::iter::ParallelIterator`
//!   (requires `rayon` in the caller's `Cargo.toml`).
//!
//! All macros share the same clause grammar:
//!
//! ```text
//! MAPPING
//!     for PAT in SEQ
//!     [if COND | if let PAT = EXPR]*
//!     [for PAT in SEQ [if COND | if let PAT = EXPR]*]*
//! ```
//!
//! Original idea and first implementation by kepler-5:
//! https://gist.github.com/kepler-5/065346184523890310e38bcf9adff03e

use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{
    Expr, Pat, Token,
    parse::{Parse, ParseStream},
    parse_macro_input, parse_quote,
};

struct Comprehension {
    mapping: Expr,
    clauses: Vec<ForIfClause>,
}

struct MapComprehension {
    key: Expr,
    value: Expr,
    clauses: Vec<ForIfClause>,
}

struct ForIfClause {
    pattern: Pat,
    sequence: Expr,
    conditions: Vec<Condition>,
}

enum Condition {
    Boolean(Expr),
    Let(Pat, Expr),
}

fn parse_clauses(input: ParseStream) -> syn::Result<Vec<ForIfClause>> {
    let first: ForIfClause = input.parse()?;
    let mut clauses = vec![first];
    while input.peek(Token![for]) {
        clauses.push(input.parse()?);
    }
    Ok(clauses)
}

impl Parse for Comprehension {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mapping = input.parse()?;
        let clauses = parse_clauses(input)?;
        Ok(Self { mapping, clauses })
    }
}

impl Parse for MapComprehension {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key = input.parse()?;
        input.parse::<Token![=>]>()?;
        let value = input.parse()?;
        let clauses = parse_clauses(input)?;
        Ok(Self {
            key,
            value,
            clauses,
        })
    }
}

impl Parse for ForIfClause {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<Token![for]>()?;
        let pattern = Pat::parse_single(input)?;
        input.parse::<Token![in]>()?;
        let sequence = input.parse()?;
        let mut conditions = Vec::new();
        while input.peek(Token![if]) {
            conditions.push(input.parse()?);
        }
        Ok(Self {
            pattern,
            sequence,
            conditions,
        })
    }
}

impl Parse for Condition {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<Token![if]>()?;
        if input.peek(Token![let]) {
            input.parse::<Token![let]>()?;
            let pat = Pat::parse_single(input)?;
            input.parse::<Token![=]>()?;
            let expr = input.parse::<Expr>()?;
            Ok(Condition::Let(pat, expr))
        } else {
            Ok(Condition::Boolean(input.parse()?))
        }
    }
}

/// Whether to emit a sequential (`core::iter`) or parallel (`rayon`) chain.
#[derive(Copy, Clone)]
enum Mode {
    Sequential,
    Parallel,
}

impl Mode {
    /// The `into_iter`-like path for this mode.
    fn into_iter_path(self) -> TokenStream2 {
        match self {
            Mode::Sequential => quote! { ::core::iter::IntoIterator::into_iter },
            Mode::Parallel => quote! { ::rayon::iter::IntoParallelIterator::into_par_iter },
        }
    }
}

/// A macro-hygienic name for the per-clause `let` binding of the sequence.
/// Using `Span::mixed_site()` keeps it from colliding with any `__seq` the
/// caller might have in scope.
fn seq_ident() -> Ident {
    Ident::new("__seq", Span::mixed_site())
}

impl Comprehension {
    fn expand(&self, mode: Mode) -> TokenStream2 {
        let mut clauses = self.clauses.iter().rev();
        // Parser guarantees at least one clause.
        let innermost = clauses.next().expect("at least one `for` clause");
        let mut out = innermost.expand_innermost(&self.mapping, mode);
        for clause in clauses {
            out = clause.expand_outer(&out, mode);
        }
        out
    }
}

impl ForIfClause {
    /// Innermost clause: yields `Iterator<Item = Mapping>` (or the parallel
    /// equivalent). No `iter::once` wrapping.
    fn expand_innermost(&self, mapping: &Expr, mode: Mode) -> TokenStream2 {
        let ForIfClause {
            pattern,
            sequence,
            conditions,
        } = self;
        let span = sequence.span();
        let seq = seq_ident();
        let into_iter = mode.into_iter_path();

        if conditions.is_empty() {
            quote_spanned! { span =>
                {
                    let #seq = #sequence;
                    #into_iter(#seq).map(move |#pattern| #mapping)
                }
            }
        } else {
            let body = wrap_conditions(
                conditions,
                quote! { ::core::option::Option::Some(#mapping) },
            );
            quote_spanned! { span =>
                {
                    let #seq = #sequence;
                    #into_iter(#seq).filter_map(move |#pattern| { #body })
                }
            }
        }
    }

    /// Outer clause: wraps an already-built inner iterator expression.
    fn expand_outer(&self, inner: &TokenStream2, mode: Mode) -> TokenStream2 {
        let ForIfClause {
            pattern,
            sequence,
            conditions,
        } = self;
        let span = sequence.span();
        let seq = seq_ident();
        let into_iter = mode.into_iter_path();

        if conditions.is_empty() {
            quote_spanned! { span =>
                {
                    let #seq = #sequence;
                    #into_iter(#seq).flat_map(move |#pattern| #inner)
                }
            }
        } else {
            let body = wrap_conditions(conditions, quote! { ::core::option::Option::Some(#inner) });
            quote_spanned! { span =>
                {
                    let #seq = #sequence;
                    #into_iter(#seq)
                        .filter_map(move |#pattern| { #body })
                        .flatten()
                }
            }
        }
    }
}

/// Build a right-nested `if` / `if let` chain whose innermost branch evaluates
/// `tail` and whose every miss-branch yields `Option::None`. Bindings from
/// `if let` conditions are in scope for all subsequent conditions and `tail`.
fn wrap_conditions(conditions: &[Condition], tail: TokenStream2) -> TokenStream2 {
    let mut out = tail;
    for cond in conditions.iter().rev() {
        out = match cond {
            Condition::Boolean(expr) => quote_spanned! { expr.span() =>
                if #expr { #out } else { ::core::option::Option::None }
            },
            Condition::Let(pat, expr) => quote_spanned! { expr.span() =>
                if let #pat = #expr { #out } else { ::core::option::Option::None }
            },
        };
    }
    out
}

/// Python-style list comprehension returning an `Iterator`.
///
/// ```ignore
/// use list_compr::comp;
///
/// let doubled: Vec<i32> = comp!(x * 2 for x in 1..4).collect();
/// let evens:   Vec<i32> = comp!(x for x in 0..10 if x % 2 == 0).collect();
/// let pairs:   Vec<_>   = comp!((x, y) for x in 0..2 for y in 0..2).collect();
///
/// // `if let` bindings are in scope for the mapping:
/// let data = vec![Some(1), None, Some(3)];
/// let vs: Vec<i32> = comp!(v for x in data if let Some(v) = x).collect();
/// ```
#[proc_macro]
pub fn comp(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let c = parse_macro_input!(input as Comprehension);
    c.expand(Mode::Sequential).into()
}

/// Like [`comp!`] but eagerly collects into a `Vec`.
///
/// ```ignore
/// use list_compr::vec_comp;
/// let v = vec_comp!(x * x for x in 1..=3); // Vec<i32> = [1, 4, 9]
/// ```
#[proc_macro]
pub fn vec_comp(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let c = parse_macro_input!(input as Comprehension);
    let iter = c.expand(Mode::Sequential);
    quote! {
        ::core::iter::Iterator::collect::<::std::vec::Vec<_>>(#iter)
    }
    .into()
}

/// Like [`comp!`] but eagerly collects into a `HashSet`.
///
/// ```ignore
/// use list_compr::set_comp;
/// let s = set_comp!(x % 3 for x in 0..10); // HashSet<i32> = {0, 1, 2}
/// ```
#[proc_macro]
pub fn set_comp(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let c = parse_macro_input!(input as Comprehension);
    let iter = c.expand(Mode::Sequential);
    quote! {
        ::core::iter::Iterator::collect::<::std::collections::HashSet<_>>(#iter)
    }
    .into()
}

/// Python-style dict comprehension. Uses `KEY => VALUE` as the mapping and
/// collects into a `HashMap`.
///
/// ```ignore
/// use list_compr::map_comp;
/// let squares = map_comp!(x => x * x for x in 1..=3);
/// // HashMap<i32, i32> = {1: 1, 2: 4, 3: 9}
/// ```
#[proc_macro]
pub fn map_comp(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let MapComprehension {
        key,
        value,
        clauses,
    } = parse_macro_input!(input as MapComprehension);
    let mapping: Expr = parse_quote!((#key, #value));
    let c = Comprehension { mapping, clauses };
    let iter = c.expand(Mode::Sequential);
    quote! {
        ::core::iter::Iterator::collect::<::std::collections::HashMap<_, _>>(#iter)
    }
    .into()
}

/// Same grammar as [`comp!`], but emits a `rayon::iter::ParallelIterator`
/// chain (`into_par_iter` / parallel `map`, `filter_map`, `flat_map`).
///
/// The caller must have `rayon` in their `Cargo.toml`. Worth using when the
/// mapping is expensive or the outermost sequence is large; for trivial
/// mappings over small ranges, stick with [`comp!`].
///
/// ```ignore
/// use list_compr::par_comp;
/// use rayon::iter::ParallelIterator;
///
/// let total: u64 = par_comp!(expensive(x) for x in 0..1_000_000).sum();
/// ```
#[proc_macro]
pub fn par_comp(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let c = parse_macro_input!(input as Comprehension);
    c.expand(Mode::Parallel).into()
}
