#include <stdio.h>
#include <stdbool.h>

int main() {
  // if else expressions in c
  
  // when if else is found to have a return type other than unit (equivalent to void):
  int val = ({
    int tmp;
    if (true) {
      // the last node should always be an expression when not returning unit!
      // that way we can assign it to our tmp return variable
      tmp = 20;
    }
    else {
      tmp = 10;
    }
    tmp; 
  });

  printf("val: %d\n", val);

  // when it solves to unit:
  ({
  if (true) {
      printf("yes!\n");
  }
  else {
      printf("no :(\n");
  }
  });


  return 0;
}
