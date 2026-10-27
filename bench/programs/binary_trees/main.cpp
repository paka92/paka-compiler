// Build and walk many full binary trees of growing depth.

#include <cstdint>
#include <cstdio>
#include <memory>

struct Node;
using Tree = std::unique_ptr<Node>;  // null is a leaf

struct Node {
    Tree left, right;
};

static Tree make(int64_t depth) {
    if (depth == 0) return nullptr;
    return std::make_unique<Node>(make(depth - 1), make(depth - 1));
}

static int64_t count(const Tree& t) {
    return t ? 1 + count(t->left) + count(t->right) : 0;
}

int main() {
    int64_t total = 0;
    for (int64_t depth = 4, trees = 262144; depth <= 18; depth += 2, trees /= 4)
        for (int64_t i = 0; i < trees; i++) total += count(make(depth));
    std::printf("%lld\n", static_cast<long long>(total));
}
