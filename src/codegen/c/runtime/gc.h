#include <stdio.h>
#include <stdlib.h>

typedef struct gc_header {
    size_t ref_count;
    size_t size;
    void (*drop)(void*);
} gc_header;

#define GC_HEADER(ptr) ((gc_header*)((char*)(ptr) - sizeof(gc_header)))

#define GC_OBJECT(header) ((void*)((char*)(header) + sizeof(gc_header)))

void* gc_alloc(size_t size) {
    gc_header* header = calloc(sizeof(gc_header), size);
    if (!header) {
       fprintf(stderr, "Out of memory");
       exit(1);
    }

    header->ref_count = 1;
    header->size = size;
    header->drop = NULL;

    return GC_OBJECT(header);
}

void* gc_retain(void* ptr) {
    if (!ptr)  return NULL;

    gc_header* header = GC_HEADER(ptr);
    header->ref_count += 1;
    return ptr;
}

void gc_drop(void* ptr) {
    if (!ptr) return;

    gc_header* header = GC_HEADER(ptr);
    header->ref_count -= 1;

    if (header->ref_count == 0) {
        if (header->drop) header->drop(ptr);
        free(header);
    }
}
