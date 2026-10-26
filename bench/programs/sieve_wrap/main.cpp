// Count the primes below N with a sieve of flags, for a run of N. Wrapping arithmetic.

#include <cstdint>
#include <cstdio>
#include <vector>

static uint64_t count_primes(uint64_t n) {
    std::vector<char> prime(n, 1);
    uint64_t count = 0;
    for (uint64_t i = 2; i < n; i++) {
        if (prime[i]) {
            count += 1;
            for (uint64_t j = i * i; j < n; j += i) prime[j] = 0;
        }
    }
    return count;
}

int main() {
    uint64_t total = 0;
    for (uint64_t r = 0; r < 80; r++) total += count_primes(10000000 + r);
    std::printf("%llu\n", static_cast<unsigned long long>(total));
}
