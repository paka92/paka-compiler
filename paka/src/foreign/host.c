/* The fixed half of a foreign host (docs/decisions/M7-02.md): the pipe loop and the codec the
   generated glue calls. Each request and reply is a frame: an 8-byte length, then that many
   bytes. Numbers are little-endian. A request is a 4-byte function number, then each argument:
   Int and Float 8 bytes, Bool 1 byte, String an 8-byte length and its bytes. A reply is 0 and
   the result (8 bytes, 1 byte or none), or 1 alone for a malformed request. */

#include <errno.h>
#include <fcntl.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

typedef struct {
  unsigned char *at, *end;
  bool bad;
} paka_req;

typedef struct {
  unsigned char bytes[9];
  size_t len;
} paka_rep;

typedef struct {
  const char *ptr;
  int64_t len;
} paka_str;

typedef paka_rep (*paka_call)(paka_req *);

/* Defined by the glue: function `n`, or none. */
static paka_call paka_function(uint32_t n);

static uint64_t paka_take(paka_req *r, size_t n) {
  if (r->bad || (size_t)(r->end - r->at) < n) {
    r->bad = true;
    return 0;
  }
  uint64_t v = 0;
  for (size_t i = 0; i < n; i++)
    v |= (uint64_t)r->at[i] << 8 * i;
  r->at += n;
  return v;
}

static inline int64_t paka_get_int(paka_req *r) { return (int64_t)paka_take(r, 8); }

static inline double paka_get_float(paka_req *r) {
  uint64_t v = paka_take(r, 8);
  double d;
  memcpy(&d, &v, 8);
  return d;
}

static inline bool paka_get_bool(paka_req *r) {
  uint64_t v = paka_take(r, 1);
  r->bad |= v > 1;
  return v;
}

static inline paka_str paka_get_str(paka_req *r) {
  uint64_t len = paka_take(r, 8);
  if (r->bad || (uint64_t)(r->end - r->at) < len) {
    r->bad = true;
    return (paka_str){"", 0};
  }
  /* The bytes move back over their length, which leaves room for the zero after them. */
  unsigned char *s = memmove(r->at - 8, r->at, len);
  s[len] = 0;
  r->at += len;
  return (paka_str){(const char *)s, (int64_t)len};
}

static inline bool paka_done(paka_req *r) { return !r->bad && r->at == r->end; }

static paka_rep paka_put(uint64_t v, size_t n) {
  paka_rep p = {{0}, 1 + n};
  for (size_t i = 0; i < n; i++)
    p.bytes[1 + i] = (unsigned char)(v >> 8 * i);
  return p;
}

static inline paka_rep paka_int(int64_t v) { return paka_put((uint64_t)v, 8); }

static inline paka_rep paka_float(double d) {
  uint64_t v;
  memcpy(&v, &d, 8);
  return paka_put(v, 8);
}

static inline paka_rep paka_bool(bool b) { return paka_put(b, 1); }
static inline paka_rep paka_none(void) { return paka_put(0, 0); }
static paka_rep paka_refused(void) { return (paka_rep){{1}, 1}; }

static bool paka_read(int fd, unsigned char *at, size_t n) {
  while (n > 0) {
    ssize_t got = read(fd, at, n);
    if (got < 0 && errno == EINTR)
      continue;
    if (got <= 0)
      return false;
    at += got;
    n -= (size_t)got;
  }
  return true;
}

static bool paka_write(int fd, const unsigned char *at, size_t n) {
  while (n > 0) {
    ssize_t put = write(fd, at, n);
    if (put < 0 && errno == EINTR)
      continue;
    if (put <= 0)
      return false;
    at += put;
    n -= (size_t)put;
  }
  return true;
}

/* Answers one request from `in` on `out`; false when the caller is gone. */
static bool paka_serve(int in, int out) {
  unsigned char head[8];
  if (!paka_read(in, head, 8))
    return false;
  paka_req r = {head, head + 8, false};
  uint64_t len = paka_take(&r, 8);
  unsigned char *body = len < SIZE_MAX ? malloc(len + 1) : NULL;
  if (!body || !paka_read(in, body, len))
    exit(2);
  r = (paka_req){body, body + len, false};
  uint32_t n = (uint32_t)paka_take(&r, 4);
  paka_call call = r.bad ? NULL : paka_function(n);
  paka_rep p = call ? call(&r) : paka_refused();
  free(body);
  unsigned char frame[8 + sizeof p.bytes];
  for (size_t i = 0; i < 8; i++)
    frame[i] = (unsigned char)((uint64_t)p.len >> 8 * i);
  memcpy(frame + 8, p.bytes, p.len);
  if (!paka_write(out, frame, 8 + p.len))
    exit(2);
  return true;
}

/* The pipe moves aside: the library's own output goes to standard error, its reads find
   nothing. */
int main(void) {
  int in = dup(0), out = dup(1), none = open("/dev/null", O_RDONLY);
  if (in < 0 || out < 0 || none < 0 || dup2(2, 1) < 0 || dup2(none, 0) < 0)
    return 2;
  close(none);
  while (paka_serve(in, out))
    ;
  return 0;
}
