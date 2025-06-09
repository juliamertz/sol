#include <stdint.h>
#include <stdio.h>

#include "gc.h"
#include "array.h"

typedef struct Rgb {
  uint8_t r;
  uint8_t g;
  uint8_t b;
} Rgb;

int main() {
  Array array = array_alloc(sizeof(int), 6);

  printf("ref count: %zu\n", array.header->ref_count);

  void *obj = GC_OBJECT(array.header);

  int i = 0;
  while (i < 6) {
      int val = i * 100;
      array_push(&array, &val);
      printf("element at %d: %d\n", i, *(int *)array_get(&array, i));
      i++;
  }

  return 0;
}
