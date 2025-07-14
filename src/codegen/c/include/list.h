#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct List {
    size_t capacity;
    size_t len;
    size_t item_size;
    gc_header *header;
} List;

List list_alloc(size_t item_size, size_t capacity) {
    void *obj = gc_alloc(item_size * capacity);
    List list = {
        .header = GC_HEADER(obj),
        .capacity = capacity,
        .len = 0,
        .item_size = item_size
    };
    return list;
}

void *list_get(List *self, size_t idx) {
    if (idx >= self->len) {
        return NULL;
    }
    void *obj = GC_OBJECT(self->header);
    return (char *)obj + idx * self->item_size;
}

void list_push(List *self, void *item) {
    if (self->len >= self->capacity) {
        size_t new_capacity = self->capacity * 2;
        void *old_obj = GC_OBJECT(self->header);
        void *new_obj = gc_realloc(old_obj, self->item_size * new_capacity);
        if (!new_obj) {
            return;
        }
        self->capacity = new_capacity;
        self->header = GC_HEADER(new_obj);
    }
    void *obj = GC_OBJECT(self->header);
    void *target = (char *)obj + self->len * self->item_size;
    memcpy(target, item, self->item_size);
    self->len += 1;
}

void list_pop(List *self, void *dst) {
    if (self->len == 0) {
        return;
    }
    void *obj = GC_OBJECT(self->header);
    void *ptr = (char *)obj + (self->len - 1) * self->item_size;
    if (dst) {
        memcpy(dst, ptr, self->item_size);
    }
    memset(ptr, 0, self->item_size);
    self->len -= 1;
}

void list_extend(List *self, void *items, size_t len) {
    while (self->len + len > self->capacity) {
        size_t new_capacity = self->capacity * 2;
        void *old_obj = GC_OBJECT(self->header);
        void *new_obj = gc_realloc(old_obj, self->item_size * new_capacity);
        if (!new_obj) {
            return;
        }
        self->capacity = new_capacity;
        self->header = GC_HEADER(new_obj);
    }

    char *obj = GC_OBJECT(self->header);
    memcpy(obj + (self->len * self->item_size), items, self->item_size * len);
    self->len += len;
}


#define list_push_rval(list, x) \
    do { \
        __auto_type temp = x; \
        list_push(list, &temp); \
    } while (0)
