#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define GC_ZERO_BEFORE_ALLOC

typedef struct gc_header {
    size_t ref_count;
    size_t size;
    void (*drop)(void*);
} gc_header;

#define GC_HEADER(ptr) ((gc_header*)((char*)(ptr) - sizeof(gc_header)))

#define GC_OBJECT(header) ((void*)((char*)(header) + sizeof(gc_header)))

void* gc_alloc(size_t size) {
    gc_header* header = malloc(sizeof(gc_header) + size);
    if (!header) {
       fprintf(stderr, "Out of memory");
       exit(1);
    }

    #ifdef GC_ZERO_BEFORE_ALLOC
        memset(GC_OBJECT(header), 0, size);
    #endif

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

void gc_release(void* ptr) {
    if (!ptr) return;

    gc_header* header = GC_HEADER(ptr);
    header->ref_count -= 1;

    if (header->ref_count == 0) {
        if (header->drop) header->drop(ptr);
        free(header);
    }
}
