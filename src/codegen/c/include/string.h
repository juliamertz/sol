#include <stdint.h>
typedef struct Str {
    Array buff;
} Str;

Str str_alloc() {
    Str str;
    str.buff = array_alloc(sizeof(uint8_t), 40);
    return str;
}

void str_push_ch(Str *self, char ch) {
    array_push(&self->buff, &ch);
}

void str_push(Str *self, char *text) {

}

// void str_push(Str *self, char *text) {
// }
