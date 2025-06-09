#include <stdio.h>

#include "gc.h"

#include "array.h"

#include "string.h"

int main() {
  Array arr = array_alloc(sizeof(int), 5);

  for (int i = 0; i < 50; i++) {
    int val = i * 100;
    array_push(&arr, &val);
    printf("element at %d: %d\n", i, *(int *)array_get(&arr, i));
  }

  int last;
  array_pop(&arr, &last);
  printf("last: %d\n", last);
  array_pop(&arr, &last);
  printf("last: %d\n", last);
  array_pop(&arr, &last);
  printf("last: %d\n", last);
  array_pop(&arr, &last);
  printf("last: %d\n", last);
  array_pop(&arr, &last);
  printf("last: %d\n", last);


  Str str = str_alloc();
  str_push_ch(&str, 'h');
  str_push_ch(&str, 'e');
  str_push_ch(&str, 'l');
  str_push_ch(&str, 'l');
  str_push_ch(&str, 'o');
  str_push_ch(&str, ' ');
  str_push_ch(&str, 'w');
  str_push_ch(&str, 'o');
  str_push_ch(&str, 'r');
  str_push_ch(&str, 'l');
  str_push_ch(&str, 'd');

  printf("str: %s", GC_OBJECT(str.buff.header));

  return 0;
}
