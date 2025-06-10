; keywords
[
  "func"
  "return"
  "if"
  "then"
  "end"
  "use"
  "extern"
] @keyword

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

; types
(type) @type

; function names
(function_def (identifier) @function)

(call (identifier) @function.call)

; parameters
(param (identifier) @variable.parameter)

; variables
(identifier) @variable

; literals
(number) @number
(string) @string

(comment) @comment

