#include <stdint.h>
#include <stdio.h>

#include "gc.h"
#include "array.h"

int main() {
  Array array = array_alloc(sizeof(int), 5);

  void *obj = GC_OBJECT(array.header);

  int i = 0;
  while (i < 50) {
      int val = i * 100;
      array_push(&array, &val);
      printf("element at %d: %d\n", i, *(int *)array_get(&array, i));
      i++;
  }

  return 0;
}
