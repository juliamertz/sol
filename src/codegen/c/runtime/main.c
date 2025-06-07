#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "gc.h"

// typedef struct Array {
//   size_t capacity;
//   size_t len;
//   gc_header *header;
// } Array;

// Array array_alloc(size_t size, size_t capacity) {
//   Array array;
//   array.header = gc_alloc(size * capacity);
//   array.capacity = capacity;
//   array.len = 0;

//   void* object = GC_OBJECT(array.header);


//   return array;
// }

// void array_push(Array *self, void *item) {
//   void *object = GC_OBJECT(self->header);
//   void *target = (char *)object + self->len * self->header->size;
//   memcpy(target, item, self->header->size);
//   self->len += 1;
// }

// void *array_get(Array *self, size_t idx) {
//   void *object = GC_OBJECT(self->header);
//   return (void *)object + idx * self->header->size;
// }

typedef struct Rgb {
  uint8_t r;
  uint8_t g;
  uint8_t b;
} Rgb;

int main() {
  Rgb *color = gc_alloc(sizeof(Rgb));
  color->r = 100;
  color->g = 150;
  color->b = 200;
  printf("color: rgb(%d, %d, %d)\n", color->r, color->g, color->b);

  Rgb *color2 = gc_retain(color);
  gc_release(color);

  printf("color: rgb(%d, %d, %d)\n", color2->r, color2->g, color2->b);
  Rgb *color3 = gc_retain(color);
  gc_release(color2);
  gc_release(color3);

  gc_header* header = GC_HEADER(color3);
  printf("refs: %zu\n", header->ref_count);

  printf("color: rgb(%d, %d, %d)\n", color2->r, color2->g, color2->b);

  // gc_header* obj = gc_alloc(sizeof(Animal));

  // Array array = array_alloc(sizeof(int), 6);

  // array_push(&array, (void*)10);
  // array_push(&array, (void*)20);
  // array_push(&array, (void*)30);
  // array_push(&array, (void*)40);
  // array_push(&array, (void*)50);

  // int *obj = GC_OBJECT(array.header);
  // printf("%d\n", (int)array_get(&array, 0));
  // printf("%d\n", (int)array_get(&array, 1));
  // printf("%d\n", (int)array_get(&array, 2));
  // printf("%d\n", (int)array_get(&array, 3));
  // printf("%d\n", (int)array_get(&array, 4));

  return 0;
}
