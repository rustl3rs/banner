const
  newline = '\n',
  terminator = choice(newline, ';'),
  task_declaration = 'task'

module.exports = grammar({
  name: 'banner',

  externals: $ => [
    // $._string_content,
    $.raw_string_literal,
  ],

  extras: $ => [
    $.comment,
    /\s/
  ],

  rules: {
    // TODO: add the actual grammar rules
    source_file: $ => repeat($._definition),

    _definition: $ => choice(
      $.task_definition
      // TODO: other kinds of definitions
    ),

    task_definition: $ => seq(
      'task',
      $.identifier,
      $.parameter_list,
      $.block
    ),

    identifier: $ => /[a-z-_]+/,

    // image/tag:v1.0.0
    // 123.123.123.123:123/image/tag:v1.0.0
    // your-domain.com/image/tag
    // your-domain.com/image/tag:v1.1.1-patch1
    // image/tag
    // image
    // image:v1.1.1-patch
    // ubuntu@sha256:45b23dee08af5e43a7fea6c4cf9c25ccf269ee113168c19722f87876677c5cb2
    // etc...
    // https://stackoverflow.com/a/62964157/186184
    image_identifier: $ => /(([a-z0-9]|[a-z0-9][a-z0-9\\-]*[a-z0-9])\\.)*([a-z0-9]|[a-z0-9][a-z0-9\\-]*[a-z0-9])(:[0-9]+\\\/)?(?:[0-9a-z-]+[/@])(?:([0-9a-z-]+))[/@]?(?:([0-9a-z-]+))?(?::[a-z0-9\\.-]+)?/,
    // image_identifier: $ => /[a-zA-Z/-0-9\\]+/,
    execute_command: $ => /[a-zA-Z/-0-9\\\s]+/,

    parameter_list: $ => seq(
      '(',
      seq('image', ':', $.image_identifier),
      ',',
      seq('execute', ":", "\"", $.execute_command, "\""),
      ')'
    ),

    lang_identifier: $ => /\w{1,20}/,
    execution_context: $ => /(.|\s)*/,
    block: $ => seq(
      '{',
      optional($.raw_string_literal),
      '}'
    ),

    // http://stackoverflow.com/questions/13014947/regex-to-match-a-c-style-multiline-comment/36328890#36328890
    comment: $ => token(choice(
      seq('//', /.*/),
      seq(
        '/*',
        /[^*]*\*+([^/*][^*]*\*+)*/,
        '/'
      )
    ))
  }
});
