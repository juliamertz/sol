#include <stdio.h>

#include "gc.h"

#include "list.h"

#include "string.h"

int main() {
  List list = list_alloc(sizeof(int), 5);

  for (int i = 0; i < 50; i++) {
    int val = i * 100;
    list_push(&list, &val);
    printf("element at %d: %d\n", i, *(int *)list_get(&list, i));
  }

  int last;
  list_pop(&list, &last);
  printf("last: %d\n", last);

  Str str = str_alloc(100);
  str_push_chars(&str,  "Hello world", 12);

  str_push_chars(&str,  "aargh!\0", 7);

  printf("str: %s", GC_OBJECT(str.buff.header));

  return 0;
}
