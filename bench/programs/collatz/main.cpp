// The start below N of the longest Collatz chain.

#include <cstdint>
#include <cstdio>

static int64_t chain(int64_t x) {
    int64_t n = 1;
    while (x != 1) {
        x = x % 2 == 0 ? x / 2 : 3 * x + 1;
        n += 1;
    }
    return n;
}

int main() {
    int64_t best = 0, best_start = 0;
    for (int64_t i = 1; i < 9000000; i++) {
        int64_t n = chain(i);
        if (n > best) {
            best = n;
            best_start = i;
        }
    }
    std::printf("%lld\n", static_cast<long long>(best_start));
}
