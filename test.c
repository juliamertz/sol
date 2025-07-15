#include <stdint.h>
#include <stdio.h>
#include <unistd.h>

#include </Users/julia/projects/2025/sol/src/codegen/c/include/gc.h>
#include </Users/julia/projects/2025/sol/src/codegen/c/include/list.h>

int main() {
  List list = ({
    List l=list_alloc(sizeof( uint8_t), 10);
    uint8_t item = 25;
    list_push(&l, &item);
    l;
  });
  uint8_t first = *(uint8_t*)list_get(&list, 0);
  printf("item: %d", first);
  return 0;
}
