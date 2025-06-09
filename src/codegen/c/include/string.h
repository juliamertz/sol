#include <stdint.h>

typedef struct Str {
    List buff;
} Str;

Str str_alloc(size_t size) {
    Str str;
    str.buff = list_alloc(sizeof(char), size);
    return str;
}

size_t str_len(Str *self) {
   return self->buff.len;
}

void str_push_ch(Str *self, char ch) {
    list_push(&self->buff, &ch);
}

void str_push_chars(Str *self, char *text, size_t len) {
    list_extend(&self->buff + self->buff.len, text, sizeof(char), len);
}

void str_push(Str *self, Str *other) {
    list_extend(&self->buff, GC_OBJECT(other->buff.header), sizeof(char), other->buff.len);
}
