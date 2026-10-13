//! The built-in names with their signatures, in one table (M2-14 sections 1.1 and 1.2, M2-15
//! section 1.3). A row is written in the syntax of a signature; a one-letter name is a type
//! variable. The resolver takes the names from it, the signature table the types.

/// What a use of a name may stand for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Type,
    Variant,
    Fn,
    Grant,
    /// An `example` or an `app`: it holds a name but is never used by name.
    Other,
}

pub const ROWS: &[(Kind, &str)] = &[
    (Kind::Type, "Int"),
    (Kind::Type, "Float"),
    (Kind::Type, "Bool"),
    (Kind::Type, "String"),
    (Kind::Type, "List<T>"),
    (Kind::Type, "Map<K, V>"),
    (Kind::Type, "Option<T>"),
    (Kind::Type, "Result<T, E>"),
    (Kind::Type, "Effect<M>"),
    (Kind::Variant, "Some(value: T) -> Option<T>"),
    (Kind::Variant, "None -> Option<T>"),
    (Kind::Variant, "Ok(value: T) -> Result<T, E>"),
    (Kind::Variant, "Err(error: E) -> Result<T, E>"),
    (Kind::Variant, "Print(text: String) -> Effect<M>"),
    (
        Kind::Variant,
        "ReadFile(path: String; then: fn(String) -> M, or: fn(String) -> M) -> Effect<M>",
    ),
    (
        Kind::Variant,
        "WriteFile(path: String, text: String; then: M, or: fn(String) -> M) -> Effect<M>",
    ),
    (Kind::Variant, "Exit -> Effect<M>"),
    (Kind::Variant, "ExitWith(status: Int) -> Effect<M>"),
    (Kind::Grant, "Console"),
    (Kind::Grant, "ReadFiles"),
    (Kind::Grant, "WriteFiles"),
    (Kind::Fn, "len(x: String) -> Int"),
    (Kind::Fn, "len(x: List<T>) -> Int"),
    (Kind::Fn, "len(x: Map<K, V>) -> Int"),
    (Kind::Fn, "is_empty(x: String) -> Bool"),
    (Kind::Fn, "is_empty(x: List<T>) -> Bool"),
    (Kind::Fn, "is_empty(x: Map<K, V>) -> Bool"),
    (Kind::Fn, "contains(text: String, part: String) -> Bool"),
    (Kind::Fn, "contains(xs: List<T>, item: T) -> Bool"),
    (Kind::Fn, "find(text: String, part: String) -> Option<Int>"),
    (Kind::Fn, "find(xs: List<T>, item: T) -> Option<Int>"),
    (Kind::Fn, "min(a: Int, b: Int) -> Int"),
    (Kind::Fn, "min(a: Float, b: Float) -> Float"),
    (Kind::Fn, "min(a: String, b: String) -> String"),
    (Kind::Fn, "max(a: Int, b: Int) -> Int"),
    (Kind::Fn, "max(a: Float, b: Float) -> Float"),
    (Kind::Fn, "max(a: String, b: String) -> String"),
    (Kind::Fn, "abs(x: Int) -> Int"),
    (Kind::Fn, "abs(x: Float) -> Float"),
    (Kind::Fn, "to_float(n: Int) -> Float"),
    (Kind::Fn, "to_int(x: Float) -> Int"),
    (Kind::Fn, "sqrt(x: Float) -> Float"),
    (Kind::Fn, "floor(x: Float) -> Float"),
    (Kind::Fn, "ceil(x: Float) -> Float"),
    (Kind::Fn, "trunc(x: Float) -> Float"),
    (Kind::Fn, "round(x: Float) -> Float"),
    (Kind::Fn, "fixed(x: Float; digits: Int) -> String"),
    (Kind::Fn, "wrap_add(a: Int, b: Int) -> Int"),
    (Kind::Fn, "wrap_sub(a: Int, b: Int) -> Int"),
    (Kind::Fn, "wrap_mul(a: Int, b: Int) -> Int"),
    (Kind::Fn, "bit_and(a: Int, b: Int) -> Int"),
    (Kind::Fn, "bit_or(a: Int, b: Int) -> Int"),
    (Kind::Fn, "bit_xor(a: Int, b: Int) -> Int"),
    (Kind::Fn, "bit_not(a: Int) -> Int"),
    (Kind::Fn, "shift_left(a: Int, count: Int) -> Int"),
    (Kind::Fn, "shift_right(a: Int, count: Int) -> Int"),
    (Kind::Fn, "parse_int(text: String) -> Option<Int>"),
    (Kind::Fn, "parse_float(text: String) -> Option<Float>"),
    (Kind::Fn, "code_at(text: String, i: Int) -> Int"),
    (Kind::Fn, "from_code(n: Int) -> String"),
    (Kind::Fn, "split(text: String, sep: String) -> List<String>"),
    (Kind::Fn, "chars(text: String) -> List<String>"),
    (Kind::Fn, "lines(text: String) -> List<String>"),
    (Kind::Fn, "words(text: String) -> List<String>"),
    (Kind::Fn, "join(parts: List<String>, sep: String) -> String"),
    (Kind::Fn, "repeat(text: String, n: Int) -> String"),
    (Kind::Fn, "pad_left(text: String, width: Int) -> String"),
    (Kind::Fn, "pad_right(text: String, width: Int) -> String"),
    (Kind::Fn, "trim(text: String) -> String"),
    (Kind::Fn, "trim_start(text: String) -> String"),
    (Kind::Fn, "trim_end(text: String) -> String"),
    (Kind::Fn, "starts_with(text: String, part: String) -> Bool"),
    (Kind::Fn, "ends_with(text: String, part: String) -> Bool"),
    (
        Kind::Fn,
        "replace(text: String, old: String, new: String) -> String",
    ),
    (Kind::Fn, "upper(text: String) -> String"),
    (Kind::Fn, "lower(text: String) -> String"),
    (Kind::Fn, "append(xs: inout List<T>, item: T)"),
    (Kind::Fn, "extend(xs: inout List<T>, more: List<T>)"),
    (Kind::Fn, "pop(xs: inout List<T>) -> T"),
    (Kind::Fn, "insert(xs: inout List<T>, item: T; at: Int)"),
    (Kind::Fn, "remove_at(xs: inout List<T>, i: Int) -> T"),
    (Kind::Fn, "swap(xs: inout List<T>, i: Int, j: Int)"),
    (Kind::Fn, "reverse(xs: inout List<T>)"),
    (Kind::Fn, "sort(xs: inout List<Int>)"),
    (Kind::Fn, "sort(xs: inout List<Float>)"),
    (Kind::Fn, "sort(xs: inout List<String>)"),
    (
        Kind::Fn,
        "sort_by(xs: inout List<T>, before: fn(T, T) -> Bool)",
    ),
    (Kind::Fn, "list_of(item: T; count: Int) -> List<T>"),
    (Kind::Fn, "get(m: Map<K, V>, key: K) -> Option<V>"),
    (Kind::Fn, "has(m: Map<K, V>, key: K) -> Bool"),
    (Kind::Fn, "remove(m: inout Map<K, V>, key: K)"),
    (Kind::Fn, "keys(m: Map<K, V>) -> List<K>"),
    (Kind::Fn, "values(m: Map<K, V>) -> List<V>"),
    (Kind::Fn, "unwrap_or(opt: Option<T>, fallback: T) -> T"),
    (Kind::Fn, "ok_or(opt: Option<T>, error: E) -> Result<T, E>"),
    (Kind::Fn, "assert(cond: Bool)"),
    (Kind::Fn, "unreachable() -> T"),
];

/// The name a row defines.
pub fn name(row: &'static str) -> &'static str {
    let end = row.find(|c: char| !c.is_alphanumeric() && c != '_');
    &row[..end.unwrap_or(row.len())]
}

/// How many type arguments a built-in type takes; a type of the program takes none.
pub fn arity(type_name: &str) -> usize {
    match type_name {
        "List" | "Option" | "Effect" => 1,
        "Map" | "Result" => 2,
        _ => 0,
    }
}
