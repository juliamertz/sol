module.exports = grammar({
	name: "sol",
	extras: ($) => [/[ \t\r]/, $.comment],

	precedences: ($) => [
		[
			"chain",
			"index",
			"construct",
			"call",
			"unary",
			"product",
			"sum",
			"cmp",
			"eq",
			"and_or",
		],
	],

	rules: {
		source_file: ($) => repeat(choice($._item, $._newline)),

		_newline: ($) => /\n/,

		_item: ($) =>
			choice($.use_stmt, $.function_def, $.struct_def, $.impl_def),

		use_stmt: ($) => seq("use", optional("extern"), $.identifier),

		function_def: ($) =>
			choice(
				seq(
					"extern",
					"func",
					$.identifier,
					$.param_list,
					"->",
					$.type,
				),
				seq(
					"func",
					$.identifier,
					$.param_list,
					"->",
					$.type,
					repeat(choice($._statement, $._newline)),
					"end",
				),
			),

		struct_def: ($) =>
			seq(
				"struct",
				$.identifier,
				"=",
				repeat(choice($.field, $._newline)),
				"end",
			),

		impl_def: ($) =>
			seq(
				"impl",
				$.identifier,
				"=",
				repeat(choice($.function_def, $._newline)),
				"end",
			),

		field: ($) => seq($.identifier, ":", $.type),

		param_list: ($) => seq("(", optional(commaSep($.param)), ")"),

		param: ($) => seq($.identifier, ":", $.type),

		type: ($) => seq($.identifier, optional($.array_suffix)),

		array_suffix: ($) => token.immediate("[]"),

		_statement: ($) =>
			choice($.let_stmt, $.return_stmt, $.expression_stmt, $.comment),

		let_stmt: ($) =>
			seq(
				"let",
				$.identifier,
				optional(seq(":", $.type)),
				"=",
				$._expression,
				$._newline,
			),

		return_stmt: ($) => seq("return", $._expression, $._newline),

		expression_stmt: ($) => seq($._expression, $._newline),

		_expression: ($) =>
			choice(
				$.binary_expr,
				$.unary,
				$.call_expr,
				$.index_expr,
				$.member_access,
				$.constructor,
				$.if_expr,
				$.list,
				$.identifier,
				$.number,
				$.string,
			),

		binary_expr: ($) =>
			choice(
				prec.left(
					"and_or",
					seq($._expression, choice("and", "or"), $._expression),
				),
				prec.left("eq", seq($._expression, "==", $._expression)),
				prec.left(
					"cmp",
					seq($._expression, choice("<", ">"), $._expression),
				),
				prec.left(
					"sum",
					seq($._expression, choice("+", "-"), $._expression),
				),
				prec.left(
					"product",
					seq($._expression, choice("*", "/"), $._expression),
				),
			),

		unary: ($) =>
			prec("unary", seq(choice("-", "!"), $._expression)),

		call_expr: ($) =>
			prec(
				"call",
				seq($._expression, "(", optional(commaSep($._expression)), ")"),
			),

		index_expr: ($) =>
			prec("index", seq($._expression, "[", $._expression, "]")),

		member_access: ($) =>
			prec("chain", seq($._expression, ".", $.identifier)),

		constructor: ($) =>
			prec(
				"construct",
				seq(
					$.identifier,
					"{",
					repeat(choice($.value_field, $._newline)),
					"}",
				),
			),

		value_field: ($) => seq($.identifier, ":", $._expression),

		if_expr: ($) =>
			seq(
				"if",
				$._expression,
				"then",
				repeat(choice($._statement, $._newline)),
				optional(
					seq("else", repeat(choice($._statement, $._newline))),
				),
				"end",
			),

		list: ($) => seq("[", optional(commaSep($._expression)), "]"),

		identifier: ($) => /[a-zA-Z_][a-zA-Z0-9_]*/,
		number: ($) => /\d+/,
		string: ($) => /"[^"]*"/,
		comment: ($) => /--[^\n]*/,
	},
});

function commaSep(rule) {
	return seq(rule, repeat(seq(",", rule)));
}
