module.exports = grammar({
  name: 'newlang',

  rules: {
    source_file: $ => repeat($._statement),

    _statement: $ => choice(
      $.use_stmt,
      $.function_def,
      $.return_stmt,
      $.expression_stmt,
      $.if_stmt
    ),

    use_stmt: $ => seq('use', $.identifier, ';'),

    function_def: $ => seq(
      'func',
      $.identifier,
      $.param_list,
      '->',
      $.type,
      $.block,
      'end'
    ),

    param_list: $ => seq(
      '(',
      optional(commaSep($.param)),
      ')'
    ),

    param: $ => seq($.identifier, ':', $.type),

    type: $ => $.identifier,

    block: $ => repeat1($._statement),

    if_stmt: $ => seq(
      'if',
      $.expression,
      'then',
      $.block,
      'end',
      optional(';')
    ),

    return_stmt: $ => seq('return', $.expression, ';'),

    expression_stmt: $ => seq($.expression, ';'),

    expression: $ => choice(
      $.call,
      $.binary_expr,
      $.identifier,
      $.number,
      $.string
    ),

    binary_expr: $ => prec.left(seq(
      $.expression,
      choice('<', '+', '-', '*', '/'),
      $.expression
    )),

    call: $ => seq(
      $.identifier,
      '(',
      optional(commaSep($.expression)),
      ')'
    ),

    identifier: $ => /[a-zA-Z_][a-zA-Z0-9_]*/,

    number: $ => /\d+/,

    string: $ => /"[^"]*"/
  }
});

function commaSep(rule) {
  return seq(rule, repeat(seq(',', rule)));
}
