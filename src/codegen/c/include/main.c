#include <stdio.h>

#include "gc.h"
#include "array.h"

int main() {
  Array arr = array_alloc(sizeof(int), 5);

  for (int i =0; i < 50; i++) {
      int val = i * 100;
      array_push(&arr, &val);
      printf("element at %d: %d\n", i, *(int *)array_get(&arr, i));
  }

  return 0;
}
