; keywords
[
  "func"
  "return"
  "if"
  "then"
  "end"
  "use"
] @keyword

; types
(type) @type

; function names
(function_def
  name: (identifier) @function)

(call
  function: (identifier) @function.call)

; parameters
(param
  (identifier) @variable.parameter)

; variables
(identifier) @variable

; literals
(number) @number
(string) @string

; operators
[
  "<"
  "+"
  "-"
  "*"
  "/"
] @operator

; punctuation
[
  "("
  ")"
  ":"
  ","
  ";"
] @punctuation.delimiter

; return type arrow
"->" @operator
