#include "calc.h"

static int64_t total;

int64_t calc_add(int64_t a) { return total += a; }

double calc_half(double x) { return x / 2; }

bool calc_longer(const char *text, int64_t text_len, int64_t than) {
  (void)text;
  return text_len > than;
}

void calc_reset(void) { total = 0; }
