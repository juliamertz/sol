#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct List {
  size_t capacity;
  size_t len;
  size_t item_size;
  gc_header *header;
} List;

List list_alloc(size_t size, size_t capacity) {
  void *obj = gc_alloc(size * capacity);
  List list = {
      .header = GC_HEADER(obj),
      .capacity = capacity,
      .len = 0,
  };

  return list;
}

void *list_get(List *self, size_t idx) {
  void *obj = GC_OBJECT(self->header);
  return (char *)obj + idx * self->item_size;
}

void list_push(List *self, void *item) {
  if (self->len >= self->capacity) {
    size_t new_capacity = self->capacity * 2;

    void *old_obj = GC_OBJECT(self->header);
    void *new_obj = gc_realloc(old_obj, self->item_size * new_capacity);

    self->capacity = new_capacity;
    self->header = GC_HEADER(new_obj);
  }

  void *target = list_get(self, self->len);
  memcpy(target, item, self->item_size);
  self->len += 1;
}

// TODO: return popped element somehow
void list_pop(List *self, void *dst) {
  if (self->len == 0) {
    // return NULL;
  }

  void *ptr = list_get(self, self->len - 1);
  void *item = memcpy(dst, ptr, self->item_size);
  memset(ptr, 0, self->item_size);

  self->len -= 1;
}

void list_extend(List *self, void *items, size_t size, size_t len) {
    // TODO: re-alloc if needed
    char *obj = GC_OBJECT(self->header);
    memcpy(obj + self->len, items, size * len);
    self->len += len;
}
