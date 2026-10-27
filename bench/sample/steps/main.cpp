// Total Collatz steps for 1 to 9,999.

#include <cstdint>
#include <cstdio>

static int64_t steps(int64_t x) {
    int64_t n = 0;
    while (x != 1) {
        x = x % 2 == 0 ? x / 2 : 3 * x + 1;
        n += 1;
    }
    return n;
}

int main() {
    int64_t total = 0;
    for (int64_t i = 1; i < 10000; i++) total += steps(i);
    std::printf("%lld\n", static_cast<long long>(total));
}
