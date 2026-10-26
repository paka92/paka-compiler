#include "crash.h"
#include <stdlib.h>

int64_t crash_count(void) {
  static int64_t calls;
  return ++calls;
}

void crash_now(void) { abort(); }
