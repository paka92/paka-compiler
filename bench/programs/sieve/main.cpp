// Count the primes below N with a sieve of flags, for a run of N.

#include <cstdint>
#include <cstdio>
#include <vector>

static int64_t count_primes(int64_t n) {
    std::vector<char> prime(n, 1);
    int64_t count = 0;
    for (int64_t i = 2; i < n; i++) {
        if (prime[i]) {
            count += 1;
            for (int64_t j = i * i; j < n; j += i) prime[j] = 0;
        }
    }
    return count;
}

int main() {
    int64_t total = 0;
    for (int64_t r = 0; r < 80; r++) total += count_primes(10000000 + r);
    std::printf("%lld\n", static_cast<long long>(total));
}
