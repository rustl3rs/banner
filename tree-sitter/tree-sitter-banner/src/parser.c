#include <tree_sitter/parser.h>

#if defined(__GNUC__) || defined(__clang__)
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wmissing-field-initializers"
#endif

#define LANGUAGE_VERSION 14
#define STATE_COUNT 24
#define LARGE_STATE_COUNT 2
#define SYMBOL_COUNT 22
#define ALIAS_COUNT 0
#define TOKEN_COUNT 16
#define EXTERNAL_TOKEN_COUNT 1
#define FIELD_COUNT 0
#define MAX_ALIAS_SEQUENCE_LENGTH 11
#define PRODUCTION_ID_COUNT 1

enum {
  anon_sym_task = 1,
  sym_identifier = 2,
  sym_image_identifier = 3,
  sym_execute_command = 4,
  anon_sym_LPAREN = 5,
  anon_sym_image = 6,
  anon_sym_COLON = 7,
  anon_sym_COMMA = 8,
  anon_sym_execute = 9,
  anon_sym_DQUOTE = 10,
  anon_sym_RPAREN = 11,
  anon_sym_LBRACE = 12,
  anon_sym_RBRACE = 13,
  sym_comment = 14,
  sym_raw_string_literal = 15,
  sym_source_file = 16,
  sym__definition = 17,
  sym_task_definition = 18,
  sym_parameter_list = 19,
  sym_block = 20,
  aux_sym_source_file_repeat1 = 21,
};

static const char * const ts_symbol_names[] = {
  [ts_builtin_sym_end] = "end",
  [anon_sym_task] = "task",
  [sym_identifier] = "identifier",
  [sym_image_identifier] = "image_identifier",
  [sym_execute_command] = "execute_command",
  [anon_sym_LPAREN] = "(",
  [anon_sym_image] = "image",
  [anon_sym_COLON] = ":",
  [anon_sym_COMMA] = ",",
  [anon_sym_execute] = "execute",
  [anon_sym_DQUOTE] = "\"",
  [anon_sym_RPAREN] = ")",
  [anon_sym_LBRACE] = "{",
  [anon_sym_RBRACE] = "}",
  [sym_comment] = "comment",
  [sym_raw_string_literal] = "raw_string_literal",
  [sym_source_file] = "source_file",
  [sym__definition] = "_definition",
  [sym_task_definition] = "task_definition",
  [sym_parameter_list] = "parameter_list",
  [sym_block] = "block",
  [aux_sym_source_file_repeat1] = "source_file_repeat1",
};

static const TSSymbol ts_symbol_map[] = {
  [ts_builtin_sym_end] = ts_builtin_sym_end,
  [anon_sym_task] = anon_sym_task,
  [sym_identifier] = sym_identifier,
  [sym_image_identifier] = sym_image_identifier,
  [sym_execute_command] = sym_execute_command,
  [anon_sym_LPAREN] = anon_sym_LPAREN,
  [anon_sym_image] = anon_sym_image,
  [anon_sym_COLON] = anon_sym_COLON,
  [anon_sym_COMMA] = anon_sym_COMMA,
  [anon_sym_execute] = anon_sym_execute,
  [anon_sym_DQUOTE] = anon_sym_DQUOTE,
  [anon_sym_RPAREN] = anon_sym_RPAREN,
  [anon_sym_LBRACE] = anon_sym_LBRACE,
  [anon_sym_RBRACE] = anon_sym_RBRACE,
  [sym_comment] = sym_comment,
  [sym_raw_string_literal] = sym_raw_string_literal,
  [sym_source_file] = sym_source_file,
  [sym__definition] = sym__definition,
  [sym_task_definition] = sym_task_definition,
  [sym_parameter_list] = sym_parameter_list,
  [sym_block] = sym_block,
  [aux_sym_source_file_repeat1] = aux_sym_source_file_repeat1,
};

static const TSSymbolMetadata ts_symbol_metadata[] = {
  [ts_builtin_sym_end] = {
    .visible = false,
    .named = true,
  },
  [anon_sym_task] = {
    .visible = true,
    .named = false,
  },
  [sym_identifier] = {
    .visible = true,
    .named = true,
  },
  [sym_image_identifier] = {
    .visible = true,
    .named = true,
  },
  [sym_execute_command] = {
    .visible = true,
    .named = true,
  },
  [anon_sym_LPAREN] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_image] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_COLON] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_COMMA] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_execute] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_DQUOTE] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_RPAREN] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_LBRACE] = {
    .visible = true,
    .named = false,
  },
  [anon_sym_RBRACE] = {
    .visible = true,
    .named = false,
  },
  [sym_comment] = {
    .visible = true,
    .named = true,
  },
  [sym_raw_string_literal] = {
    .visible = true,
    .named = true,
  },
  [sym_source_file] = {
    .visible = true,
    .named = true,
  },
  [sym__definition] = {
    .visible = false,
    .named = true,
  },
  [sym_task_definition] = {
    .visible = true,
    .named = true,
  },
  [sym_parameter_list] = {
    .visible = true,
    .named = true,
  },
  [sym_block] = {
    .visible = true,
    .named = true,
  },
  [aux_sym_source_file_repeat1] = {
    .visible = false,
    .named = false,
  },
};

static const TSSymbol ts_alias_sequences[PRODUCTION_ID_COUNT][MAX_ALIAS_SEQUENCE_LENGTH] = {
  [0] = {0},
};

static const uint16_t ts_non_terminal_alias_map[] = {
  0,
};

static const TSStateId ts_primary_state_ids[STATE_COUNT] = {
  [0] = 0,
  [1] = 1,
  [2] = 2,
  [3] = 3,
  [4] = 4,
  [5] = 5,
  [6] = 6,
  [7] = 7,
  [8] = 8,
  [9] = 9,
  [10] = 10,
  [11] = 11,
  [12] = 12,
  [13] = 13,
  [14] = 14,
  [15] = 15,
  [16] = 16,
  [17] = 17,
  [18] = 18,
  [19] = 19,
  [20] = 20,
  [21] = 21,
  [22] = 22,
  [23] = 23,
};

static inline bool sym_execute_command_character_set_1(int32_t c) {
  return (c < '/'
    ? (c < ' '
      ? (c < '\r'
        ? c == '\t'
        : c <= '\r')
      : (c <= ' ' || c == '-'))
    : (c <= '0' || (c < '\\'
      ? (c < 'A'
        ? c == '9'
        : c <= 'Z')
      : (c <= '\\' || (c >= 'a' && c <= 'z')))));
}

static inline bool sym_execute_command_character_set_2(int32_t c) {
  return (c < '0'
    ? (c < ' '
      ? (c < '\r'
        ? (c >= '\t' && c <= '\n')
        : c <= '\r')
      : (c <= ' ' || c == '-'))
    : (c <= '0' || (c < '\\'
      ? (c < 'A'
        ? c == '9'
        : c <= 'Z')
      : (c <= '\\' || (c >= 'a' && c <= 'z')))));
}

static inline bool sym_execute_command_character_set_3(int32_t c) {
  return (c < '/'
    ? (c < ' '
      ? (c < '\r'
        ? (c >= '\t' && c <= '\n')
        : c <= '\r')
      : (c <= ' ' || c == '-'))
    : (c <= '0' || (c < '\\'
      ? (c < 'A'
        ? c == '9'
        : c <= 'Z')
      : (c <= '\\' || (c >= 'a' && c <= 'z')))));
}

static bool ts_lex(TSLexer *lexer, TSStateId state) {
  START_LEXER();
  eof = lexer->eof(lexer);
  switch (state) {
    case 0:
      if (eof) ADVANCE(33);
      if (lookahead == '"') ADVANCE(48);
      if (lookahead == '(') ADVANCE(43);
      if (lookahead == ')') ADVANCE(49);
      if (lookahead == ',') ADVANCE(46);
      if (lookahead == '/') ADVANCE(1);
      if (lookahead == ':') ADVANCE(45);
      if (lookahead == 'e') ADVANCE(24);
      if (lookahead == 'i') ADVANCE(20);
      if (lookahead == 't') ADVANCE(12);
      if (lookahead == '{') ADVANCE(50);
      if (lookahead == '}') ADVANCE(51);
      if (lookahead == '\t' ||
          lookahead == '\n' ||
          lookahead == '\r' ||
          lookahead == ' ') SKIP(0)
      END_STATE();
    case 1:
      if (lookahead == '*') ADVANCE(3);
      if (lookahead == '/') ADVANCE(53);
      END_STATE();
    case 2:
      if (lookahead == '*') ADVANCE(2);
      if (lookahead == '/') ADVANCE(52);
      if (lookahead != 0) ADVANCE(3);
      END_STATE();
    case 3:
      if (lookahead == '*') ADVANCE(2);
      if (lookahead != 0) ADVANCE(3);
      END_STATE();
    case 4:
      if (lookahead == '-') ADVANCE(6);
      if (lookahead == ':') ADVANCE(28);
      if (lookahead == '\\') ADVANCE(26);
      if (lookahead == '/' ||
          lookahead == '@') ADVANCE(30);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(4);
      END_STATE();
    case 5:
      if (lookahead == '-') ADVANCE(6);
      if (lookahead == ':') ADVANCE(28);
      if (lookahead == '\\') ADVANCE(26);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(4);
      END_STATE();
    case 6:
      if (lookahead == '-') ADVANCE(6);
      if (lookahead == '\\') ADVANCE(25);
      if (lookahead == '/' ||
          lookahead == '@') ADVANCE(30);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(4);
      END_STATE();
    case 7:
      if (lookahead == '/') ADVANCE(1);
      if (lookahead == '\t' ||
          lookahead == '\n' ||
          lookahead == '\r' ||
          lookahead == ' ') SKIP(7)
      if (('0' <= lookahead && lookahead <= '9') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(5);
      END_STATE();
    case 8:
      if (lookahead == '/') ADVANCE(1);
      if (lookahead == '\t' ||
          lookahead == '\n' ||
          lookahead == '\r' ||
          lookahead == ' ') SKIP(8)
      if (lookahead == '-' ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(35);
      END_STATE();
    case 9:
      if (lookahead == '/') ADVANCE(40);
      if (lookahead == '\t' ||
          lookahead == '\n' ||
          lookahead == '\r' ||
          lookahead == ' ') ADVANCE(41);
      if (lookahead == '-' ||
          lookahead == '0' ||
          lookahead == '9' ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '\\' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(42);
      END_STATE();
    case 10:
      if (lookahead == '/') ADVANCE(31);
      END_STATE();
    case 11:
      if (lookahead == '\\') ADVANCE(10);
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(11);
      END_STATE();
    case 12:
      if (lookahead == 'a') ADVANCE(21);
      END_STATE();
    case 13:
      if (lookahead == 'a') ADVANCE(18);
      END_STATE();
    case 14:
      if (lookahead == 'c') ADVANCE(23);
      END_STATE();
    case 15:
      if (lookahead == 'e') ADVANCE(14);
      END_STATE();
    case 16:
      if (lookahead == 'e') ADVANCE(44);
      END_STATE();
    case 17:
      if (lookahead == 'e') ADVANCE(47);
      END_STATE();
    case 18:
      if (lookahead == 'g') ADVANCE(16);
      END_STATE();
    case 19:
      if (lookahead == 'k') ADVANCE(34);
      END_STATE();
    case 20:
      if (lookahead == 'm') ADVANCE(13);
      END_STATE();
    case 21:
      if (lookahead == 's') ADVANCE(19);
      END_STATE();
    case 22:
      if (lookahead == 't') ADVANCE(17);
      END_STATE();
    case 23:
      if (lookahead == 'u') ADVANCE(22);
      END_STATE();
    case 24:
      if (lookahead == 'x') ADVANCE(15);
      END_STATE();
    case 25:
      if (lookahead == '-' ||
          lookahead == '\\') ADVANCE(25);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(5);
      END_STATE();
    case 26:
      if (lookahead == '-' ||
          lookahead == '\\') ADVANCE(25);
      if (('0' <= lookahead && lookahead <= '9') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(5);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(29);
      END_STATE();
    case 27:
      if (lookahead == '/' ||
          lookahead == '@') ADVANCE(30);
      if (lookahead == '-' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(27);
      END_STATE();
    case 28:
      if (('0' <= lookahead && lookahead <= '9')) ADVANCE(11);
      END_STATE();
    case 29:
      if (('0' <= lookahead && lookahead <= '9') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(5);
      END_STATE();
    case 30:
      if (lookahead == '-' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(36);
      END_STATE();
    case 31:
      if (lookahead == '-' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(27);
      END_STATE();
    case 32:
      if (lookahead == '-' ||
          lookahead == '.' ||
          ('0' <= lookahead && lookahead <= '9') ||
          lookahead == '\\' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(38);
      END_STATE();
    case 33:
      ACCEPT_TOKEN(ts_builtin_sym_end);
      END_STATE();
    case 34:
      ACCEPT_TOKEN(anon_sym_task);
      END_STATE();
    case 35:
      ACCEPT_TOKEN(sym_identifier);
      if (lookahead == '-' ||
          lookahead == '_' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(35);
      END_STATE();
    case 36:
      ACCEPT_TOKEN(sym_image_identifier);
      if (lookahead == ':') ADVANCE(32);
      if (lookahead == '/' ||
          lookahead == '@') ADVANCE(37);
      if (lookahead == '-' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(36);
      END_STATE();
    case 37:
      ACCEPT_TOKEN(sym_image_identifier);
      if (lookahead == ':') ADVANCE(32);
      if (lookahead == '-' ||
          ('0' <= lookahead && lookahead <= '9') ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(37);
      END_STATE();
    case 38:
      ACCEPT_TOKEN(sym_image_identifier);
      if (lookahead == '-' ||
          lookahead == '.' ||
          ('0' <= lookahead && lookahead <= '9') ||
          lookahead == '\\' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(38);
      END_STATE();
    case 39:
      ACCEPT_TOKEN(sym_execute_command);
      if (lookahead == '\n') ADVANCE(42);
      if (sym_execute_command_character_set_1(lookahead)) ADVANCE(39);
      if (lookahead != 0) ADVANCE(53);
      END_STATE();
    case 40:
      ACCEPT_TOKEN(sym_execute_command);
      if (lookahead == '*') ADVANCE(3);
      if (lookahead == '/') ADVANCE(39);
      if (sym_execute_command_character_set_2(lookahead)) ADVANCE(42);
      END_STATE();
    case 41:
      ACCEPT_TOKEN(sym_execute_command);
      if (lookahead == '/') ADVANCE(40);
      if (lookahead == '\t' ||
          lookahead == '\n' ||
          lookahead == '\r' ||
          lookahead == ' ') ADVANCE(41);
      if (lookahead == '-' ||
          lookahead == '0' ||
          lookahead == '9' ||
          ('A' <= lookahead && lookahead <= 'Z') ||
          lookahead == '\\' ||
          ('a' <= lookahead && lookahead <= 'z')) ADVANCE(42);
      END_STATE();
    case 42:
      ACCEPT_TOKEN(sym_execute_command);
      if (sym_execute_command_character_set_3(lookahead)) ADVANCE(42);
      END_STATE();
    case 43:
      ACCEPT_TOKEN(anon_sym_LPAREN);
      END_STATE();
    case 44:
      ACCEPT_TOKEN(anon_sym_image);
      END_STATE();
    case 45:
      ACCEPT_TOKEN(anon_sym_COLON);
      END_STATE();
    case 46:
      ACCEPT_TOKEN(anon_sym_COMMA);
      END_STATE();
    case 47:
      ACCEPT_TOKEN(anon_sym_execute);
      END_STATE();
    case 48:
      ACCEPT_TOKEN(anon_sym_DQUOTE);
      END_STATE();
    case 49:
      ACCEPT_TOKEN(anon_sym_RPAREN);
      END_STATE();
    case 50:
      ACCEPT_TOKEN(anon_sym_LBRACE);
      END_STATE();
    case 51:
      ACCEPT_TOKEN(anon_sym_RBRACE);
      END_STATE();
    case 52:
      ACCEPT_TOKEN(sym_comment);
      END_STATE();
    case 53:
      ACCEPT_TOKEN(sym_comment);
      if (lookahead != 0 &&
          lookahead != '\n') ADVANCE(53);
      END_STATE();
    default:
      return false;
  }
}

static const TSLexMode ts_lex_modes[STATE_COUNT] = {
  [0] = {.lex_state = 0, .external_lex_state = 1},
  [1] = {.lex_state = 0},
  [2] = {.lex_state = 0},
  [3] = {.lex_state = 0},
  [4] = {.lex_state = 0},
  [5] = {.lex_state = 0},
  [6] = {.lex_state = 0},
  [7] = {.lex_state = 0},
  [8] = {.lex_state = 0},
  [9] = {.lex_state = 0, .external_lex_state = 1},
  [10] = {.lex_state = 7},
  [11] = {.lex_state = 0},
  [12] = {.lex_state = 8},
  [13] = {.lex_state = 0},
  [14] = {.lex_state = 0},
  [15] = {.lex_state = 0},
  [16] = {.lex_state = 0},
  [17] = {.lex_state = 0},
  [18] = {.lex_state = 0},
  [19] = {.lex_state = 0},
  [20] = {.lex_state = 9},
  [21] = {.lex_state = 0},
  [22] = {.lex_state = 0},
  [23] = {.lex_state = 0},
};

enum {
  ts_external_token_raw_string_literal = 0,
};

static const TSSymbol ts_external_scanner_symbol_map[EXTERNAL_TOKEN_COUNT] = {
  [ts_external_token_raw_string_literal] = sym_raw_string_literal,
};

static const bool ts_external_scanner_states[2][EXTERNAL_TOKEN_COUNT] = {
  [1] = {
    [ts_external_token_raw_string_literal] = true,
  },
};

static const uint16_t ts_parse_table[LARGE_STATE_COUNT][SYMBOL_COUNT] = {
  [0] = {
    [ts_builtin_sym_end] = ACTIONS(1),
    [anon_sym_task] = ACTIONS(1),
    [anon_sym_LPAREN] = ACTIONS(1),
    [anon_sym_image] = ACTIONS(1),
    [anon_sym_COLON] = ACTIONS(1),
    [anon_sym_COMMA] = ACTIONS(1),
    [anon_sym_execute] = ACTIONS(1),
    [anon_sym_DQUOTE] = ACTIONS(1),
    [anon_sym_RPAREN] = ACTIONS(1),
    [anon_sym_LBRACE] = ACTIONS(1),
    [anon_sym_RBRACE] = ACTIONS(1),
    [sym_comment] = ACTIONS(3),
    [sym_raw_string_literal] = ACTIONS(1),
  },
  [1] = {
    [sym_source_file] = STATE(16),
    [sym__definition] = STATE(2),
    [sym_task_definition] = STATE(2),
    [aux_sym_source_file_repeat1] = STATE(2),
    [ts_builtin_sym_end] = ACTIONS(5),
    [anon_sym_task] = ACTIONS(7),
    [sym_comment] = ACTIONS(3),
  },
};

static const uint16_t ts_small_parse_table[] = {
  [0] = 4,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(7), 1,
      anon_sym_task,
    ACTIONS(9), 1,
      ts_builtin_sym_end,
    STATE(3), 3,
      sym__definition,
      sym_task_definition,
      aux_sym_source_file_repeat1,
  [15] = 4,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(11), 1,
      ts_builtin_sym_end,
    ACTIONS(13), 1,
      anon_sym_task,
    STATE(3), 3,
      sym__definition,
      sym_task_definition,
      aux_sym_source_file_repeat1,
  [30] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(16), 2,
      ts_builtin_sym_end,
      anon_sym_task,
  [38] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(18), 2,
      ts_builtin_sym_end,
      anon_sym_task,
  [46] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(20), 2,
      ts_builtin_sym_end,
      anon_sym_task,
  [54] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(22), 1,
      anon_sym_LPAREN,
    STATE(8), 1,
      sym_parameter_list,
  [64] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(24), 1,
      anon_sym_LBRACE,
    STATE(4), 1,
      sym_block,
  [74] = 3,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(26), 1,
      anon_sym_RBRACE,
    ACTIONS(28), 1,
      sym_raw_string_literal,
  [84] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(30), 1,
      sym_image_identifier,
  [91] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(32), 1,
      anon_sym_COLON,
  [98] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(34), 1,
      sym_identifier,
  [105] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(36), 1,
      anon_sym_RBRACE,
  [112] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(38), 1,
      anon_sym_image,
  [119] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(40), 1,
      anon_sym_COMMA,
  [126] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(42), 1,
      ts_builtin_sym_end,
  [133] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(44), 1,
      anon_sym_execute,
  [140] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(46), 1,
      anon_sym_COLON,
  [147] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(48), 1,
      anon_sym_DQUOTE,
  [154] = 2,
    ACTIONS(50), 1,
      sym_execute_command,
    ACTIONS(52), 1,
      sym_comment,
  [161] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(54), 1,
      anon_sym_DQUOTE,
  [168] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(56), 1,
      anon_sym_RPAREN,
  [175] = 2,
    ACTIONS(3), 1,
      sym_comment,
    ACTIONS(58), 1,
      anon_sym_LBRACE,
};

static const uint32_t ts_small_parse_table_map[] = {
  [SMALL_STATE(2)] = 0,
  [SMALL_STATE(3)] = 15,
  [SMALL_STATE(4)] = 30,
  [SMALL_STATE(5)] = 38,
  [SMALL_STATE(6)] = 46,
  [SMALL_STATE(7)] = 54,
  [SMALL_STATE(8)] = 64,
  [SMALL_STATE(9)] = 74,
  [SMALL_STATE(10)] = 84,
  [SMALL_STATE(11)] = 91,
  [SMALL_STATE(12)] = 98,
  [SMALL_STATE(13)] = 105,
  [SMALL_STATE(14)] = 112,
  [SMALL_STATE(15)] = 119,
  [SMALL_STATE(16)] = 126,
  [SMALL_STATE(17)] = 133,
  [SMALL_STATE(18)] = 140,
  [SMALL_STATE(19)] = 147,
  [SMALL_STATE(20)] = 154,
  [SMALL_STATE(21)] = 161,
  [SMALL_STATE(22)] = 168,
  [SMALL_STATE(23)] = 175,
};

static const TSParseActionEntry ts_parse_actions[] = {
  [0] = {.entry = {.count = 0, .reusable = false}},
  [1] = {.entry = {.count = 1, .reusable = false}}, RECOVER(),
  [3] = {.entry = {.count = 1, .reusable = true}}, SHIFT_EXTRA(),
  [5] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_source_file, 0),
  [7] = {.entry = {.count = 1, .reusable = true}}, SHIFT(12),
  [9] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_source_file, 1),
  [11] = {.entry = {.count = 1, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2),
  [13] = {.entry = {.count = 2, .reusable = true}}, REDUCE(aux_sym_source_file_repeat1, 2), SHIFT_REPEAT(12),
  [16] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_task_definition, 4),
  [18] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_block, 3),
  [20] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_block, 2),
  [22] = {.entry = {.count = 1, .reusable = true}}, SHIFT(14),
  [24] = {.entry = {.count = 1, .reusable = true}}, SHIFT(9),
  [26] = {.entry = {.count = 1, .reusable = true}}, SHIFT(6),
  [28] = {.entry = {.count = 1, .reusable = true}}, SHIFT(13),
  [30] = {.entry = {.count = 1, .reusable = true}}, SHIFT(15),
  [32] = {.entry = {.count = 1, .reusable = true}}, SHIFT(10),
  [34] = {.entry = {.count = 1, .reusable = true}}, SHIFT(7),
  [36] = {.entry = {.count = 1, .reusable = true}}, SHIFT(5),
  [38] = {.entry = {.count = 1, .reusable = true}}, SHIFT(11),
  [40] = {.entry = {.count = 1, .reusable = true}}, SHIFT(17),
  [42] = {.entry = {.count = 1, .reusable = true}},  ACCEPT_INPUT(),
  [44] = {.entry = {.count = 1, .reusable = true}}, SHIFT(18),
  [46] = {.entry = {.count = 1, .reusable = true}}, SHIFT(19),
  [48] = {.entry = {.count = 1, .reusable = true}}, SHIFT(20),
  [50] = {.entry = {.count = 1, .reusable = false}}, SHIFT(21),
  [52] = {.entry = {.count = 1, .reusable = false}}, SHIFT_EXTRA(),
  [54] = {.entry = {.count = 1, .reusable = true}}, SHIFT(22),
  [56] = {.entry = {.count = 1, .reusable = true}}, SHIFT(23),
  [58] = {.entry = {.count = 1, .reusable = true}}, REDUCE(sym_parameter_list, 11),
};

#ifdef __cplusplus
extern "C" {
#endif
void *tree_sitter_banner_external_scanner_create(void);
void tree_sitter_banner_external_scanner_destroy(void *);
bool tree_sitter_banner_external_scanner_scan(void *, TSLexer *, const bool *);
unsigned tree_sitter_banner_external_scanner_serialize(void *, char *);
void tree_sitter_banner_external_scanner_deserialize(void *, const char *, unsigned);

#ifdef _WIN32
#define extern __declspec(dllexport)
#endif

extern const TSLanguage *tree_sitter_banner(void) {
  static const TSLanguage language = {
    .version = LANGUAGE_VERSION,
    .symbol_count = SYMBOL_COUNT,
    .alias_count = ALIAS_COUNT,
    .token_count = TOKEN_COUNT,
    .external_token_count = EXTERNAL_TOKEN_COUNT,
    .state_count = STATE_COUNT,
    .large_state_count = LARGE_STATE_COUNT,
    .production_id_count = PRODUCTION_ID_COUNT,
    .field_count = FIELD_COUNT,
    .max_alias_sequence_length = MAX_ALIAS_SEQUENCE_LENGTH,
    .parse_table = &ts_parse_table[0][0],
    .small_parse_table = ts_small_parse_table,
    .small_parse_table_map = ts_small_parse_table_map,
    .parse_actions = ts_parse_actions,
    .symbol_names = ts_symbol_names,
    .symbol_metadata = ts_symbol_metadata,
    .public_symbol_map = ts_symbol_map,
    .alias_map = ts_non_terminal_alias_map,
    .alias_sequences = &ts_alias_sequences[0][0],
    .lex_modes = ts_lex_modes,
    .lex_fn = ts_lex,
    .external_scanner = {
      &ts_external_scanner_states[0][0],
      ts_external_scanner_symbol_map,
      tree_sitter_banner_external_scanner_create,
      tree_sitter_banner_external_scanner_destroy,
      tree_sitter_banner_external_scanner_scan,
      tree_sitter_banner_external_scanner_serialize,
      tree_sitter_banner_external_scanner_deserialize,
    },
    .primary_state_ids = ts_primary_state_ids,
  };
  return &language;
}
#ifdef __cplusplus
}
#endif
