#include <tree_sitter/parser.h>
#include <wctype.h>

/*
  Nicked the this from the Rust Treesitter implementation here:
  https://github.com/tree-sitter/tree-sitter-rust/blob/master/src/scanner.c

  This may diverge over time, but we'll have to see.

  I'd really like to pass back a RAW_STRING_LITTERAL that is broken down into
  START_TAG, LANG, STRING_CONTENT, END_TAG as I want the RAW_STRING_LITERAL
  to behave much like doc comments in many languages. e.g.
  ```bash
  some bash command
  # with comments
  and more bash commands
  possibly(functions) {
      more bash
      ls -lta
  }
  ```

  except with ``` replaced by r####"...."####

  The reason for not using the ``` syntax is because I also recognise that this
  block of text is potentially a script of one of the languages that uses doc
  comments; like rust or python; and that may lead to some pretty gnarly
  escaping, which I want to avoid.

  By parsing it in the doc comments way, we could even provide appropriate
  syntax highlighting and LSP support in future.
*/
enum TokenType {
  RAW_STRING_LITERAL,
};

void *tree_sitter_banner_external_scanner_create() { return NULL; }
void tree_sitter_banner_external_scanner_destroy(void *p) {}
void tree_sitter_banner_external_scanner_reset(void *p) {}
unsigned tree_sitter_banner_external_scanner_serialize(void *p, char *buffer) {
  return 0;
}
void tree_sitter_banner_external_scanner_deserialize(void *p, const char *b,
                                                     unsigned n) {}

static void advance(TSLexer *lexer) { lexer->advance(lexer, false); }

bool tree_sitter_banner_external_scanner_scan(void *payload, TSLexer *lexer,
                                              const bool *valid_symbols) {

  while (iswspace(lexer->lookahead))
    lexer->advance(lexer, true);

  if (valid_symbols[RAW_STRING_LITERAL] && (lexer->lookahead == 'r')) {
    lexer->result_symbol = RAW_STRING_LITERAL;
    advance(lexer);

    unsigned opening_hash_count = 0;
    while (lexer->lookahead == '#') {
      advance(lexer);
      opening_hash_count++;
    }

    if (lexer->lookahead != '"')
      return false;
    advance(lexer);

    for (;;) {
      if (lexer->lookahead == 0) {
        return false;
      } else if (lexer->lookahead == '"') {
        advance(lexer);
        unsigned hash_count = 0;
        while (lexer->lookahead == '#' && hash_count < opening_hash_count) {
          advance(lexer);
          hash_count++;
        }
        if (hash_count == opening_hash_count) {
          return true;
        }
      } else {
        advance(lexer);
      }
    }
  }

  return false;
}