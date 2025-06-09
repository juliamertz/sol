#include <stdio.h>
#include <stdlib.h>

typedef struct Array {
  size_t capacity;
  size_t len;
  size_t item_size;
  gc_header *header;
} Array;

Array array_alloc(size_t size, size_t capacity) {
  Array array;
  void *obj = gc_alloc(size * capacity);
  array.header = GC_HEADER(obj);
  array.item_size = size;
  array.capacity = capacity;
  array.len = 0;

  return array;
}

void *array_get(Array *self, size_t idx) {
  void *obj = GC_OBJECT(self->header);
  return (char *)obj + idx * self->item_size;
}

void array_push(Array *self, void *item) {
  if (self->len >= self->capacity) {
    size_t new_capacity = self->capacity * 2;

    void *old_obj = GC_OBJECT(self->header);
    void *new_obj = gc_realloc(old_obj, self->item_size * new_capacity);

    self->capacity = new_capacity;
    self->header = GC_HEADER(new_obj);
  }

  void *target = array_get(self, self->len);
  memcpy(target, item, self->item_size);
  self->len += 1;
}
