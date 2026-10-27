// Breadth-first shortest paths on a random graph held as lists of neighbours, from several sources.

fn graph(n: usize, edges: usize) -> Vec<Vec<usize>> {
    let mut adj = vec![Vec::new(); n];
    let mut s: i64 = 1;
    let mut next = || {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((s >> 11) & 9007199254740991) as usize % n
    };
    for _ in 0..edges {
        let (a, b) = (next(), next());
        adj[a].push(b);
        adj[b].push(a);
    }
    adj
}

fn distances(adj: &[Vec<usize>], source: usize) -> i64 {
    let mut dist = vec![-1i64; adj.len()];
    let mut queue = vec![0; adj.len()];
    dist[source] = 0;
    queue[0] = source;
    let (mut head, mut tail, mut total) = (0, 1, 0);
    while head < tail {
        let v = queue[head];
        head += 1;
        total += dist[v];
        for &w in &adj[v] {
            if dist[w] < 0 {
                dist[w] = dist[v] + 1;
                queue[tail] = w;
                tail += 1;
            }
        }
    }
    total
}

fn main() {
    let adj = graph(1000000, 3000000);
    let total: i64 = (0..8).map(|source| distances(&adj, source)).sum();
    println!("{total}");
}
