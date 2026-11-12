/* The test library the foreign host tests call (paka/tests/foreign.rs). */
#include <stdbool.h>
#include <stdint.h>

int64_t probe_sum(const char *text, int64_t text_len);
double probe_scale(double x, int64_t by, bool negate);
bool probe_odd(int64_t n);
int64_t probe_count(void);
void probe_crash(void);
void probe_stall(void);
