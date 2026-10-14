#include <stdio.h> // io
/* header
   comment */
int main(void) {
  printf("// not a comment /* either */\n"); /* trailing */
  char q = '"'; // quote char
  return 0; /* outer /* inner */ still outer */
}
