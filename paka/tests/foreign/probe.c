#include "probe.h"
#include <stdio.h>
#include <unistd.h>

/* The byte sum of `text`, or -1 when the zero after its end is missing. */
int64_t probe_sum(const char *text, int64_t text_len) {
  int64_t sum = 0;
  for (int64_t i = 0; i < text_len; i++)
    sum += (unsigned char)text[i];
  return text[text_len] == 0 ? sum : -1;
}

double probe_scale(double x, int64_t by, bool negate) { return (negate ? -x : x) * (double)by; }

bool probe_odd(int64_t n) { return n % 2 != 0; }

/* Counts its calls; it prints and reads, which must not reach the pipe. */
int64_t probe_count(void) {
  static int64_t calls;
  printf("count\n");
  fflush(stdout);
  return getchar() == EOF ? ++calls : -1;
}

void probe_crash(void) { *(volatile int *)0 = 1; }

void probe_stall(void) { pause(); }
