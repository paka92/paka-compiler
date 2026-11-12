// Breadth-first shortest paths on a random graph held as lists of neighbours, from several sources.

#include <cstdint>
#include <cstdio>
#include <vector>

using Graph = std::vector<std::vector<int64_t>>;

static Graph graph(int64_t n, int64_t edges) {
    Graph adj(n);
    uint64_t s = 1;
    auto next = [&] {
        s = s * 6364136223846793005u + 1442695040888963407u;
        return int64_t((s >> 11) & 9007199254740991u) % n;
    };
    for (int64_t e = 0; e < edges; e++) {
        int64_t a = next(), b = next();
        adj[a].push_back(b);
        adj[b].push_back(a);
    }
    return adj;
}

static int64_t distances(const Graph& adj, int64_t source) {
    std::vector<int64_t> dist(adj.size(), -1), queue(adj.size(), 0);
    dist[source] = 0;
    queue[0] = source;
    int64_t head = 0, tail = 1, total = 0;
    while (head < tail) {
        int64_t v = queue[head++];
        total += dist[v];
        for (int64_t w : adj[v])
            if (dist[w] < 0) {
                dist[w] = dist[v] + 1;
                queue[tail++] = w;
            }
    }
    return total;
}

int main() {
    Graph adj = graph(1000000, 3000000);
    int64_t total = 0;
    for (int64_t source = 0; source < 8; source++) total += distances(adj, source);
    std::printf("%lld\n", static_cast<long long>(total));
}
