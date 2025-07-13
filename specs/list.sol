use stdio;

extern func printf(format: Str) -> Int;
extern func list_push(list: List) -> Int;

func main() -> Int
  let items : Int[] = [10,10];
  list_push(items, 20);
end
