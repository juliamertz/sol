; keywords
[
  "func"
  "return"
  "if"
  "then"
  "end"
  "use"
  "let"
  "extern"
  "struct"
] @keyword

; operators
[
  "<"
  "+"
  "-"
  "*"
  "/"
  "and"
  "or"
  "="
  "=="
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
((type) @type 
  (#set! "priority" 110))

; function names
(function_def 
  (identifier) @function
  (#set! "priority" 110))

(call (identifier) @function.call)

; parameters
(param (identifier) @variable.parameter)

; variables
(identifier) @variable

; literals
(number) @number
(string) @string

(comment) @comment

