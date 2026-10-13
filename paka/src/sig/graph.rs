//! Strongly connected components, by Tarjan's pass with an explicit stack.

const NONE: usize = usize::MAX;

struct Tarjan<'e> {
    edges: &'e [Vec<usize>],
    index: Vec<usize>,
    low: Vec<usize>,
    done: Vec<bool>,
    next: usize,
    stack: Vec<usize>,
    /// Each node being walked and the next of its edges to follow.
    work: Vec<(usize, usize)>,
    out: Vec<Vec<usize>>,
}

impl Tarjan<'_> {
    fn enter(&mut self, v: usize) {
        self.index[v] = self.next;
        self.low[v] = self.next;
        self.next += 1;
        self.stack.push(v);
        self.work.push((v, 0));
    }

    fn leave(&mut self, v: usize) {
        if let Some(&(parent, _)) = self.work.last() {
            self.low[parent] = self.low[parent].min(self.low[v]);
        }
        if self.low[v] != self.index[v] {
            return;
        }
        let from = self.stack.iter().rposition(|&w| w == v).unwrap();
        let members = self.stack.split_off(from);
        members.iter().for_each(|&w| self.done[w] = true);
        self.out.push(members);
    }

    fn walk(&mut self, root: usize) {
        self.enter(root);
        while let Some(&(v, k)) = self.work.last() {
            let Some(&w) = self.edges[v].get(k) else {
                self.work.pop();
                self.leave(v);
                continue;
            };
            self.work.last_mut().unwrap().1 += 1;
            if self.index[w] == NONE {
                self.enter(w);
            } else if !self.done[w] {
                self.low[v] = self.low[v].min(self.index[w]);
            }
        }
    }
}

/// The components of the graph whose node `i` points to `edges[i]`, each after every component it
/// points to.
pub(super) fn components(edges: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let n = edges.len();
    let mut t = Tarjan {
        edges,
        index: vec![NONE; n],
        low: vec![0; n],
        done: vec![false; n],
        next: 0,
        stack: Vec::new(),
        work: Vec::new(),
        out: Vec::new(),
    };
    for v in 0..n {
        if t.index[v] == NONE {
            t.walk(v);
        }
    }
    t.out
}
