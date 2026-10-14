#include <stdio.h> // io
#define MAX(a, b) \
  ((a) > (b) ? (a) : (b)) // pick
/* header
   comment */
int main(void) {
  printf("// not a comment /* either */\n"); /* trailing */
  char q = '"'; // quote char
  return 0; /* outer /* inner */ still outer */
}
