// Build and walk many full binary trees of growing depth.

struct Node {
    left: Tree,
    right: Tree,
}

type Tree = Option<Box<Node>>;

fn make(depth: i64) -> Tree {
    if depth == 0 {
        return None;
    }
    Some(Box::new(Node {
        left: make(depth - 1),
        right: make(depth - 1),
    }))
}

fn count(t: &Tree) -> i64 {
    match t {
        None => 0,
        Some(n) => 1 + count(&n.left) + count(&n.right),
    }
}

fn main() {
    let mut total: i64 = 0;
    let mut depth = 4;
    let mut trees = 262144;
    while depth <= 18 {
        for _ in 0..trees {
            total += count(&make(depth));
        }
        depth += 2;
        trees /= 4;
    }
    println!("{total}");
}
