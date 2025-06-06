#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

#include "gc.h"

typedef struct Array {
  uint32_t capacity;
  uint32_t len;
  int* ptr;
} Array;

Array array_alloc(size_t size, uint32_t capacity) {
  Array array;
  array.ptr = malloc(size * capacity);
  array.capacity = capacity;
  array.len = 0;

  return array;
}

void array_push(Array *self, int item) {
  self->ptr[self->len] = item;
  self->len += 1;
  return;
}

int array_get(Array *self, uint32_t idx) {
    return self->ptr[idx];
}

typedef struct Rgb {
  uint8_t r;
  uint8_t g;
  uint8_t b;
} Rgb;

int main() {
  Rgb* color = gc_alloc(sizeof(Rgb));
  color->r = 100;
  color->g = 150;
  color->b = 200;
  printf("color: rgb(%d, %d, %d)\n", color->r, color->g, color->b);

  // gc_header* obj = gc_alloc(sizeof(Animal));

  Array array = array_alloc(sizeof(int), 6);

  array_push(&array, 10);
  array_push(&array, 20);
  array_push(&array, 30);
  array_push(&array, 40);
  array_push(&array, 50);

  int* obj = array.ptr;
  printf("%d\n", array_get(&array, 0));
  printf("%d\n", array_get(&array, 1));
  printf("%d\n", array_get(&array, 2));
  printf("%d\n", array_get(&array, 3));
  printf("%d\n", array_get(&array, 4));
  // printf("%d\n", array_get(&array, 5));

  return 0;
}
