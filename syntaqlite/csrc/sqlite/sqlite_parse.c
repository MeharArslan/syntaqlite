/*
** 2000-05-29
**
** The author disclaims copyright to this source code.  In place of
** a legal notice, here is a blessing:
**
**    May you do good and not evil.
**    May you find forgiveness for yourself and forgive others.
**    May you share freely, never taking more than you give.
**
*************************************************************************
** Driver template for the LEMON parser generator.
**
** The "lemon" program processes an LALR(1) input grammar file, then uses
** this template to construct a parser.  The "lemon" program inserts text
** at each "%%" line.  Also, any "P-a-r-s-e" identifier prefix (without the
** interstitial "-" characters) contained in this template is changed into
** the value of the %name directive from the grammar.  Otherwise, the content
** of this template is copied straight through into the generate parser
** source file.
**
** The following is the concatenation of all %include directives from the
** input grammar file:
*/
/************ Begin %include sections from the grammar ************************/
#include <string.h>

#include "csrc/sqlite/dialect_builder.h"
#include "syntaqlite/types.h"
#include "syntaqlite_ext/ast_builder.h"
#include "syntaqlite_sqlite/sqlite_tokens.h"

/* BEGIN GRAMMAR_TYPES */
// Grammar-specific struct types for multi-valued grammar nonterminals.
// These are used by Lemon-generated parser actions to bundle multiple
// values through a single nonterminal reduction.

// columnname: passes name span + typetoken span from column definition.
typedef struct SynqColumnNameValue {
  SyntaqliteSourceSpan name;
  SyntaqliteSourceSpan typetoken;
} SynqColumnNameValue;

// ccons / tcons / generated: a constraint node + pending constraint name.
typedef struct SynqConstraintValue {
  uint32_t node;
  SyntaqliteSourceSpan pending_name;
} SynqConstraintValue;

// carglist / conslist: accumulated constraint list + pending name for next.
typedef struct SynqConstraintListValue {
  uint32_t list;
  SyntaqliteSourceSpan pending_name;
} SynqConstraintListValue;

// on_using: ON expr / USING column-list discriminator.
typedef struct SynqOnUsingValue {
  uint32_t on_expr;
  uint32_t using_cols;
} SynqOnUsingValue;

// with: recursive flag + CTE list node ID.
typedef struct SynqWithValue {
  uint32_t cte_list;
  int is_recursive;
} SynqWithValue;
/* END GRAMMAR_TYPES */

#define YYPARSEFREENEVERNULL 1
/**************** End of %include directives **********************************/
/* These constants specify the various numeric values for terminal symbols.
***************** Begin token definitions *************************************/
#ifndef SYNTAQLITE_TK_ABORT
#define SYNTAQLITE_TK_ABORT 1
#define SYNTAQLITE_TK_ACTION 2
#define SYNTAQLITE_TK_AFTER 3
#define SYNTAQLITE_TK_ANALYZE 4
#define SYNTAQLITE_TK_ASC 5
#define SYNTAQLITE_TK_ATTACH 6
#define SYNTAQLITE_TK_BEFORE 7
#define SYNTAQLITE_TK_BEGIN 8
#define SYNTAQLITE_TK_BY 9
#define SYNTAQLITE_TK_CASCADE 10
#define SYNTAQLITE_TK_CAST 11
#define SYNTAQLITE_TK_CONFLICT 12
#define SYNTAQLITE_TK_DATABASE 13
#define SYNTAQLITE_TK_DEFERRED 14
#define SYNTAQLITE_TK_DESC 15
#define SYNTAQLITE_TK_DETACH 16
#define SYNTAQLITE_TK_EACH 17
#define SYNTAQLITE_TK_END 18
#define SYNTAQLITE_TK_EXCLUSIVE 19
#define SYNTAQLITE_TK_EXPLAIN 20
#define SYNTAQLITE_TK_FAIL 21
#define SYNTAQLITE_TK_OR 22
#define SYNTAQLITE_TK_AND 23
#define SYNTAQLITE_TK_NOT 24
#define SYNTAQLITE_TK_IS 25
#define SYNTAQLITE_TK_ISNOT 26
#define SYNTAQLITE_TK_MATCH 27
#define SYNTAQLITE_TK_LIKE_KW 28
#define SYNTAQLITE_TK_BETWEEN 29
#define SYNTAQLITE_TK_IN 30
#define SYNTAQLITE_TK_ISNULL 31
#define SYNTAQLITE_TK_NOTNULL 32
#define SYNTAQLITE_TK_NE 33
#define SYNTAQLITE_TK_EQ 34
#define SYNTAQLITE_TK_GT 35
#define SYNTAQLITE_TK_LE 36
#define SYNTAQLITE_TK_LT 37
#define SYNTAQLITE_TK_GE 38
#define SYNTAQLITE_TK_ESCAPE 39
#define SYNTAQLITE_TK_ID 40
#define SYNTAQLITE_TK_COLUMNKW 41
#define SYNTAQLITE_TK_DO 42
#define SYNTAQLITE_TK_FOR 43
#define SYNTAQLITE_TK_IGNORE 44
#define SYNTAQLITE_TK_IMMEDIATE 45
#define SYNTAQLITE_TK_INITIALLY 46
#define SYNTAQLITE_TK_INSTEAD 47
#define SYNTAQLITE_TK_NO 48
#define SYNTAQLITE_TK_PLAN 49
#define SYNTAQLITE_TK_QUERY 50
#define SYNTAQLITE_TK_KEY 51
#define SYNTAQLITE_TK_OF 52
#define SYNTAQLITE_TK_OFFSET 53
#define SYNTAQLITE_TK_PRAGMA 54
#define SYNTAQLITE_TK_RAISE 55
#define SYNTAQLITE_TK_RECURSIVE 56
#define SYNTAQLITE_TK_RELEASE 57
#define SYNTAQLITE_TK_REPLACE 58
#define SYNTAQLITE_TK_RESTRICT 59
#define SYNTAQLITE_TK_ROW 60
#define SYNTAQLITE_TK_ROWS 61
#define SYNTAQLITE_TK_ROLLBACK 62
#define SYNTAQLITE_TK_SAVEPOINT 63
#define SYNTAQLITE_TK_TEMP 64
#define SYNTAQLITE_TK_TRIGGER 65
#define SYNTAQLITE_TK_VACUUM 66
#define SYNTAQLITE_TK_VIEW 67
#define SYNTAQLITE_TK_VIRTUAL 68
#define SYNTAQLITE_TK_WITH 69
#define SYNTAQLITE_TK_WITHOUT 70
#define SYNTAQLITE_TK_NULLS 71
#define SYNTAQLITE_TK_FIRST 72
#define SYNTAQLITE_TK_LAST 73
#define SYNTAQLITE_TK_CURRENT 74
#define SYNTAQLITE_TK_FOLLOWING 75
#define SYNTAQLITE_TK_PARTITION 76
#define SYNTAQLITE_TK_PRECEDING 77
#define SYNTAQLITE_TK_RANGE 78
#define SYNTAQLITE_TK_UNBOUNDED 79
#define SYNTAQLITE_TK_EXCLUDE 80
#define SYNTAQLITE_TK_GROUPS 81
#define SYNTAQLITE_TK_OTHERS 82
#define SYNTAQLITE_TK_TIES 83
#define SYNTAQLITE_TK_GENERATED 84
#define SYNTAQLITE_TK_ALWAYS 85
#define SYNTAQLITE_TK_WITHIN 86
#define SYNTAQLITE_TK_MATERIALIZED 87
#define SYNTAQLITE_TK_REINDEX 88
#define SYNTAQLITE_TK_RENAME 89
#define SYNTAQLITE_TK_CTIME_KW 90
#define SYNTAQLITE_TK_IF 91
#define SYNTAQLITE_TK_ANY 92
#define SYNTAQLITE_TK_BITAND 93
#define SYNTAQLITE_TK_BITOR 94
#define SYNTAQLITE_TK_LSHIFT 95
#define SYNTAQLITE_TK_RSHIFT 96
#define SYNTAQLITE_TK_PLUS 97
#define SYNTAQLITE_TK_MINUS 98
#define SYNTAQLITE_TK_STAR 99
#define SYNTAQLITE_TK_SLASH 100
#define SYNTAQLITE_TK_REM 101
#define SYNTAQLITE_TK_CONCAT 102
#define SYNTAQLITE_TK_PTR 103
#define SYNTAQLITE_TK_COLLATE 104
#define SYNTAQLITE_TK_BITNOT 105
#define SYNTAQLITE_TK_ON 106
#define SYNTAQLITE_TK_INDEXED 107
#define SYNTAQLITE_TK_STRING 108
#define SYNTAQLITE_TK_JOIN_KW 109
#define SYNTAQLITE_TK_INTEGER 110
#define SYNTAQLITE_TK_FLOAT 111
#define SYNTAQLITE_TK_SEMI 112
#define SYNTAQLITE_TK_LP 113
#define SYNTAQLITE_TK_ORDER 114
#define SYNTAQLITE_TK_RP 115
#define SYNTAQLITE_TK_GROUP 116
#define SYNTAQLITE_TK_AS 117
#define SYNTAQLITE_TK_COMMA 118
#define SYNTAQLITE_TK_DOT 119
#define SYNTAQLITE_TK_UNION 120
#define SYNTAQLITE_TK_ALL 121
#define SYNTAQLITE_TK_EXCEPT 122
#define SYNTAQLITE_TK_INTERSECT 123
#define SYNTAQLITE_TK_EXISTS 124
#define SYNTAQLITE_TK_NULL 125
#define SYNTAQLITE_TK_DISTINCT 126
#define SYNTAQLITE_TK_FROM 127
#define SYNTAQLITE_TK_CASE 128
#define SYNTAQLITE_TK_WHEN 129
#define SYNTAQLITE_TK_THEN 130
#define SYNTAQLITE_TK_ELSE 131
#define SYNTAQLITE_TK_TABLE 132
#define SYNTAQLITE_TK_CONSTRAINT 133
#define SYNTAQLITE_TK_DEFAULT 134
#define SYNTAQLITE_TK_PRIMARY 135
#define SYNTAQLITE_TK_UNIQUE 136
#define SYNTAQLITE_TK_CHECK 137
#define SYNTAQLITE_TK_REFERENCES 138
#define SYNTAQLITE_TK_AUTOINCR 139
#define SYNTAQLITE_TK_INSERT 140
#define SYNTAQLITE_TK_DELETE 141
#define SYNTAQLITE_TK_UPDATE 142
#define SYNTAQLITE_TK_SET 143
#define SYNTAQLITE_TK_DEFERRABLE 144
#define SYNTAQLITE_TK_FOREIGN 145
#define SYNTAQLITE_TK_INTO 146
#define SYNTAQLITE_TK_VALUES 147
#define SYNTAQLITE_TK_WHERE 148
#define SYNTAQLITE_TK_RETURNING 149
#define SYNTAQLITE_TK_NOTHING 150
#define SYNTAQLITE_TK_BLOB 151
#define SYNTAQLITE_TK_QNUMBER 152
#define SYNTAQLITE_TK_VARIABLE 153
#define SYNTAQLITE_TK_DROP 154
#define SYNTAQLITE_TK_INDEX 155
#define SYNTAQLITE_TK_ALTER 156
#define SYNTAQLITE_TK_TO 157
#define SYNTAQLITE_TK_ADD 158
#define SYNTAQLITE_TK_COMMIT 159
#define SYNTAQLITE_TK_TRANSACTION 160
#define SYNTAQLITE_TK_SELECT 161
#define SYNTAQLITE_TK_HAVING 162
#define SYNTAQLITE_TK_LIMIT 163
#define SYNTAQLITE_TK_JOIN 164
#define SYNTAQLITE_TK_USING 165
#define SYNTAQLITE_TK_CREATE 166
#define SYNTAQLITE_TK_WINDOW 167
#define SYNTAQLITE_TK_OVER 168
#define SYNTAQLITE_TK_FILTER 169
#define SYNTAQLITE_TK_COLUMN 170
#define SYNTAQLITE_TK_AGG_FUNCTION 171
#define SYNTAQLITE_TK_AGG_COLUMN 172
#define SYNTAQLITE_TK_TRUEFALSE 173
#define SYNTAQLITE_TK_FUNCTION 174
#define SYNTAQLITE_TK_UPLUS 175
#define SYNTAQLITE_TK_UMINUS 176
#define SYNTAQLITE_TK_TRUTH 177
#define SYNTAQLITE_TK_REGISTER 178
#define SYNTAQLITE_TK_VECTOR 179
#define SYNTAQLITE_TK_SELECT_COLUMN 180
#define SYNTAQLITE_TK_IF_NULL_ROW 181
#define SYNTAQLITE_TK_ASTERISK 182
#define SYNTAQLITE_TK_SPAN 183
#define SYNTAQLITE_TK_ERROR 184
#define SYNTAQLITE_TK_SPACE 185
#define SYNTAQLITE_TK_COMMENT 186
#define SYNTAQLITE_TK_ILLEGAL 187
#endif
/**************** End token definitions ***************************************/

/* The next sections is a series of control #defines.
** various aspects of the generated parser.
**    YYCODETYPE         is the data type used to store the integer codes
**                       that represent terminal and non-terminal symbols.
**                       "unsigned char" is used if there are fewer than
**                       256 symbols.  Larger types otherwise.
**    YYNOCODE           is a number of type YYCODETYPE that is not used for
**                       any terminal or nonterminal symbol.
**    YYFALLBACK         If defined, this indicates that one or more tokens
**                       (also known as: "terminal symbols") have fall-back
**                       values which should be used if the original symbol
**                       would not parse.  This permits keywords to sometimes
**                       be used as identifiers, for example.
**    YYACTIONTYPE       is the data type used for "action codes" - numbers
**                       that indicate what to do in response to the next
**                       token.
**    SynqSqliteParseTOKENTYPE     is the data type used for minor type for
*terminal
**                       symbols.  Background: A "minor type" is a semantic
**                       value associated with a terminal or non-terminal
**                       symbols.  For example, for an "ID" terminal symbol,
**                       the minor type might be the name of the identifier.
**                       Each non-terminal can have a different minor type.
**                       Terminal symbols all have the same minor type, though.
**                       This macros defines the minor type for terminal
**                       symbols.
**    YYMINORTYPE        is the data type used for all minor types.
**                       This is typically a union of many types, one of
**                       which is SynqSqliteParseTOKENTYPE.  The entry in the
*union
**                       for terminal symbols is called "yy0".
**    YYSTACKDEPTH       is the maximum depth of the parser's stack.  If
**                       zero the stack is dynamically sized using realloc()
**    SynqSqliteParseARG_SDECL     A static variable declaration for the
*%extra_argument
**    SynqSqliteParseARG_PDECL     A parameter declaration for the
*%extra_argument
**    SynqSqliteParseARG_PARAM     Code to pass %extra_argument as a subroutine
*parameter
**    SynqSqliteParseARG_STORE     Code to store %extra_argument into yypParser
**    SynqSqliteParseARG_FETCH     Code to extract %extra_argument from
*yypParser
**    SynqSqliteParseCTX_*         As SynqSqliteParseARG_ except for
*%extra_context
**    YYREALLOC          Name of the realloc() function to use
**    YYFREE             Name of the free() function to use
**    YYDYNSTACK         True if stack space should be extended on heap
**    YYERRORSYMBOL      is the code number of the error symbol.  If not
**                       defined, then do no error processing.
**    YYNSTATE           the combined number of states.
**    YYNRULE            the number of rules in the grammar
**    YYNTOKEN           Number of terminal symbols
**    YY_MAX_SHIFT       Maximum value for shift actions
**    YY_MIN_SHIFTREDUCE Minimum value for shift-reduce actions
**    YY_MAX_SHIFTREDUCE Maximum value for shift-reduce actions
**    YY_ERROR_ACTION    The yy_action[] code for syntax error
**    YY_ACCEPT_ACTION   The yy_action[] code for accept
**    YY_NO_ACTION       The yy_action[] code for no-op
**    YY_MIN_REDUCE      Minimum value for reduce actions
**    YY_MAX_REDUCE      Maximum value for reduce actions
**    YY_MIN_DSTRCTR     Minimum symbol value that has a destructor
**    YY_MAX_DSTRCTR     Maximum symbol value that has a destructor
*/
#ifndef INTERFACE
#define INTERFACE 1
#endif
/************* Begin control #defines *****************************************/
#define YYCODETYPE unsigned short int
#define YYNOCODE 324
#define YYACTIONTYPE unsigned short int
#define YYWILDCARD 92
#define SynqSqliteParseTOKENTYPE SynqParseToken
typedef union {
  int yyinit;
  SynqSqliteParseTOKENTYPE yy0;
  SynqConstraintListValue yy10;
  SynqConstraintValue yy34;
  uint32_t yy213;
  int yy220;
  SynqOnUsingValue yy304;
  SynqColumnNameValue yy400;
  SynqWithValue yy465;
  int yy649;
} YYMINORTYPE;
#ifndef YYSTACKDEPTH
#define YYSTACKDEPTH 100
#endif
#define SynqSqliteParseARG_SDECL SynqParseCtx* pCtx;
#define SynqSqliteParseARG_PDECL , SynqParseCtx* pCtx
#define SynqSqliteParseARG_PARAM , pCtx
#define SynqSqliteParseARG_FETCH SynqParseCtx* pCtx = yypParser->pCtx;
#define SynqSqliteParseARG_STORE yypParser->pCtx = pCtx;
#define YYREALLOC realloc
#define YYFREE free
#define YYDYNSTACK 0
#define SynqSqliteParseCTX_SDECL
#define SynqSqliteParseCTX_PDECL
#define SynqSqliteParseCTX_PARAM
#define SynqSqliteParseCTX_FETCH
#define SynqSqliteParseCTX_STORE
#define YYERRORSYMBOL 192
#define YYERRSYMDT yy649
#define YYFALLBACK 1
#define YYNSTATE 595
#define YYNRULE 412
#define YYNRULE_WITH_ACTION 412
#define YYNTOKEN 188
#define YY_MAX_SHIFT 594
#define YY_MIN_SHIFTREDUCE 859
#define YY_MAX_SHIFTREDUCE 1270
#define YY_ERROR_ACTION 1271
#define YY_ACCEPT_ACTION 1272
#define YY_NO_ACTION 1273
#define YY_MIN_REDUCE 1274
#define YY_MAX_REDUCE 1685
#define YY_MIN_DSTRCTR 0
#define YY_MAX_DSTRCTR 0
/************* End control #defines *******************************************/
#define YY_NLOOKAHEAD ((int)(sizeof(yy_lookahead) / sizeof(yy_lookahead[0])))

/* Define the yytestcase() macro to be a no-op if is not already defined
** otherwise.
**
** Applications can choose to define yytestcase() in the %include section
** to a macro that can assist in verifying code coverage.  For production
** code the yytestcase() macro should be turned off.  But it is useful
** for testing.
*/
#ifndef yytestcase
#define yytestcase(X)
#endif

/* Macro to determine if stack space has the ability to grow using
** heap memory.
*/
#if YYSTACKDEPTH <= 0 || YYDYNSTACK
#define YYGROWABLESTACK 1
#else
#define YYGROWABLESTACK 0
#endif

/* Guarantee a minimum number of initial stack slots.
 */
#if YYSTACKDEPTH <= 0
#undef YYSTACKDEPTH
#define YYSTACKDEPTH 2 /* Need a minimum stack size */
#endif

/* Next are the tables used to determine what action to take based on the
** current state and lookahead token.  These tables are used to implement
** functions that take a state number and lookahead value and return an
** action integer.
**
** Suppose the action integer is N.  Then the action is determined as
** follows
**
**   0 <= N <= YY_MAX_SHIFT             Shift N.  That is, push the lookahead
**                                      token onto the stack and goto state N.
**
**   N between YY_MIN_SHIFTREDUCE       Shift to an arbitrary state then
**     and YY_MAX_SHIFTREDUCE           reduce by rule N-YY_MIN_SHIFTREDUCE.
**
**   N == YY_ERROR_ACTION               A syntax error has occurred.
**
**   N == YY_ACCEPT_ACTION              The parser accepts its input.
**
**   N == YY_NO_ACTION                  No such action.  Denotes unused
**                                      slots in the yy_action[] table.
**
**   N between YY_MIN_REDUCE            Reduce by rule N-YY_MIN_REDUCE
**     and YY_MAX_REDUCE
**
** The action table is constructed as a single large table named yy_action[].
** Given state S and lookahead X, the action is computed as either:
**
**    (A)   N = yy_action[ yy_shift_ofst[S] + X ]
**    (B)   N = yy_default[S]
**
** The (A) formula is preferred.  The B formula is used instead if
** yy_lookahead[yy_shift_ofst[S]+X] is not equal to X.
**
** The formulas above are for computing the action when the lookahead is
** a terminal symbol.  If the lookahead is a non-terminal (as occurs after
** a reduce action) then the yy_reduce_ofst[] array is used in place of
** the yy_shift_ofst[] array.
**
** The following are the tables generated in this section:
**
**  yy_action[]        A single table containing all actions.
**  yy_lookahead[]     A table containing the lookahead for each entry in
**                     yy_action.  Used to detect hash collisions.
**  yy_shift_ofst[]    For each state, the offset into yy_action for
**                     shifting terminals.
**  yy_reduce_ofst[]   For each state, the offset into yy_action for
**                     shifting non-terminals after a reduce.
**  yy_default[]       Default action for each state.
**
*********** Begin parsing tables **********************************************/
#define YY_ACTTAB_COUNT (2225)
static const YYACTIONTYPE yy_action[] = {
    /*     0 */ 7,
    1345,
    236,
    508,
    540,
    1068,
    1663,
    236,
    140,
    142,
    /*    10 */ 295,
    1527,
    530,
    140,
    142,
    1069,
    1602,
    295,
    1527,
    410,
    /*    20 */ 1077,
    1339,
    133,
    134,
    420,
    91,
    389,
    905,
    905,
    902,
    /*    30 */ 887,
    896,
    896,
    135,
    135,
    136,
    136,
    136,
    136,
    395,
    /*    40 */ 1078,
    217,
    528,
    1336,
    396,
    133,
    134,
    420,
    91,
    388,
    /*    50 */ 905,
    905,
    902,
    887,
    896,
    896,
    135,
    135,
    136,
    136,
    /*    60 */ 136,
    136,
    491,
    1008,
    295,
    1527,
    297,
    295,
    1527,
    306,
    /*    70 */ 136,
    136,
    136,
    136,
    139,
    1501,
    112,
    1009,
    408,
    291,
    /*    80 */ 290,
    1076,
    236,
    232,
    272,
    236,
    1470,
    232,
    140,
    142,
    /*    90 */ 1602,
    140,
    142,
    132,
    132,
    132,
    132,
    138,
    138,
    137,
    /*   100 */ 137,
    137,
    131,
    130,
    453,
    571,
    569,
    1601,
    464,
    465,
    /*   110 */ 1509,
    426,
    571,
    569,
    541,
    1655,
    132,
    132,
    132,
    132,
    /*   120 */ 138,
    138,
    137,
    137,
    137,
    131,
    130,
    453,
    132,
    132,
    /*   130 */ 132,
    132,
    138,
    138,
    137,
    137,
    137,
    131,
    130,
    453,
    /*   140 */ 493,
    111,
    133,
    134,
    420,
    91,
    453,
    905,
    905,
    902,
    /*   150 */ 887,
    896,
    896,
    135,
    135,
    136,
    136,
    136,
    136,
    571,
    /*   160 */ 569,
    1356,
    571,
    569,
    133,
    134,
    420,
    91,
    1342,
    905,
    /*   170 */ 905,
    902,
    887,
    896,
    896,
    135,
    135,
    136,
    136,
    136,
    /*   180 */ 136,
    1601,
    1599,
    1597,
    137,
    137,
    137,
    131,
    130,
    453,
    /*   190 */ 1365,
    116,
    1392,
    95,
    1390,
    1257,
    299,
    1257,
    132,
    132,
    /*   200 */ 132,
    132,
    138,
    138,
    137,
    137,
    137,
    131,
    130,
    453,
    /*   210 */ 404,
    1681,
    948,
    132,
    132,
    132,
    132,
    138,
    138,
    137,
    /*   220 */ 137,
    137,
    131,
    130,
    453,
    138,
    138,
    137,
    137,
    137,
    /*   230 */ 131,
    130,
    453,
    1004,
    493,
    132,
    132,
    132,
    132,
    138,
    /*   240 */ 138,
    137,
    137,
    137,
    131,
    130,
    453,
    133,
    134,
    420,
    /*   250 */ 91,
    399,
    905,
    905,
    902,
    887,
    896,
    896,
    135,
    135,
    /*   260 */ 136,
    136,
    136,
    136,
    1057,
    52,
    295,
    1527,
    1633,
    295,
    /*   270 */ 1527,
    1334,
    133,
    134,
    420,
    91,
    587,
    905,
    905,
    902,
    /*   280 */ 887,
    896,
    896,
    135,
    135,
    136,
    136,
    136,
    136,
    1057,
    /*   290 */ 302,
    1401,
    439,
    1401,
    271,
    133,
    134,
    420,
    91,
    579,
    /*   300 */ 905,
    905,
    902,
    887,
    896,
    896,
    135,
    135,
    136,
    136,
    /*   310 */ 136,
    136,
    161,
    324,
    1449,
    359,
    477,
    346,
    132,
    132,
    /*   320 */ 132,
    132,
    138,
    138,
    137,
    137,
    137,
    131,
    130,
    453,
    /*   330 */ 1348,
    1057,
    1058,
    1057,
    403,
    430,
    403,
    1347,
    1400,
    48,
    /*   340 */ 1399,
    579,
    96,
    132,
    132,
    132,
    132,
    138,
    138,
    137,
    /*   350 */ 137,
    137,
    131,
    130,
    453,
    324,
    1057,
    1058,
    1057,
    484,
    /*   360 */ 361,
    571,
    569,
    1252,
    571,
    569,
    132,
    132,
    132,
    132,
    /*   370 */ 138,
    138,
    137,
    137,
    137,
    131,
    130,
    453,
    553,
    495,
    /*   380 */ 1252,
    399,
    264,
    1252,
    520,
    517,
    516,
    343,
    372,
    1220,
    /*   390 */ 394,
    1057,
    1668,
    479,
    515,
    133,
    134,
    420,
    91,
    222,
    /*   400 */ 905,
    905,
    902,
    887,
    896,
    896,
    135,
    135,
    136,
    136,
    /*   410 */ 136,
    136,
    295,
    1527,
    580,
    1057,
    552,
    1382,
    133,
    134,
    /*   420 */ 420,
    91,
    448,
    905,
    905,
    902,
    887,
    896,
    896,
    135,
    /*   430 */ 135,
    136,
    136,
    136,
    136,
    295,
    1527,
    583,
    1163,
    1163,
    /*   440 */ 505,
    133,
    134,
    420,
    91,
    1252,
    905,
    905,
    902,
    887,
    /*   450 */ 896,
    896,
    135,
    135,
    136,
    136,
    136,
    136,
    1057,
    1058,
    /*   460 */ 1057,
    1214,
    1252,
    386,
    200,
    1252,
    132,
    132,
    132,
    132,
    /*   470 */ 138,
    138,
    137,
    137,
    137,
    131,
    130,
    453,
    237,
    549,
    /*   480 */ 1630,
    325,
    1057,
    1058,
    1057,
    1057,
    1648,
    406,
    521,
    132,
    /*   490 */ 132,
    132,
    132,
    138,
    138,
    137,
    137,
    137,
    131,
    130,
    /*   500 */ 453,
    45,
    116,
    131,
    130,
    453,
    493,
    571,
    569,
    1057,
    /*   510 */ 535,
    939,
    132,
    132,
    132,
    132,
    138,
    138,
    137,
    137,
    /*   520 */ 137,
    131,
    130,
    453,
    424,
    46,
    906,
    906,
    903,
    888,
    /*   530 */ 571,
    569,
    1057,
    6,
    931,
    133,
    134,
    420,
    91,
    1149,
    /*   540 */ 905,
    905,
    902,
    887,
    896,
    896,
    135,
    135,
    136,
    136,
    /*   550 */ 136,
    136,
    1057,
    1058,
    1057,
    1296,
    589,
    426,
    133,
    134,
    /*   560 */ 420,
    91,
    305,
    905,
    905,
    902,
    887,
    896,
    896,
    135,
    /*   570 */ 135,
    136,
    136,
    136,
    136,
    501,
    1057,
    1058,
    1057,
    1621,
    /*   580 */ 472,
    133,
    134,
    420,
    91,
    482,
    905,
    905,
    902,
    887,
    /*   590 */ 896,
    896,
    135,
    135,
    136,
    136,
    136,
    136,
    1401,
    1057,
    /*   600 */ 1058,
    1057,
    463,
    1572,
    1280,
    201,
    132,
    132,
    132,
    132,
    /*   610 */ 138,
    138,
    137,
    137,
    137,
    131,
    130,
    453,
    268,
    212,
    /*   620 */ 295,
    1527,
    1525,
    483,
    897,
    1121,
    317,
    1057,
    168,
    132,
    /*   630 */ 132,
    132,
    132,
    138,
    138,
    137,
    137,
    137,
    131,
    130,
    /*   640 */ 453,
    403,
    217,
    1557,
    50,
    1399,
    313,
    588,
    157,
    571,
    /*   650 */ 569,
    1270,
    132,
    132,
    132,
    132,
    138,
    138,
    137,
    137,
    /*   660 */ 137,
    131,
    130,
    453,
    252,
    582,
    436,
    884,
    884,
    1383,
    /*   670 */ 310,
    1057,
    540,
    508,
    1052,
    133,
    134,
    420,
    91,
    418,
    /*   680 */ 905,
    905,
    902,
    887,
    896,
    896,
    135,
    135,
    136,
    136,
    /*   690 */ 136,
    136,
    1057,
    1121,
    1057,
    1058,
    1057,
    425,
    133,
    134,
    /*   700 */ 420,
    91,
    276,
    905,
    905,
    902,
    887,
    896,
    896,
    135,
    /*   710 */ 135,
    136,
    136,
    136,
    136,
    571,
    569,
    462,
    1057,
    233,
    /*   720 */ 1401,
    133,
    134,
    420,
    91,
    493,
    905,
    905,
    902,
    887,
    /*   730 */ 896,
    896,
    135,
    135,
    136,
    136,
    136,
    136,
    1057,
    1058,
    /*   740 */ 1057,
    1239,
    1609,
    1610,
    5,
    1500,
    132,
    132,
    132,
    132,
    /*   750 */ 138,
    138,
    137,
    137,
    137,
    131,
    130,
    453,
    1471,
    1057,
    /*   760 */ 1058,
    1057,
    405,
    403,
    1238,
    3,
    47,
    1399,
    1035,
    132,
    /*   770 */ 132,
    132,
    132,
    138,
    138,
    137,
    137,
    137,
    131,
    130,
    /*   780 */ 453,
    307,
    543,
    1656,
    456,
    1057,
    1058,
    1057,
    129,
    1401,
    /*   790 */ 123,
    42,
    132,
    132,
    132,
    132,
    138,
    138,
    137,
    137,
    /*   800 */ 137,
    131,
    130,
    453,
    1124,
    236,
    121,
    44,
    86,
    1123,
    /*   810 */ 531,
    140,
    142,
    186,
    1075,
    133,
    134,
    420,
    91,
    587,
    /*   820 */ 905,
    905,
    902,
    887,
    896,
    896,
    135,
    135,
    136,
    136,
    /*   830 */ 136,
    136,
    403,
    1234,
    586,
    49,
    1399,
    133,
    134,
    420,
    /*   840 */ 91,
    499,
    905,
    905,
    902,
    887,
    896,
    896,
    135,
    135,
    /*   850 */ 136,
    136,
    136,
    136,
    1236,
    20,
    1645,
    1449,
    431,
    1645,
    /*   860 */ 1057,
    398,
    133,
    134,
    420,
    91,
    587,
    905,
    905,
    902,
    /*   870 */ 887,
    896,
    896,
    135,
    135,
    136,
    136,
    136,
    136,
    1609,
    /*   880 */ 1610,
    1555,
    404,
    1681,
    563,
    579,
    132,
    132,
    132,
    132,
    /*   890 */ 138,
    138,
    137,
    137,
    137,
    131,
    130,
    453,
    499,
    324,
    /*   900 */ 1615,
    292,
    1527,
    159,
    1449,
    293,
    1527,
    16,
    132,
    132,
    /*   910 */ 132,
    132,
    138,
    138,
    137,
    137,
    137,
    131,
    130,
    453,
    /*   920 */ 7,
    1684,
    508,
    128,
    301,
    538,
    1662,
    1057,
    1058,
    1057,
    /*   930 */ 547,
    1421,
    219,
    132,
    132,
    132,
    132,
    138,
    138,
    137,
    /*   940 */ 137,
    137,
    131,
    130,
    453,
    328,
    133,
    141,
    420,
    91,
    /*   950 */ 1422,
    905,
    905,
    902,
    887,
    896,
    896,
    135,
    135,
    136,
    /*   960 */ 136,
    136,
    136,
    134,
    420,
    91,
    93,
    905,
    905,
    902,
    /*   970 */ 887,
    896,
    896,
    135,
    135,
    136,
    136,
    136,
    136,
    420,
    /*   980 */ 91,
    358,
    905,
    905,
    902,
    887,
    896,
    896,
    135,
    135,
    /*   990 */ 136,
    136,
    136,
    136,
    429,
    467,
    571,
    569,
    52,
    329,
    /*  1000 */ 571,
    569,
    590,
    1004,
    136,
    136,
    136,
    136,
    508,
    587,
    /*  1010 */ 1339,
    1394,
    237,
    549,
    44,
    125,
    39,
    132,
    132,
    132,
    /*  1020 */ 132,
    138,
    138,
    137,
    137,
    137,
    131,
    130,
    453,
    44,
    /*  1030 */ 1422,
    459,
    1337,
    132,
    132,
    132,
    132,
    138,
    138,
    137,
    /*  1040 */ 137,
    137,
    131,
    130,
    453,
    1369,
    576,
    1449,
    132,
    132,
    /*  1050 */ 132,
    132,
    138,
    138,
    137,
    137,
    137,
    131,
    130,
    453,
    /*  1060 */ 44,
    590,
    132,
    132,
    132,
    132,
    138,
    138,
    137,
    137,
    /*  1070 */ 137,
    131,
    130,
    453,
    125,
    417,
    416,
    186,
    7,
    9,
    /*  1080 */ 1499,
    1063,
    52,
    934,
    1660,
    359,
    86,
    1368,
    127,
    127,
    /*  1090 */ 459,
    107,
    579,
    587,
    471,
    38,
    126,
    587,
    459,
    577,
    /*  1100 */ 459,
    1059,
    1061,
    489,
    4,
    576,
    324,
    579,
    264,
    330,
    /*  1110 */ 520,
    517,
    516,
    455,
    454,
    584,
    1061,
    1252,
    561,
    34,
    /*  1120 */ 515,
    324,
    118,
    557,
    555,
    1668,
    1192,
    1192,
    508,
    554,
    /*  1130 */ 86,
    1449,
    562,
    1063,
    1252,
    1449,
    590,
    1252,
    579,
    1284,
    /*  1140 */ 1063,
    587,
    1061,
    1062,
    1064,
    1057,
    224,
    127,
    127,
    125,
    /*  1150 */ 934,
    1060,
    324,
    1059,
    1061,
    126,
    84,
    459,
    577,
    459,
    /*  1160 */ 1059,
    1061,
    536,
    4,
    955,
    459,
    1252,
    587,
    1061,
    512,
    /*  1170 */ 1234,
    875,
    268,
    956,
    584,
    1061,
    234,
    401,
    34,
    1449,
    /*  1180 */ 576,
    591,
    444,
    1252,
    7,
    1147,
    1252,
    427,
    187,
    550,
    /*  1190 */ 1662,
    1236,
    1282,
    1646,
    1061,
    1062,
    1646,
    499,
    1572,
    555,
    /*  1200 */ 1498,
    1061,
    1062,
    1064,
    556,
    1449,
    563,
    1523,
    393,
    1670,
    /*  1210 */ 1572,
    590,
    1057,
    1058,
    1057,
    1063,
    957,
    559,
    20,
    551,
    /*  1220 */ 532,
    434,
    127,
    127,
    125,
    1275,
    594,
    593,
    1280,
    587,
    /*  1230 */ 126,
    1147,
    459,
    577,
    459,
    1059,
    1061,
    1160,
    4,
    875,
    /*  1240 */ 459,
    1160,
    523,
    558,
    295,
    1527,
    1525,
    217,
    1367,
    584,
    /*  1250 */ 1061,
    425,
    1263,
    34,
    524,
    576,
    368,
    86,
    364,
    458,
    /*  1260 */ 532,
    437,
    441,
    404,
    1681,
    958,
    1572,
    1449,
    587,
    338,
    /*  1270 */ 313,
    340,
    157,
    438,
    555,
    376,
    1061,
    1062,
    1064,
    554,
    /*  1280 */ 308,
    503,
    385,
    384,
    413,
    1263,
    430,
    1057,
    252,
    375,
    /*  1290 */ 1063,
    270,
    286,
    526,
    379,
    525,
    269,
    127,
    127,
    497,
    /*  1300 */ 1572,
    1121,
    375,
    533,
    473,
    126,
    1449,
    459,
    577,
    459,
    /*  1310 */ 1059,
    1061,
    1522,
    4,
    1274,
    1430,
    404,
    1681,
    239,
    876,
    /*  1320 */ 327,
    508,
    283,
    370,
    584,
    1061,
    475,
    419,
    34,
    504,
    /*  1330 */ 326,
    1057,
    334,
    563,
    461,
    2,
    341,
    590,
    215,
    571,
    /*  1340 */ 569,
    462,
    108,
    533,
    1063,
    1524,
    877,
    98,
    221,
    442,
    /*  1350 */ 125,
    1061,
    1062,
    1064,
    1057,
    1058,
    1057,
    430,
    6,
    339,
    /*  1360 */ 1519,
    147,
    1060,
    445,
    1059,
    1061,
    459,
    1381,
    240,
    1121,
    /*  1370 */ 969,
    331,
    587,
    86,
    10,
    565,
    333,
    242,
    276,
    1061,
    /*  1380 */ 178,
    576,
    498,
    43,
    587,
    234,
    230,
    876,
    295,
    1527,
    /*  1390 */ 564,
    288,
    163,
    366,
    1147,
    225,
    62,
    151,
    1057,
    1058,
    /*  1400 */ 1057,
    534,
    238,
    1504,
    22,
    1061,
    1062,
    587,
    587,
    216,
    /*  1410 */ 1449,
    295,
    1527,
    585,
    590,
    542,
    1063,
    20,
    20,
    1503,
    /*  1420 */ 440,
    309,
    1449,
    127,
    127,
    1099,
    862,
    90,
    587,
    587,
    /*  1430 */ 1101,
    126,
    7,
    459,
    577,
    459,
    1059,
    1061,
    1661,
    4,
    /*  1440 */ 1147,
    942,
    1077,
    459,
    561,
    1449,
    1449,
    590,
    500,
    563,
    /*  1450 */ 584,
    1061,
    573,
    1449,
    34,
    1148,
    1100,
    1093,
    576,
    1068,
    /*  1460 */ 125,
    579,
    1078,
    267,
    266,
    265,
    1449,
    1449,
    421,
    1069,
    /*  1470 */ 469,
    20,
    566,
    334,
    209,
    324,
    459,
    1061,
    1062,
    1064,
    /*  1480 */ 1219,
    300,
    587,
    571,
    569,
    575,
    1158,
    20,
    67,
    7,
    /*  1490 */ 210,
    576,
    1430,
    1063,
    963,
    1659,
    7,
    160,
    587,
    587,
    /*  1500 */ 127,
    127,
    1658,
    1076,
    86,
    468,
    571,
    569,
    126,
    942,
    /*  1510 */ 459,
    577,
    459,
    1059,
    1061,
    587,
    4,
    1239,
    1502,
    20,
    /*  1520 */ 1449,
    1428,
    560,
    476,
    419,
    964,
    1063,
    584,
    1061,
    572,
    /*  1530 */ 587,
    34,
    230,
    127,
    127,
    1116,
    1449,
    1449,
    405,
    108,
    /*  1540 */ 1235,
    126,
    485,
    459,
    577,
    459,
    1059,
    1061,
    20,
    4,
    /*  1550 */ 356,
    457,
    294,
    1449,
    1061,
    1062,
    1064,
    590,
    1472,
    587,
    /*  1560 */ 584,
    1061,
    109,
    86,
    34,
    239,
    1427,
    327,
    1449,
    283,
    /*  1570 */ 125,
    86,
    86,
    248,
    587,
    486,
    419,
    326,
    207,
    334,
    /*  1580 */ 319,
    461,
    587,
    587,
    86,
    68,
    459,
    1061,
    1062,
    1064,
    /*  1590 */ 1117,
    69,
    21,
    53,
    383,
    587,
    587,
    1449,
    534,
    225,
    /*  1600 */ 1124,
    576,
    587,
    587,
    587,
    1123,
    1675,
    490,
    419,
    484,
    /*  1610 */ 361,
    1228,
    1449,
    382,
    407,
    240,
    296,
    54,
    331,
    351,
    /*  1620 */ 1449,
    1449,
    116,
    333,
    242,
    474,
    357,
    178,
    587,
    116,
    /*  1630 */ 43,
    318,
    419,
    1449,
    1449,
    249,
    1063,
    251,
    432,
    450,
    /*  1640 */ 1449,
    1449,
    1449,
    127,
    127,
    1071,
    1072,
    451,
    452,
    238,
    /*  1650 */ 487,
    126,
    1644,
    459,
    577,
    459,
    1059,
    1061,
    303,
    4,
    /*  1660 */ 320,
    70,
    995,
    592,
    164,
    71,
    1449,
    72,
    349,
    73,
    /*  1670 */ 584,
    1061,
    587,
    862,
    34,
    239,
    587,
    327,
    587,
    283,
    /*  1680 */ 587,
    337,
    74,
    55,
    1057,
    1025,
    480,
    326,
    274,
    334,
    /*  1690 */ 1193,
    1193,
    492,
    587,
    587,
    274,
    352,
    1061,
    1062,
    1064,
    /*  1700 */ 494,
    574,
    513,
    274,
    373,
    279,
    1590,
    116,
    579,
    24,
    /*  1710 */ 1449,
    1191,
    1191,
    567,
    1449,
    421,
    1449,
    469,
    1449,
    1588,
    /*  1720 */ 334,
    1065,
    324,
    1152,
    56,
    240,
    274,
    1219,
    331,
    363,
    /*  1730 */ 995,
    1449,
    1449,
    333,
    242,
    587,
    991,
    178,
    57,
    279,
    /*  1740 */ 43,
    455,
    454,
    1272,
    1,
    1276,
    594,
    593,
    1280,
    587,
    /*  1750 */ 1188,
    1057,
    1058,
    1057,
    1192,
    1192,
    58,
    1495,
    75,
    238,
    /*  1760 */ 76,
    367,
    369,
    77,
    295,
    1527,
    1525,
    587,
    371,
    587,
    /*  1770 */ 1386,
    587,
    78,
    1449,
    587,
    79,
    988,
    51,
    1190,
    1366,
    /*  1780 */ 59,
    378,
    60,
    587,
    19,
    1189,
    587,
    1449,
    587,
    1065,
    /*  1790 */ 313,
    587,
    157,
    587,
    1224,
    587,
    1344,
    118,
    1223,
    61,
    /*  1800 */ 1222,
    118,
    1338,
    118,
    1569,
    1449,
    335,
    1449,
    252,
    1449,
    /*  1810 */ 587,
    873,
    1449,
    169,
    162,
    1571,
    116,
    1535,
    579,
    1308,
    /*  1820 */ 400,
    1449,
    1295,
    80,
    1449,
    421,
    1449,
    469,
    145,
    1449,
    /*  1830 */ 334,
    1449,
    324,
    1449,
    587,
    146,
    81,
    1219,
    509,
    587,
    /*  1840 */ 284,
    227,
    63,
    82,
    170,
    12,
    587,
    587,
    1449,
    354,
    /*  1850 */ 314,
    64,
    88,
    587,
    587,
    2,
    173,
    1416,
    315,
    571,
    /*  1860 */ 569,
    462,
    587,
    587,
    174,
    345,
    298,
    587,
    83,
    65,
    /*  1870 */ 171,
    172,
    1449,
    316,
    1530,
    587,
    247,
    1449,
    153,
    587,
    /*  1880 */ 587,
    587,
    587,
    348,
    1449,
    1449,
    360,
    355,
    87,
    587,
    /*  1890 */ 1444,
    1449,
    1449,
    89,
    1443,
    411,
    304,
    496,
    381,
    587,
    /*  1900 */ 1449,
    1449,
    1331,
    148,
    587,
    1449,
    518,
    1365,
    231,
    1407,
    /*  1910 */ 152,
    165,
    154,
    1449,
    587,
    149,
    1560,
    1449,
    1449,
    1449,
    /*  1920 */ 1449,
    587,
    587,
    587,
    1561,
    392,
    587,
    1449,
    144,
    150,
    /*  1930 */ 85,
    1559,
    66,
    93,
    226,
    1558,
    1408,
    1449,
    1622,
    587,
    /*  1940 */ 587,
    587,
    1449,
    587,
    578,
    1202,
    94,
    213,
    278,
    214,
    /*  1950 */ 1614,
    1612,
    1449,
    97,
    428,
    1105,
    244,
    241,
    1093,
    1449,
    /*  1960 */ 1449,
    1449,
    470,
    223,
    1449,
    198,
    243,
    466,
    113,
    1511,
    /*  1970 */ 1417,
    1510,
    1415,
    190,
    191,
    192,
    189,
    1449,
    1449,
    1449,
    /*  1980 */ 245,
    1449,
    193,
    13,
    561,
    344,
    183,
    195,
    1414,
    347,
    /*  1990 */ 478,
    511,
    254,
    109,
    1628,
    256,
    481,
    211,
    409,
    1418,
    /*  2000 */ 488,
    362,
    1446,
    202,
    14,
    1445,
    502,
    259,
    103,
    507,
    /*  2010 */ 285,
    261,
    1574,
    262,
    412,
    365,
    1332,
    527,
    414,
    1389,
    /*  2020 */ 1388,
    1387,
    443,
    1376,
    415,
    105,
    1353,
    948,
    1359,
    1358,
    /*  2030 */ 1352,
    1375,
    380,
    1351,
    40,
    1350,
    275,
    1529,
    1528,
    529,
    /*  2040 */ 387,
    218,
    110,
    537,
    390,
    391,
    1666,
    277,
    446,
    11,
    /*  2050 */ 1306,
    1481,
    397,
    447,
    1482,
    122,
    323,
    449,
    402,
    321,
    /*  2060 */ 1665,
    237,
    322,
    422,
    581,
    423,
    188,
    175,
    1594,
    1595,
    /*  2070 */ 1593,
    1592,
    1680,
    176,
    158,
    311,
    228,
    229,
    177,
    92,
    /*  2080 */ 1212,
    220,
    460,
    1210,
    1185,
    1183,
    332,
    156,
    336,
    1081,
    /*  2090 */ 246,
    342,
    194,
    179,
    350,
    250,
    253,
    1117,
    23,
    196,
    /*  2100 */ 255,
    1171,
    353,
    180,
    181,
    433,
    435,
    197,
    99,
    199,
    /*  2110 */ 100,
    101,
    102,
    1176,
    257,
    258,
    182,
    1170,
    166,
    15,
    /*  2120 */ 1161,
    274,
    203,
    506,
    1167,
    204,
    260,
    510,
    1217,
    263,
    /*  2130 */ 382,
    205,
    514,
    104,
    25,
    519,
    26,
    374,
    946,
    522,
    /*  2140 */ 959,
    377,
    106,
    312,
    184,
    185,
    1155,
    1150,
    155,
    287,
    /*  2150 */ 289,
    273,
    27,
    206,
    118,
    41,
    1242,
    235,
    114,
    544,
    /*  2160 */ 539,
    167,
    208,
    545,
    546,
    115,
    548,
    1268,
    28,
    29,
    /*  2170 */ 30,
    1258,
    1254,
    8,
    1256,
    1262,
    116,
    31,
    1261,
    117,
    /*  2180 */ 895,
    890,
    889,
    119,
    989,
    32,
    33,
    568,
    909,
    570,
    /*  2190 */ 36,
    37,
    1273,
    1273,
    1273,
    1074,
    280,
    124,
    35,
    17,
    /*  2200 */ 1273,
    1473,
    120,
    983,
    886,
    885,
    18,
    1273,
    883,
    1273,
    /*  2210 */ 281,
    1273,
    1273,
    1273,
    1273,
    1273,
    874,
    870,
    1273,
    282,
    /*  2220 */ 143,
    864,
    1273,
    1273,
    863,
};
static const YYCODETYPE yy_lookahead[] = {
    /*     0 */ 312, 243, 207, 205, 205, 5,   318, 207, 213, 214,
    /*    10 */ 209, 210, 211, 213, 214, 15,  205, 209, 210, 211,
    /*    20 */ 1,   205, 22,  23,  24,  25,  259, 27,  28,  29,
    /*    30 */ 30,  31,  32,  33,  34,  35,  36,  37,  38,  244,
    /*    40 */ 21,  205, 226, 227, 244, 22,  23,  24,  25,  282,
    /*    50 */ 27,  28,  29,  30,  31,  32,  33,  34,  35,  36,
    /*    60 */ 37,  38,  254, 44,  209, 210, 211, 209, 210, 211,
    /*    70 */ 35,  36,  37,  38,  39,  277, 53,  58,  242, 204,
    /*    80 */ 283, 62,  207, 286, 283, 207, 198, 286, 213, 214,
    /*    90 */ 205, 213, 214, 93,  94,  95,  96,  97,  98,  99,
    /*   100 */ 100, 101, 102, 103, 104, 304, 305, 296, 297, 298,
    /*   110 */ 199, 200, 304, 305, 315, 316, 93,  94,  95,  96,
    /*   120 */ 97,  98,  99,  100, 101, 102, 103, 104, 93,  94,
    /*   130 */ 95,  96,  97,  98,  99,  100, 101, 102, 103, 104,
    /*   140 */ 205, 118, 22,  23,  24,  25,  104, 27,  28,  29,
    /*   150 */ 30,  31,  32,  33,  34,  35,  36,  37,  38,  304,
    /*   160 */ 305, 223, 304, 305, 22,  23,  24,  25,  230, 27,
    /*   170 */ 28,  29,  30,  31,  32,  33,  34,  35,  36,  37,
    /*   180 */ 38,  296, 297, 298, 99,  100, 101, 102, 103, 104,
    /*   190 */ 221, 118, 223, 51,  225, 75,  261, 77,  93,  94,
    /*   200 */ 95,  96,  97,  98,  99,  100, 101, 102, 103, 104,
    /*   210 */ 322, 323, 139, 93,  94,  95,  96,  97,  98,  99,
    /*   220 */ 100, 101, 102, 103, 104, 97,  98,  99,  100, 101,
    /*   230 */ 102, 103, 104, 58,  205, 93,  94,  95,  96,  97,
    /*   240 */ 98,  99,  100, 101, 102, 103, 104, 22,  23,  24,
    /*   250 */ 25,  205, 27,  28,  29,  30,  31,  32,  33,  34,
    /*   260 */ 35,  36,  37,  38,  40,  194, 209, 210, 211, 209,
    /*   270 */ 210, 211, 22,  23,  24,  25,  205, 27,  28,  29,
    /*   280 */ 30,  31,  32,  33,  34,  35,  36,  37,  38,  40,
    /*   290 */ 261, 205, 246, 205, 70,  22,  23,  24,  25,  147,
    /*   300 */ 27,  28,  29,  30,  31,  32,  33,  34,  35,  36,
    /*   310 */ 37,  38,  118, 161, 243, 140, 141, 142, 93,  94,
    /*   320 */ 95,  96,  97,  98,  99,  100, 101, 102, 103, 104,
    /*   330 */ 243, 107, 108, 109, 248, 205, 248, 243, 252, 251,
    /*   340 */ 252, 147, 117, 93,  94,  95,  96,  97,  98,  99,
    /*   350 */ 100, 101, 102, 103, 104, 161, 107, 108, 109, 141,
    /*   360 */ 142, 304, 305, 61,  304, 305, 93,  94,  95,  96,
    /*   370 */ 97,  98,  99,  100, 101, 102, 103, 104, 76,  205,
    /*   380 */ 78,  205, 133, 81,  135, 136, 137, 257, 115, 64,
    /*   390 */ 319, 40,  321, 68,  145, 22,  23,  24,  25,  149,
    /*   400 */ 27,  28,  29,  30,  31,  32,  33,  34,  35,  36,
    /*   410 */ 37,  38,  209, 210, 211, 40,  114, 232, 22,  23,
    /*   420 */ 24,  25,  246, 27,  28,  29,  30,  31,  32,  33,
    /*   430 */ 34,  35,  36,  37,  38,  209, 210, 211, 140, 141,
    /*   440 */ 142, 22,  23,  24,  25,  61,  27,  28,  29,  30,
    /*   450 */ 31,  32,  33,  34,  35,  36,  37,  38,  107, 108,
    /*   460 */ 109, 136, 78,  205, 113, 81,  93,  94,  95,  96,
    /*   470 */ 97,  98,  99,  100, 101, 102, 103, 104, 168, 169,
    /*   480 */ 155, 205, 107, 108, 109, 40,  309, 310, 115, 93,
    /*   490 */ 94,  95,  96,  97,  98,  99,  100, 101, 102, 103,
    /*   500 */ 104, 56,  118, 102, 103, 104, 205, 304, 305, 40,
    /*   510 */ 205, 115, 93,  94,  95,  96,  97,  98,  99,  100,
    /*   520 */ 101, 102, 103, 104, 238, 56,  27,  28,  29,  30,
    /*   530 */ 304, 305, 40,  204, 115, 22,  23,  24,  25,  164,
    /*   540 */ 27,  28,  29,  30,  31,  32,  33,  34,  35,  36,
    /*   550 */ 37,  38,  107, 108, 109, 210, 199, 200, 22,  23,
    /*   560 */ 24,  25,  261, 27,  28,  29,  30,  31,  32,  33,
    /*   570 */ 34,  35,  36,  37,  38,  289, 107, 108, 109, 303,
    /*   580 */ 254, 22,  23,  24,  25,  299, 27,  28,  29,  30,
    /*   590 */ 31,  32,  33,  34,  35,  36,  37,  38,  205, 107,
    /*   600 */ 108, 109, 191, 205, 193, 113, 93,  94,  95,  96,
    /*   610 */ 97,  98,  99,  100, 101, 102, 103, 104, 27,  290,
    /*   620 */ 209, 210, 211, 294, 125, 40,  267, 40,  115, 93,
    /*   630 */ 94,  95,  96,  97,  98,  99,  100, 101, 102, 103,
    /*   640 */ 104, 248, 205, 284, 251, 252, 235, 201, 237, 304,
    /*   650 */ 305, 115, 93,  94,  95,  96,  97,  98,  99,  100,
    /*   660 */ 101, 102, 103, 104, 253, 120, 268, 122, 123, 232,
    /*   670 */ 233, 40,  205, 205, 115, 22,  23,  24,  25,  242,
    /*   680 */ 27,  28,  29,  30,  31,  32,  33,  34,  35,  36,
    /*   690 */ 37,  38,  40,  108, 107, 108, 109, 106, 22,  23,
    /*   700 */ 24,  25,  117, 27,  28,  29,  30,  31,  32,  33,
    /*   710 */ 34,  35,  36,  37,  38,  304, 305, 306, 40,  205,
    /*   720 */ 205, 22,  23,  24,  25,  205, 27,  28,  29,  30,
    /*   730 */ 31,  32,  33,  34,  35,  36,  37,  38,  107, 108,
    /*   740 */ 109, 92,  296, 297, 113, 277, 93,  94,  95,  96,
    /*   750 */ 97,  98,  99,  100, 101, 102, 103, 104, 198, 107,
    /*   760 */ 108, 109, 113, 248, 115, 113, 251, 252, 115, 93,
    /*   770 */ 94,  95,  96,  97,  98,  99,  100, 101, 102, 103,
    /*   780 */ 104, 261, 99,  316, 201, 107, 108, 109, 129, 205,
    /*   790 */ 131, 113, 93,  94,  95,  96,  97,  98,  99,  100,
    /*   800 */ 101, 102, 103, 104, 121, 207, 130, 69,  194, 126,
    /*   810 */ 196, 213, 214, 205, 115, 22,  23,  24,  25,  205,
    /*   820 */ 27,  28,  29,  30,  31,  32,  33,  34,  35,  36,
    /*   830 */ 37,  38,  248, 92,  205, 251, 252, 22,  23,  24,
    /*   840 */ 25,  205, 27,  28,  29,  30,  31,  32,  33,  34,
    /*   850 */ 35,  36,  37,  38,  113, 194, 115, 243, 197, 118,
    /*   860 */ 40,  205, 22,  23,  24,  25,  205, 27,  28,  29,
    /*   870 */ 30,  31,  32,  33,  34,  35,  36,  37,  38,  296,
    /*   880 */ 297, 285, 322, 323, 270, 147, 93,  94,  95,  96,
    /*   890 */ 97,  98,  99,  100, 101, 102, 103, 104, 205, 161,
    /*   900 */ 302, 209, 210, 113, 243, 209, 210, 117, 93,  94,
    /*   910 */ 95,  96,  97,  98,  99,  100, 101, 102, 103, 104,
    /*   920 */ 312, 205, 205, 130, 288, 317, 318, 107, 108, 109,
    /*   930 */ 86,  247, 117, 93,  94,  95,  96,  97,  98,  99,
    /*   940 */ 100, 101, 102, 103, 104, 205, 22,  23,  24,  25,
    /*   950 */ 266, 27,  28,  29,  30,  31,  32,  33,  34,  35,
    /*   960 */ 36,  37,  38,  23,  24,  25,  146, 27,  28,  29,
    /*   970 */ 30,  31,  32,  33,  34,  35,  36,  37,  38,  24,
    /*   980 */ 25,  288, 27,  28,  29,  30,  31,  32,  33,  34,
    /*   990 */ 35,  36,  37,  38,  277, 278, 304, 305, 194, 205,
    /*  1000 */ 304, 305, 11,  58,  35,  36,  37,  38,  205, 205,
    /*  1010 */ 205, 247, 168, 169, 69,  24,  34,  93,  94,  95,
    /*  1020 */ 96,  97,  98,  99,  100, 101, 102, 103, 104, 69,
    /*  1030 */ 266, 40,  227, 93,  94,  95,  96,  97,  98,  99,
    /*  1040 */ 100, 101, 102, 103, 104, 222, 55,  243, 93,  94,
    /*  1050 */ 95,  96,  97,  98,  99,  100, 101, 102, 103, 104,
    /*  1060 */ 69,  11,  93,  94,  95,  96,  97,  98,  99,  100,
    /*  1070 */ 101, 102, 103, 104, 24,  97,  98,  205, 312, 29,
    /*  1080 */ 277, 90,  194, 40,  318, 140, 194, 222, 97,  98,
    /*  1090 */ 40,  113, 147, 205, 134, 113, 105, 205, 107, 108,
    /*  1100 */ 109, 110, 111, 106, 113, 55,  161, 147, 133, 205,
    /*  1110 */ 135, 136, 137, 97,  98,  124, 125, 61,  114, 128,
    /*  1120 */ 145, 161, 118, 319, 74,  321, 110, 111, 205, 79,
    /*  1130 */ 194, 243, 196, 90,  78,  243, 11,  81,  147, 198,
    /*  1140 */ 90,  205, 151, 152, 153, 40,  149, 97,  98,  24,
    /*  1150 */ 107, 108, 161, 110, 111, 105, 194, 107, 108, 109,
    /*  1160 */ 110, 111, 270, 113, 125, 40,  61,  205, 125, 24,
    /*  1170 */ 92,  40,  27,  134, 124, 125, 109, 215, 128, 243,
    /*  1180 */ 55,  76,  24,  78,  312, 118, 81,  307, 308, 317,
    /*  1190 */ 318, 113, 198, 115, 151, 152, 118, 205, 205, 74,
    /*  1200 */ 277, 151, 152, 153, 79,  243, 270, 205, 320, 321,
    /*  1210 */ 205, 11,  107, 108, 109, 90,  10,  48,  194, 114,
    /*  1220 */ 24,  197, 97,  98,  24,  190, 191, 192, 193, 205,
    /*  1230 */ 105, 164, 107, 108, 109, 110, 111, 3,   113, 108,
    /*  1240 */ 40,  7,   84,  74,  209, 210, 211, 205, 222, 124,
    /*  1250 */ 125, 106, 83,  128, 48,  55,  65,  194, 67,  196,
    /*  1260 */ 24,  268, 104, 322, 323, 59,  205, 243, 205, 65,
    /*  1270 */ 235, 67,  237, 268, 74,  117, 151, 152, 153, 79,
    /*  1280 */ 288, 47,  240, 125, 242, 116, 205, 40,  253, 144,
    /*  1290 */ 90,  133, 134, 135, 136, 137, 138, 97,  98,  24,
    /*  1300 */ 205, 40,  144, 107, 256, 105, 243, 107, 108, 109,
    /*  1310 */ 110, 111, 205, 113, 0,   267, 322, 323, 4,   40,
    /*  1320 */ 6,   205, 8,   132, 124, 125, 202, 203, 128, 268,
    /*  1330 */ 16,  40,  18,  270, 20,  300, 132, 11,  257, 304,
    /*  1340 */ 305, 306, 106, 107, 90,  205, 99,  148, 149, 143,
    /*  1350 */ 24,  151, 152, 153, 107, 108, 109, 205, 204, 155,
    /*  1360 */ 205, 194, 108, 268, 110, 111, 40,  115, 54,  108,
    /*  1370 */ 118, 57,  205, 194, 113, 196, 62,  63,  117, 125,
    /*  1380 */ 66,  55,  107, 69,  205, 109, 118, 108, 209, 210,
    /*  1390 */ 211, 115, 113, 277, 118, 127, 194, 194, 107, 108,
    /*  1400 */ 109, 165, 88,  205, 194, 151, 152, 205, 205, 257,
    /*  1410 */ 243, 209, 210, 211, 11,  205, 90,  194, 194, 205,
    /*  1420 */ 197, 197, 243, 97,  98,  14,  112, 24,  205, 205,
    /*  1430 */ 19,  105, 312, 107, 108, 109, 110, 111, 318, 113,
    /*  1440 */ 164, 40,  1,   40,  114, 243, 243, 11,  294, 270,
    /*  1450 */ 124, 125, 126, 243, 128, 164, 45,  41,  55,  5,
    /*  1460 */ 24,  147, 21,  140, 141, 142, 243, 243, 154, 15,
    /*  1470 */ 156, 194, 270, 159, 197, 161, 40,  151, 152, 153,
    /*  1480 */ 166, 256, 205, 304, 305, 44,  18,  194, 194, 312,
    /*  1490 */ 197, 55,  267, 90,  14,  318, 312, 167, 205, 205,
    /*  1500 */ 97,  98,  318, 62,  194, 205, 304, 305, 105, 108,
    /*  1510 */ 107, 108, 109, 110, 111, 205, 113, 92,  205, 194,
    /*  1520 */ 243, 205, 197, 202, 203, 45,  90,  124, 125, 126,
    /*  1530 */ 205, 128, 118, 97,  98,  99,  243, 243, 113, 106,
    /*  1540 */ 115, 105, 142, 107, 108, 109, 110, 111, 194, 113,
    /*  1550 */ 150, 197, 113, 243, 151, 152, 153, 11,  119, 205,
    /*  1560 */ 124, 125, 148, 194, 128, 4,   205, 6,   243, 8,
    /*  1570 */ 24,  194, 194, 157, 205, 202, 203, 16,  279, 18,
    /*  1580 */ 270, 20,  205, 205, 194, 194, 40,  151, 152, 153,
    /*  1590 */ 118, 194, 194, 194, 125, 205, 205, 243, 165, 127,
    /*  1600 */ 121, 55,  205, 205, 205, 126, 314, 202, 203, 141,
    /*  1610 */ 142, 115, 243, 144, 118, 54,  89,  194, 57,  115,
    /*  1620 */ 243, 243, 118, 62,  63,  205, 115, 66,  205, 118,
    /*  1630 */ 69,  202, 203, 243, 243, 117, 90,  119, 42,  270,
    /*  1640 */ 243, 243, 243, 97,  98,  72,  73,  270, 270, 88,
    /*  1650 */ 142, 105, 205, 107, 108, 109, 110, 111, 150, 113,
    /*  1660 */ 270, 194, 40,  114, 115, 194, 243, 194, 205, 194,
    /*  1670 */ 124, 125, 205, 112, 128, 4,   205, 6,   205, 8,
    /*  1680 */ 205, 154, 194, 194, 40,  115, 205, 16,  118, 18,
    /*  1690 */ 110, 111, 115, 205, 205, 118, 205, 151, 152, 153,
    /*  1700 */ 115, 266, 115, 118, 115, 118, 205, 118, 147, 113,
    /*  1710 */ 243, 110, 111, 24,  243, 154, 243, 156, 243, 205,
    /*  1720 */ 159, 40,  161, 115, 194, 54,  118, 166, 57,  205,
    /*  1730 */ 108, 243, 243, 62,  63,  205, 115, 66,  194, 118,
    /*  1740 */ 69,  97,  98,  188, 189, 190, 191, 192, 193, 205,
    /*  1750 */ 106, 107, 108, 109, 110, 111, 194, 205, 194, 88,
    /*  1760 */ 194, 205, 205, 194, 209, 210, 211, 205, 205, 205,
    /*  1770 */ 205, 205, 194, 243, 205, 194, 87,  194, 134, 205,
    /*  1780 */ 194, 205, 194, 205, 194, 141, 205, 243, 205, 108,
    /*  1790 */ 235, 205, 237, 205, 115, 205, 205, 118, 115, 194,
    /*  1800 */ 115, 118, 205, 118, 205, 243, 273, 243, 253, 243,
    /*  1810 */ 205, 115, 243, 115, 118, 205, 118, 205, 147, 205,
    /*  1820 */ 205, 243, 205, 194, 243, 154, 243, 156, 194, 243,
    /*  1830 */ 159, 243, 161, 243, 205, 194, 194, 166, 291, 205,
    /*  1840 */ 287, 218, 194, 194, 206, 195, 205, 205, 243, 295,
    /*  1850 */ 279, 194, 194, 205, 205, 300, 194, 264, 279, 304,
    /*  1860 */ 305, 306, 205, 205, 194, 258, 262, 205, 194, 194,
    /*  1870 */ 194, 194, 243, 279, 279, 205, 276, 243, 194, 205,
    /*  1880 */ 205, 205, 205, 258, 243, 243, 262, 269, 194, 205,
    /*  1890 */ 264, 243, 243, 194, 264, 264, 269, 295, 244, 205,
    /*  1900 */ 243, 243, 236, 194, 205, 243, 219, 221, 228, 244,
    /*  1910 */ 194, 194, 194, 243, 205, 194, 284, 243, 243, 243,
    /*  1920 */ 243, 205, 205, 205, 284, 262, 205, 243, 194, 194,
    /*  1930 */ 194, 284, 194, 146, 195, 284, 244, 243, 303, 205,
    /*  1940 */ 205, 205, 243, 205, 217, 13,  301, 259, 119, 259,
    /*  1950 */ 208, 208, 243, 301, 208, 63,  160, 275, 41,  243,
    /*  1960 */ 243, 243, 91,  149, 243, 113, 275, 274, 163, 274,
    /*  1970 */ 265, 274, 260, 272, 272, 272, 276, 243, 243, 243,
    /*  1980 */ 276, 243, 272, 263, 114, 259, 22,  255, 260, 259,
    /*  1990 */ 208, 91,  239, 148, 269, 239, 208, 113, 269, 255,
    /*  2000 */ 269, 208, 265, 255, 263, 265, 245, 239, 129, 43,
    /*  2010 */ 208, 239, 293, 239, 269, 292, 208, 106, 245, 229,
    /*  2020 */ 229, 229, 46,  224, 245, 113, 229, 139, 234, 234,
    /*  2030 */ 219, 224, 229, 229, 118, 229, 208, 260, 260, 241,
    /*  2040 */ 259, 281, 162, 116, 280, 269, 313, 80,  71,  113,
    /*  2050 */ 212, 271, 208, 104, 271, 129, 216, 117, 245, 250,
    /*  2060 */ 313, 168, 250, 311, 249, 311, 308, 231, 204, 204,
    /*  2070 */ 204, 204, 323, 231, 220, 220, 218, 218, 231, 204,
    /*  2080 */ 49,  113, 50,  112, 115, 115, 157, 132, 158, 124,
    /*  2090 */ 157, 147, 146, 143, 132, 117, 165, 118, 113, 127,
    /*  2100 */ 106, 112, 155, 143, 143, 42,  12,  127, 34,  146,
    /*  2110 */ 34,  34,  34,  107, 9,   119, 143, 112, 8,   117,
    /*  2120 */ 52,  118, 52,  17,  60,  106, 119, 24,  124, 138,
    /*  2130 */ 144, 113, 51,  113, 113, 51,  113, 115, 40,  85,
    /*  2140 */ 2,   117, 113, 51,  12,  118, 107, 164, 115, 115,
    /*  2150 */ 115, 9,   9,   113, 118, 113, 115, 119, 9,   114,
    /*  2160 */ 117, 115, 118, 113, 116, 148, 113, 115, 9,   9,
    /*  2170 */ 9,   60,  77,  23,  75,  60,  118, 9,   82,  118,
    /*  2180 */ 115, 115, 115, 127, 87,  113, 113, 118, 18,  118,
    /*  2190 */ 9,   9,   324, 324, 324, 115, 113, 118, 113, 113,
    /*  2200 */ 324, 119, 127, 115, 115, 115, 113, 324, 121, 324,
    /*  2210 */ 119, 324, 324, 324, 324, 324, 115, 115, 324, 119,
    /*  2220 */ 113, 112, 324, 324, 112, 324, 324, 324, 324, 324,
    /*  2230 */ 324, 324, 324, 324, 324, 324, 324, 324, 324, 324,
    /*  2240 */ 324, 324, 324, 324, 324, 324, 324, 324, 324, 324,
    /*  2250 */ 324, 324, 324, 324, 324, 324, 324, 324, 324, 324,
    /*  2260 */ 324, 324, 324, 324, 324, 324, 324, 324, 324, 324,
    /*  2270 */ 324, 324, 324, 324, 324, 324, 324, 324, 324, 324,
    /*  2280 */ 324, 324, 324, 324, 324, 324, 324, 324, 324, 324,
    /*  2290 */ 324, 324, 324, 324, 324, 324, 324, 324, 324, 324,
    /*  2300 */ 324, 324, 324, 324, 324, 324, 324, 324, 324, 324,
    /*  2310 */ 324, 324, 324, 324, 324, 324, 324, 324, 324, 324,
    /*  2320 */ 324, 324, 324, 324, 324, 324, 324, 324, 324, 324,
    /*  2330 */ 324, 324, 324, 324, 324, 324, 324, 324, 324, 324,
    /*  2340 */ 324, 324, 324, 324, 324, 324, 324, 324, 324, 324,
    /*  2350 */ 324, 324, 324, 324, 324, 324, 324, 324, 324, 324,
    /*  2360 */ 324, 324, 324, 324, 324, 324, 324, 324, 324, 324,
    /*  2370 */ 324, 324, 324, 324, 324, 324, 324, 324, 324, 324,
    /*  2380 */ 324, 324, 324, 324, 324, 324, 324, 324, 324, 324,
    /*  2390 */ 324, 324, 324, 324, 324, 324, 324, 324, 324, 324,
    /*  2400 */ 324, 324, 324, 324, 324, 324, 324, 324, 324, 324,
    /*  2410 */ 324, 324, 324,
};
#define YY_SHIFT_COUNT (594)
#define YY_SHIFT_MIN (0)
#define YY_SHIFT_MAX (2182)
static const unsigned short int yy_shift_ofst[] = {
    /*     0 */ 1561,
    1314,
    1671,
    991,
    991,
    738,
    945,
    1050,
    1125,
    1200,
    /*    10 */ 1546,
    1546,
    1546,
    960,
    738,
    738,
    738,
    738,
    738,
    0,
    /*    20 */ 0,
    142,
    840,
    1546,
    1546,
    1546,
    1546,
    1546,
    1546,
    1546,
    /*    30 */ 1546,
    1546,
    1546,
    1546,
    1546,
    1546,
    1546,
    1546,
    1644,
    1644,
    /*    40 */ 249,
    1105,
    1105,
    445,
    469,
    587,
    587,
    194,
    194,
    194,
    /*    50 */ 194,
    23,
    120,
    225,
    250,
    273,
    373,
    396,
    419,
    513,
    /*    60 */ 536,
    559,
    653,
    676,
    699,
    793,
    815,
    840,
    840,
    840,
    /*    70 */ 840,
    840,
    840,
    840,
    840,
    840,
    840,
    840,
    840,
    840,
    /*    80 */ 840,
    840,
    840,
    840,
    840,
    924,
    840,
    940,
    955,
    955,
    /*    90 */ 1326,
    1403,
    1436,
    1546,
    1546,
    1546,
    1546,
    1546,
    1546,
    1546,
    /*   100 */ 1546,
    1546,
    1546,
    1546,
    1546,
    1546,
    1546,
    1546,
    1546,
    1546,
    /*   110 */ 1546,
    1546,
    1546,
    1546,
    1546,
    1546,
    1546,
    1546,
    1546,
    1546,
    /*   120 */ 1546,
    1546,
    1546,
    1546,
    1546,
    1546,
    1546,
    1546,
    1546,
    1546,
    /*   130 */ 1546,
    1546,
    1546,
    1546,
    1546,
    1546,
    1546,
    1546,
    1546,
    1546,
    /*   140 */ 1546,
    1546,
    1546,
    1546,
    35,
    969,
    969,
    969,
    969,
    969,
    /*   150 */ 969,
    969,
    105,
    128,
    85,
    224,
    587,
    325,
    1145,
    587,
    /*   160 */ 587,
    587,
    1016,
    1016,
    844,
    401,
    218,
    310,
    310,
    310,
    /*   170 */ 152,
    42,
    42,
    2225,
    2225,
    1158,
    1158,
    1158,
    820,
    351,
    /*   180 */ 351,
    351,
    351,
    19,
    19,
    224,
    302,
    741,
    1078,
    587,
    /*   190 */ 587,
    587,
    587,
    587,
    587,
    587,
    587,
    587,
    587,
    587,
    /*   200 */ 587,
    587,
    587,
    587,
    587,
    587,
    587,
    1236,
    587,
    384,
    /*   210 */ 384,
    587,
    1468,
    1056,
    1056,
    1196,
    1196,
    1131,
    1330,
    1131,
    /*   220 */ 2225,
    2225,
    2225,
    2225,
    2225,
    2225,
    2225,
    1043,
    1254,
    1254,
    /*   230 */ 492,
    975,
    631,
    375,
    1291,
    1247,
    652,
    678,
    587,
    587,
    /*   240 */ 587,
    587,
    587,
    587,
    587,
    587,
    587,
    587,
    587,
    587,
    /*   250 */ 587,
    587,
    175,
    587,
    587,
    587,
    587,
    587,
    587,
    587,
    /*   260 */ 587,
    587,
    587,
    587,
    587,
    1206,
    1206,
    1206,
    587,
    587,
    /*   270 */ 587,
    587,
    1276,
    587,
    587,
    1261,
    587,
    1169,
    587,
    587,
    /*   280 */ 1441,
    587,
    587,
    1411,
    298,
    1234,
    978,
    585,
    585,
    585,
    /*   290 */ 1067,
    585,
    545,
    545,
    683,
    545,
    1416,
    997,
    1199,
    1268,
    /*   300 */ 1199,
    1275,
    1414,
    997,
    997,
    1414,
    997,
    1268,
    1275,
    73,
    /*   310 */ 1252,
    591,
    1454,
    790,
    1433,
    1433,
    1433,
    1433,
    1472,
    1004,
    /*   320 */ 1004,
    1454,
    1454,
    659,
    1479,
    1787,
    1932,
    1932,
    1829,
    1829,
    /*   330 */ 1829,
    1892,
    1892,
    1796,
    1796,
    1796,
    1917,
    1917,
    1871,
    1871,
    /*   340 */ 1871,
    1871,
    1814,
    1852,
    1805,
    1870,
    1964,
    1805,
    1870,
    1829,
    /*   350 */ 1900,
    1845,
    1829,
    1900,
    1845,
    1814,
    1814,
    1845,
    1852,
    1964,
    /*   360 */ 1845,
    1964,
    1884,
    1829,
    1900,
    1879,
    1966,
    1829,
    1900,
    1829,
    /*   370 */ 1900,
    1884,
    1911,
    1911,
    1911,
    1976,
    1912,
    1912,
    1884,
    1911,
    /*   380 */ 1888,
    1911,
    1976,
    1911,
    1911,
    1916,
    1829,
    1805,
    1870,
    1805,
    /*   390 */ 1880,
    1927,
    1845,
    1967,
    1967,
    1977,
    1977,
    1936,
    1829,
    1949,
    /*   400 */ 1949,
    1926,
    1940,
    1884,
    1893,
    2225,
    2225,
    2225,
    2225,
    2225,
    /*   410 */ 2225,
    2225,
    2225,
    2225,
    2225,
    2225,
    2225,
    2225,
    2225,
    2225,
    /*   420 */ 499,
    1204,
    649,
    1425,
    1191,
    1323,
    1279,
    1496,
    982,
    1527,
    /*   430 */ 1518,
    1504,
    1400,
    1508,
    1511,
    1596,
    1570,
    1577,
    1585,
    1587,
    /*   440 */ 1589,
    1401,
    1039,
    1480,
    1469,
    1608,
    1573,
    1622,
    1621,
    1689,
    /*   450 */ 1679,
    1683,
    1685,
    1681,
    1580,
    1601,
    1696,
    1698,
    1549,
    1439,
    /*   460 */ 2031,
    2032,
    1968,
    1971,
    1969,
    1970,
    1929,
    1930,
    1933,
    1955,
    /*   470 */ 1965,
    1944,
    1946,
    1950,
    1978,
    1979,
    1979,
    1972,
    1931,
    1962,
    /*   480 */ 1985,
    1994,
    1947,
    1989,
    1980,
    1960,
    1979,
    1961,
    2063,
    2094,
    /*   490 */ 1979,
    1963,
    2074,
    2076,
    2077,
    2078,
    1973,
    2006,
    2105,
    1996,
    /*   500 */ 2005,
    2110,
    2002,
    2068,
    2003,
    2070,
    2064,
    2106,
    2007,
    2019,
    /*   510 */ 2004,
    2103,
    1986,
    1991,
    2018,
    2081,
    2020,
    2021,
    2022,
    2023,
    /*   520 */ 2084,
    2098,
    2024,
    2054,
    2138,
    2029,
    2092,
    2132,
    2027,
    2033,
    /*   530 */ 2034,
    2035,
    2039,
    2142,
    2040,
    1983,
    2036,
    2143,
    2041,
    2042,
    /*   540 */ 2043,
    2044,
    2038,
    2046,
    2149,
    2045,
    2050,
    2048,
    2017,
    2053,
    /*   550 */ 2052,
    2159,
    2160,
    2161,
    2095,
    2111,
    2099,
    2150,
    2115,
    2096,
    /*   560 */ 2058,
    2168,
    2065,
    2036,
    2066,
    2067,
    2061,
    2097,
    2072,
    2069,
    /*   570 */ 2073,
    2071,
    2056,
    2075,
    2079,
    2080,
    2083,
    2082,
    2170,
    2085,
    /*   580 */ 2088,
    2086,
    2087,
    2089,
    2093,
    2090,
    2091,
    2100,
    2101,
    2102,
    /*   590 */ 2107,
    2181,
    2182,
    2109,
    2112,
};
#define YY_REDUCE_COUNT (419)
#define YY_REDUCE_MIN (-312)
#define YY_REDUCE_MAX (1875)
static const short yy_reduce_ofst[] = {
    /*     0 */ 1555,
    1035,
    411,
    1179,
    1202,
    -199,
    -192,
    71,
    888,
    804,
    /*    10 */ 614,
    936,
    1063,
    -145,
    -142,
    57,
    60,
    203,
    226,
    -205,
    /*    20 */ -200,
    598,
    -125,
    661,
    1024,
    1223,
    1224,
    892,
    1277,
    1293,
    /*    30 */ 1310,
    1325,
    1369,
    1377,
    962,
    1378,
    1390,
    1354,
    -189,
    -115,
    /*    40 */ 437,
    608,
    872,
    88,
    393,
    515,
    584,
    692,
    696,
    692,
    /*    50 */ 696,
    -122,
    -122,
    -122,
    -122,
    -122,
    -122,
    -122,
    -122,
    -122,
    /*    60 */ -122,
    -122,
    -122,
    -122,
    -122,
    -122,
    -122,
    -122,
    -122,
    -122,
    /*    70 */ -122,
    -122,
    -122,
    -122,
    -122,
    -122,
    -122,
    -122,
    -122,
    -122,
    /*    80 */ -122,
    -122,
    -122,
    -122,
    -122,
    -122,
    -122,
    -122,
    -122,
    -122,
    /*    90 */ 1167,
    1203,
    1210,
    1294,
    1391,
    1397,
    1398,
    1399,
    1423,
    1467,
    /*   100 */ 1471,
    1473,
    1475,
    1488,
    1489,
    1530,
    1544,
    1562,
    1564,
    1566,
    /*   110 */ 1569,
    1578,
    1581,
    1583,
    1586,
    1588,
    1590,
    1605,
    1629,
    1634,
    /*   120 */ 1641,
    1642,
    1648,
    1649,
    1657,
    1658,
    1662,
    1670,
    1674,
    1675,
    /*   130 */ 1676,
    1677,
    1684,
    1694,
    1699,
    1709,
    1716,
    1717,
    1718,
    1721,
    /*   140 */ 1734,
    1735,
    1736,
    1738,
    -122,
    -122,
    -122,
    -122,
    -122,
    -122,
    /*   150 */ -122,
    -122,
    -122,
    -122,
    -122,
    -184,
    717,
    286,
    -31,
    1042,
    /*   160 */ -201,
    86,
    446,
    583,
    -112,
    -122,
    329,
    560,
    941,
    994,
    /*   170 */ 345,
    -122,
    -122,
    -122,
    -122,
    -62,
    -62,
    -62,
    276,
    -65,
    /*   180 */ 29,
    301,
    520,
    684,
    764,
    805,
    -312,
    177,
    177,
    -164,
    /*   190 */ -202,
    468,
    803,
    923,
    130,
    1081,
    1152,
    636,
    398,
    693,
    /*   200 */ 993,
    1005,
    992,
    1061,
    1116,
    46,
    1095,
    359,
    467,
    766,
    /*   210 */ 1120,
    176,
    1154,
    1177,
    1184,
    1048,
    1225,
    -89,
    -233,
    357,
    /*   220 */ 880,
    1124,
    1321,
    1373,
    1405,
    -203,
    1429,
    -242,
    87,
    94,
    /*   230 */ 174,
    185,
    258,
    305,
    514,
    629,
    656,
    716,
    740,
    794,
    /*   240 */ 904,
    1002,
    1107,
    1140,
    1155,
    1198,
    1214,
    1300,
    1313,
    1316,
    /*   250 */ 1361,
    1420,
    326,
    1447,
    1463,
    1481,
    1491,
    1501,
    1514,
    1524,
    /*   260 */ 1552,
    1556,
    1557,
    1563,
    1565,
    823,
    865,
    1026,
    1574,
    1576,
    /*   270 */ 1591,
    1597,
    596,
    1599,
    1610,
    1299,
    1612,
    1292,
    1614,
    1615,
    /*   280 */ 1435,
    1617,
    629,
    1533,
    1547,
    1553,
    1623,
    1571,
    1579,
    1594,
    /*   290 */ 596,
    1595,
    1638,
    1638,
    1650,
    1638,
    1600,
    1593,
    1607,
    1604,
    /*   300 */ 1625,
    1554,
    1618,
    1626,
    1630,
    1627,
    1631,
    1624,
    1602,
    1687,
    /*   310 */ 1680,
    1686,
    1654,
    1666,
    1632,
    1640,
    1647,
    1651,
    1663,
    1688,
    /*   320 */ 1690,
    1665,
    1692,
    1727,
    1739,
    1635,
    1645,
    1652,
    1742,
    1743,
    /*   330 */ 1746,
    1682,
    1691,
    1693,
    1695,
    1697,
    1700,
    1704,
    1701,
    1702,
    /*   340 */ 1703,
    1710,
    1705,
    1720,
    1712,
    1726,
    1732,
    1728,
    1730,
    1782,
    /*   350 */ 1753,
    1725,
    1788,
    1756,
    1729,
    1737,
    1740,
    1731,
    1741,
    1744,
    /*   360 */ 1745,
    1748,
    1761,
    1793,
    1768,
    1719,
    1723,
    1802,
    1772,
    1808,
    /*   370 */ 1774,
    1773,
    1790,
    1791,
    1792,
    1799,
    1794,
    1795,
    1779,
    1797,
    /*   380 */ 1811,
    1803,
    1807,
    1804,
    1806,
    1798,
    1828,
    1777,
    1781,
    1778,
    /*   390 */ 1760,
    1764,
    1776,
    1733,
    1747,
    1780,
    1783,
    1838,
    1844,
    1809,
    /*   400 */ 1812,
    1840,
    1815,
    1813,
    1749,
    1752,
    1754,
    1758,
    1836,
    1864,
    /*   410 */ 1865,
    1866,
    1867,
    1842,
    1854,
    1855,
    1858,
    1859,
    1847,
    1875,
};
static const YYACTIONTYPE yy_default[] = {
    /*     0 */ 1411,
    1411,
    1411,
    1464,
    1271,
    1556,
    1271,
    1271,
    1271,
    1271,
    /*    10 */ 1464,
    1464,
    1464,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1485,
    /*    20 */ 1485,
    1619,
    1534,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    /*    30 */ 1271,
    1271,
    1271,
    1271,
    1330,
    1271,
    1271,
    1271,
    1271,
    1271,
    /*    40 */ 1271,
    1664,
    1664,
    1271,
    1271,
    1271,
    1271,
    1413,
    1412,
    1271,
    /*    50 */ 1271,
    1552,
    1271,
    1271,
    1432,
    1271,
    1271,
    1271,
    1271,
    1271,
    /*    60 */ 1271,
    1465,
    1466,
    1271,
    1271,
    1271,
    1271,
    1623,
    1616,
    1620,
    /*    70 */ 1438,
    1437,
    1436,
    1435,
    1584,
    1566,
    1544,
    1548,
    1554,
    1553,
    /*    80 */ 1465,
    1326,
    1327,
    1325,
    1329,
    1271,
    1466,
    1456,
    1462,
    1455,
    /*    90 */ 1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    /*   100 */ 1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    /*   110 */ 1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    /*   120 */ 1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    /*   130 */ 1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    /*   140 */ 1271,
    1271,
    1271,
    1271,
    1322,
    1316,
    1315,
    1314,
    1454,
    1323,
    /*   150 */ 1319,
    1313,
    1453,
    1457,
    1451,
    1335,
    1271,
    1636,
    1391,
    1271,
    /*   160 */ 1271,
    1271,
    1271,
    1271,
    1468,
    1452,
    1534,
    1469,
    1283,
    1281,
    /*   170 */ 1271,
    1459,
    1458,
    1461,
    1460,
    1505,
    1341,
    1340,
    1624,
    1271,
    /*   180 */ 1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1664,
    1271,
    1271,
    1271,
    /*   190 */ 1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    /*   200 */ 1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1568,
    1271,
    1664,
    /*   210 */ 1664,
    1271,
    1534,
    1664,
    1664,
    1429,
    1429,
    1286,
    1549,
    1286,
    /*   220 */ 1647,
    1533,
    1533,
    1533,
    1533,
    1556,
    1533,
    1271,
    1271,
    1271,
    /*   230 */ 1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1613,
    1611,
    /*   240 */ 1271,
    1271,
    1271,
    1271,
    1518,
    1271,
    1271,
    1271,
    1271,
    1271,
    /*   250 */ 1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    /*   260 */ 1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    /*   270 */ 1271,
    1271,
    1271,
    1271,
    1271,
    1537,
    1271,
    1271,
    1271,
    1271,
    /*   280 */ 1271,
    1271,
    1271,
    1513,
    1271,
    1577,
    1395,
    1537,
    1537,
    1537,
    /*   290 */ 1542,
    1537,
    1397,
    1396,
    1540,
    1526,
    1507,
    1441,
    1431,
    1541,
    /*   300 */ 1431,
    1589,
    1543,
    1441,
    1441,
    1543,
    1441,
    1541,
    1589,
    1362,
    /*   310 */ 1385,
    1355,
    1485,
    1271,
    1568,
    1568,
    1568,
    1568,
    1541,
    1549,
    /*   320 */ 1549,
    1485,
    1485,
    1328,
    1540,
    1624,
    1618,
    1618,
    1307,
    1307,
    /*   330 */ 1307,
    1521,
    1521,
    1517,
    1517,
    1517,
    1507,
    1507,
    1497,
    1497,
    /*   340 */ 1497,
    1497,
    1448,
    1439,
    1551,
    1549,
    1420,
    1551,
    1549,
    1307,
    /*   350 */ 1631,
    1543,
    1307,
    1631,
    1543,
    1448,
    1448,
    1543,
    1439,
    1420,
    /*   360 */ 1543,
    1420,
    1405,
    1307,
    1631,
    1583,
    1581,
    1307,
    1631,
    1307,
    /*   370 */ 1631,
    1405,
    1393,
    1393,
    1393,
    1377,
    1271,
    1271,
    1405,
    1393,
    /*   380 */ 1362,
    1393,
    1377,
    1393,
    1393,
    1380,
    1307,
    1551,
    1549,
    1551,
    /*   390 */ 1547,
    1545,
    1543,
    1674,
    1674,
    1488,
    1488,
    1309,
    1307,
    1409,
    /*   400 */ 1409,
    1271,
    1271,
    1405,
    1682,
    1652,
    1652,
    1647,
    1343,
    1534,
    /*   410 */ 1534,
    1534,
    1534,
    1343,
    1364,
    1364,
    1395,
    1395,
    1343,
    1534,
    /*   420 */ 1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1287,
    1271,
    1596,
    1506,
    /*   430 */ 1425,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    /*   440 */ 1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1402,
    /*   450 */ 1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1293,
    /*   460 */ 1271,
    1626,
    1642,
    1271,
    1271,
    1271,
    1512,
    1271,
    1271,
    1271,
    /*   470 */ 1271,
    1271,
    1271,
    1271,
    1426,
    1433,
    1434,
    1271,
    1271,
    1271,
    /*   480 */ 1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1447,
    1271,
    1271,
    1271,
    /*   490 */ 1442,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1587,
    /*   500 */ 1271,
    1271,
    1271,
    1271,
    1580,
    1579,
    1271,
    1271,
    1494,
    1271,
    /*   510 */ 1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    /*   520 */ 1271,
    1360,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1333,
    1271,
    /*   530 */ 1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1546,
    1271,
    1271,
    1271,
    /*   540 */ 1271,
    1679,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    /*   550 */ 1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    1271,
    /*   560 */ 1550,
    1271,
    1271,
    1463,
    1271,
    1271,
    1271,
    1271,
    1271,
    1641,
    /*   570 */ 1271,
    1640,
    1271,
    1271,
    1271,
    1271,
    1271,
    1475,
    1271,
    1271,
    /*   580 */ 1271,
    1271,
    1297,
    1271,
    1271,
    1271,
    1294,
    1271,
    1271,
    1271,
    /*   590 */ 1271,
    1271,
    1271,
    1271,
    1271,
};
/********** End of lemon-generated parsing tables *****************************/

/* The next table maps tokens (terminal symbols) into fallback tokens.
** If a construct like the following:
**
**      %fallback ID X Y Z.
**
** appears in the grammar, then ID becomes a fallback token for X, Y,
** and Z.  Whenever one of the tokens X, Y, or Z is input to the parser
** but it does not parse, the type of the token is changed to ID and
** the parse is retried before an error is thrown.
**
** This feature can be used, for example, to cause some keywords in a language
** to revert to identifiers if they keyword does not apply in the context where
** it appears.
*/
#ifdef YYFALLBACK
static const YYCODETYPE yyFallback[] = {
    0,  /*          $ => nothing */
    40, /*      ABORT => ID */
    40, /*     ACTION => ID */
    40, /*      AFTER => ID */
    40, /*    ANALYZE => ID */
    40, /*        ASC => ID */
    40, /*     ATTACH => ID */
    40, /*     BEFORE => ID */
    40, /*      BEGIN => ID */
    40, /*         BY => ID */
    40, /*    CASCADE => ID */
    40, /*       CAST => ID */
    40, /*   CONFLICT => ID */
    40, /*   DATABASE => ID */
    40, /*   DEFERRED => ID */
    40, /*       DESC => ID */
    40, /*     DETACH => ID */
    40, /*       EACH => ID */
    40, /*        END => ID */
    40, /*  EXCLUSIVE => ID */
    40, /*    EXPLAIN => ID */
    40, /*       FAIL => ID */
    0,  /*         OR => nothing */
    0,  /*        AND => nothing */
    0,  /*        NOT => nothing */
    0,  /*         IS => nothing */
    0,  /*      ISNOT => nothing */
    40, /*      MATCH => ID */
    40, /*    LIKE_KW => ID */
    0,  /*    BETWEEN => nothing */
    0,  /*         IN => nothing */
    0,  /*     ISNULL => nothing */
    0,  /*    NOTNULL => nothing */
    0,  /*         NE => nothing */
    0,  /*         EQ => nothing */
    0,  /*         GT => nothing */
    0,  /*         LE => nothing */
    0,  /*         LT => nothing */
    0,  /*         GE => nothing */
    0,  /*     ESCAPE => nothing */
    0,  /*         ID => nothing */
    40, /*   COLUMNKW => ID */
    40, /*         DO => ID */
    40, /*        FOR => ID */
    40, /*     IGNORE => ID */
    40, /*  IMMEDIATE => ID */
    40, /*  INITIALLY => ID */
    40, /*    INSTEAD => ID */
    40, /*         NO => ID */
    40, /*       PLAN => ID */
    40, /*      QUERY => ID */
    40, /*        KEY => ID */
    40, /*         OF => ID */
    40, /*     OFFSET => ID */
    40, /*     PRAGMA => ID */
    40, /*      RAISE => ID */
    40, /*  RECURSIVE => ID */
    40, /*    RELEASE => ID */
    40, /*    REPLACE => ID */
    40, /*   RESTRICT => ID */
    40, /*        ROW => ID */
    40, /*       ROWS => ID */
    40, /*   ROLLBACK => ID */
    40, /*  SAVEPOINT => ID */
    40, /*       TEMP => ID */
    40, /*    TRIGGER => ID */
    40, /*     VACUUM => ID */
    40, /*       VIEW => ID */
    40, /*    VIRTUAL => ID */
    40, /*       WITH => ID */
    40, /*    WITHOUT => ID */
    40, /*      NULLS => ID */
    40, /*      FIRST => ID */
    40, /*       LAST => ID */
    40, /*    CURRENT => ID */
    40, /*  FOLLOWING => ID */
    40, /*  PARTITION => ID */
    40, /*  PRECEDING => ID */
    40, /*      RANGE => ID */
    40, /*  UNBOUNDED => ID */
    40, /*    EXCLUDE => ID */
    40, /*     GROUPS => ID */
    40, /*     OTHERS => ID */
    40, /*       TIES => ID */
    40, /*  GENERATED => ID */
    40, /*     ALWAYS => ID */
    40, /*     WITHIN => ID */
    40, /* MATERIALIZED => ID */
    40, /*    REINDEX => ID */
    40, /*     RENAME => ID */
    40, /*   CTIME_KW => ID */
    40, /*         IF => ID */
    0,  /*        ANY => nothing */
    0,  /*     BITAND => nothing */
    0,  /*      BITOR => nothing */
    0,  /*     LSHIFT => nothing */
    0,  /*     RSHIFT => nothing */
    0,  /*       PLUS => nothing */
    0,  /*      MINUS => nothing */
    0,  /*       STAR => nothing */
    0,  /*      SLASH => nothing */
    0,  /*        REM => nothing */
    0,  /*     CONCAT => nothing */
    0,  /*        PTR => nothing */
    0,  /*    COLLATE => nothing */
    0,  /*     BITNOT => nothing */
    0,  /*         ON => nothing */
    0,  /*    INDEXED => nothing */
    0,  /*     STRING => nothing */
    0,  /*    JOIN_KW => nothing */
    0,  /*    INTEGER => nothing */
    0,  /*      FLOAT => nothing */
    0,  /*       SEMI => nothing */
    0,  /*         LP => nothing */
    0,  /*      ORDER => nothing */
    0,  /*         RP => nothing */
    0,  /*      GROUP => nothing */
    0,  /*         AS => nothing */
    0,  /*      COMMA => nothing */
    0,  /*        DOT => nothing */
    0,  /*      UNION => nothing */
    0,  /*        ALL => nothing */
    0,  /*     EXCEPT => nothing */
    0,  /*  INTERSECT => nothing */
    0,  /*     EXISTS => nothing */
    0,  /*       NULL => nothing */
    0,  /*   DISTINCT => nothing */
    0,  /*       FROM => nothing */
    0,  /*       CASE => nothing */
    0,  /*       WHEN => nothing */
    0,  /*       THEN => nothing */
    0,  /*       ELSE => nothing */
    0,  /*      TABLE => nothing */
    0,  /* CONSTRAINT => nothing */
    0,  /*    DEFAULT => nothing */
    0,  /*    PRIMARY => nothing */
    0,  /*     UNIQUE => nothing */
    0,  /*      CHECK => nothing */
    0,  /* REFERENCES => nothing */
    0,  /*   AUTOINCR => nothing */
    0,  /*     INSERT => nothing */
    0,  /*     DELETE => nothing */
    0,  /*     UPDATE => nothing */
    0,  /*        SET => nothing */
    0,  /* DEFERRABLE => nothing */
    0,  /*    FOREIGN => nothing */
    0,  /*       INTO => nothing */
    0,  /*     VALUES => nothing */
    0,  /*      WHERE => nothing */
    0,  /*  RETURNING => nothing */
    0,  /*    NOTHING => nothing */
    0,  /*       BLOB => nothing */
    0,  /*    QNUMBER => nothing */
    0,  /*   VARIABLE => nothing */
    0,  /*       DROP => nothing */
    0,  /*      INDEX => nothing */
    0,  /*      ALTER => nothing */
    0,  /*         TO => nothing */
    0,  /*        ADD => nothing */
    0,  /*     COMMIT => nothing */
    0,  /* TRANSACTION => nothing */
    0,  /*     SELECT => nothing */
    0,  /*     HAVING => nothing */
    0,  /*      LIMIT => nothing */
    0,  /*       JOIN => nothing */
    0,  /*      USING => nothing */
    0,  /*     CREATE => nothing */
    0,  /*     WINDOW => nothing */
    0,  /*       OVER => nothing */
    0,  /*     FILTER => nothing */
    0,  /*     COLUMN => nothing */
    0,  /* AGG_FUNCTION => nothing */
    0,  /* AGG_COLUMN => nothing */
    0,  /*  TRUEFALSE => nothing */
    0,  /*   FUNCTION => nothing */
    0,  /*      UPLUS => nothing */
    0,  /*     UMINUS => nothing */
    0,  /*      TRUTH => nothing */
    0,  /*   REGISTER => nothing */
    0,  /*     VECTOR => nothing */
    0,  /* SELECT_COLUMN => nothing */
    0,  /* IF_NULL_ROW => nothing */
    0,  /*   ASTERISK => nothing */
    0,  /*       SPAN => nothing */
    0,  /*      ERROR => nothing */
    0,  /*      SPACE => nothing */
    0,  /*    COMMENT => nothing */
    0,  /*    ILLEGAL => nothing */
};
#endif /* YYFALLBACK */

/* The following structure represents a single element of the
** parser's stack.  Information stored includes:
**
**   +  The state number for the parser at this level of the stack.
**
**   +  The value of the token stored at this level of the stack.
**      (In other words, the "major" token.)
**
**   +  The semantic value stored at this level of the stack.  This is
**      the information used by the action routines in the grammar.
**      It is sometimes called the "minor" token.
**
** After the "shift" half of a SHIFTREDUCE action, the stateno field
** actually contains the reduce action for the second half of the
** SHIFTREDUCE.
*/
struct yyStackEntry {
  YYACTIONTYPE stateno; /* The state-number, or reduce action in SHIFTREDUCE */
  YYCODETYPE major;     /* The major token value.  This is the code
                        ** number for the token at this stack level */
  YYMINORTYPE minor;    /* The user-supplied minor token value.  This
                        ** is the value of the token  */
};
typedef struct yyStackEntry yyStackEntry;

/* The state of the parser is completely contained in an instance of
** the following structure */
struct yyParser {
  yyStackEntry* yytos; /* Pointer to top element of the stack */
#ifdef YYTRACKMAXSTACKDEPTH
  int yyhwm; /* High-water mark of the stack */
#endif
#ifndef YYNOERRORRECOVERY
  int yyerrcnt; /* Shifts left before out of the error */
#endif
  SynqSqliteParseARG_SDECL           /* A place to hold %extra_argument */
      SynqSqliteParseCTX_SDECL       /* A place to hold %extra_context */
      yyStackEntry* yystackEnd;      /* Last entry in the stack */
  yyStackEntry* yystack;             /* The parser stack */
  yyStackEntry yystk0[YYSTACKDEPTH]; /* Initial stack space */
};
typedef struct yyParser yyParser;

#include <assert.h>
#ifndef NDEBUG
#include <stdio.h>
static FILE* yyTraceFILE = 0;
static char* yyTracePrompt = 0;
#endif /* NDEBUG */

#ifndef NDEBUG
/*
** Turn parser tracing on by giving a stream to which to write the trace
** and a prompt to preface each trace message.  Tracing is turned off
** by making either argument NULL
**
** Inputs:
** <ul>
** <li> A FILE* to which trace output should be written.
**      If NULL, then tracing is turned off.
** <li> A prefix string written at the beginning of every
**      line of trace output.  If NULL, then tracing is
**      turned off.
** </ul>
**
** Outputs:
** None.
*/
void SynqSqliteParseTrace(FILE* TraceFILE, char* zTracePrompt) {
  yyTraceFILE = TraceFILE;
  yyTracePrompt = zTracePrompt;
  if (yyTraceFILE == 0)
    yyTracePrompt = 0;
  else if (yyTracePrompt == 0)
    yyTraceFILE = 0;
}
#endif /* NDEBUG */

#if defined(YYCOVERAGE) || !defined(NDEBUG)
/* For tracing shifts, the names of all terminals and nonterminals
** are required.  The following table supplies these names */
static const char* const yyTokenName[] = {
    /*    0 */ "$",
    /*    1 */ "ABORT",
    /*    2 */ "ACTION",
    /*    3 */ "AFTER",
    /*    4 */ "ANALYZE",
    /*    5 */ "ASC",
    /*    6 */ "ATTACH",
    /*    7 */ "BEFORE",
    /*    8 */ "BEGIN",
    /*    9 */ "BY",
    /*   10 */ "CASCADE",
    /*   11 */ "CAST",
    /*   12 */ "CONFLICT",
    /*   13 */ "DATABASE",
    /*   14 */ "DEFERRED",
    /*   15 */ "DESC",
    /*   16 */ "DETACH",
    /*   17 */ "EACH",
    /*   18 */ "END",
    /*   19 */ "EXCLUSIVE",
    /*   20 */ "EXPLAIN",
    /*   21 */ "FAIL",
    /*   22 */ "OR",
    /*   23 */ "AND",
    /*   24 */ "NOT",
    /*   25 */ "IS",
    /*   26 */ "ISNOT",
    /*   27 */ "MATCH",
    /*   28 */ "LIKE_KW",
    /*   29 */ "BETWEEN",
    /*   30 */ "IN",
    /*   31 */ "ISNULL",
    /*   32 */ "NOTNULL",
    /*   33 */ "NE",
    /*   34 */ "EQ",
    /*   35 */ "GT",
    /*   36 */ "LE",
    /*   37 */ "LT",
    /*   38 */ "GE",
    /*   39 */ "ESCAPE",
    /*   40 */ "ID",
    /*   41 */ "COLUMNKW",
    /*   42 */ "DO",
    /*   43 */ "FOR",
    /*   44 */ "IGNORE",
    /*   45 */ "IMMEDIATE",
    /*   46 */ "INITIALLY",
    /*   47 */ "INSTEAD",
    /*   48 */ "NO",
    /*   49 */ "PLAN",
    /*   50 */ "QUERY",
    /*   51 */ "KEY",
    /*   52 */ "OF",
    /*   53 */ "OFFSET",
    /*   54 */ "PRAGMA",
    /*   55 */ "RAISE",
    /*   56 */ "RECURSIVE",
    /*   57 */ "RELEASE",
    /*   58 */ "REPLACE",
    /*   59 */ "RESTRICT",
    /*   60 */ "ROW",
    /*   61 */ "ROWS",
    /*   62 */ "ROLLBACK",
    /*   63 */ "SAVEPOINT",
    /*   64 */ "TEMP",
    /*   65 */ "TRIGGER",
    /*   66 */ "VACUUM",
    /*   67 */ "VIEW",
    /*   68 */ "VIRTUAL",
    /*   69 */ "WITH",
    /*   70 */ "WITHOUT",
    /*   71 */ "NULLS",
    /*   72 */ "FIRST",
    /*   73 */ "LAST",
    /*   74 */ "CURRENT",
    /*   75 */ "FOLLOWING",
    /*   76 */ "PARTITION",
    /*   77 */ "PRECEDING",
    /*   78 */ "RANGE",
    /*   79 */ "UNBOUNDED",
    /*   80 */ "EXCLUDE",
    /*   81 */ "GROUPS",
    /*   82 */ "OTHERS",
    /*   83 */ "TIES",
    /*   84 */ "GENERATED",
    /*   85 */ "ALWAYS",
    /*   86 */ "WITHIN",
    /*   87 */ "MATERIALIZED",
    /*   88 */ "REINDEX",
    /*   89 */ "RENAME",
    /*   90 */ "CTIME_KW",
    /*   91 */ "IF",
    /*   92 */ "ANY",
    /*   93 */ "BITAND",
    /*   94 */ "BITOR",
    /*   95 */ "LSHIFT",
    /*   96 */ "RSHIFT",
    /*   97 */ "PLUS",
    /*   98 */ "MINUS",
    /*   99 */ "STAR",
    /*  100 */ "SLASH",
    /*  101 */ "REM",
    /*  102 */ "CONCAT",
    /*  103 */ "PTR",
    /*  104 */ "COLLATE",
    /*  105 */ "BITNOT",
    /*  106 */ "ON",
    /*  107 */ "INDEXED",
    /*  108 */ "STRING",
    /*  109 */ "JOIN_KW",
    /*  110 */ "INTEGER",
    /*  111 */ "FLOAT",
    /*  112 */ "SEMI",
    /*  113 */ "LP",
    /*  114 */ "ORDER",
    /*  115 */ "RP",
    /*  116 */ "GROUP",
    /*  117 */ "AS",
    /*  118 */ "COMMA",
    /*  119 */ "DOT",
    /*  120 */ "UNION",
    /*  121 */ "ALL",
    /*  122 */ "EXCEPT",
    /*  123 */ "INTERSECT",
    /*  124 */ "EXISTS",
    /*  125 */ "NULL",
    /*  126 */ "DISTINCT",
    /*  127 */ "FROM",
    /*  128 */ "CASE",
    /*  129 */ "WHEN",
    /*  130 */ "THEN",
    /*  131 */ "ELSE",
    /*  132 */ "TABLE",
    /*  133 */ "CONSTRAINT",
    /*  134 */ "DEFAULT",
    /*  135 */ "PRIMARY",
    /*  136 */ "UNIQUE",
    /*  137 */ "CHECK",
    /*  138 */ "REFERENCES",
    /*  139 */ "AUTOINCR",
    /*  140 */ "INSERT",
    /*  141 */ "DELETE",
    /*  142 */ "UPDATE",
    /*  143 */ "SET",
    /*  144 */ "DEFERRABLE",
    /*  145 */ "FOREIGN",
    /*  146 */ "INTO",
    /*  147 */ "VALUES",
    /*  148 */ "WHERE",
    /*  149 */ "RETURNING",
    /*  150 */ "NOTHING",
    /*  151 */ "BLOB",
    /*  152 */ "QNUMBER",
    /*  153 */ "VARIABLE",
    /*  154 */ "DROP",
    /*  155 */ "INDEX",
    /*  156 */ "ALTER",
    /*  157 */ "TO",
    /*  158 */ "ADD",
    /*  159 */ "COMMIT",
    /*  160 */ "TRANSACTION",
    /*  161 */ "SELECT",
    /*  162 */ "HAVING",
    /*  163 */ "LIMIT",
    /*  164 */ "JOIN",
    /*  165 */ "USING",
    /*  166 */ "CREATE",
    /*  167 */ "WINDOW",
    /*  168 */ "OVER",
    /*  169 */ "FILTER",
    /*  170 */ "COLUMN",
    /*  171 */ "AGG_FUNCTION",
    /*  172 */ "AGG_COLUMN",
    /*  173 */ "TRUEFALSE",
    /*  174 */ "FUNCTION",
    /*  175 */ "UPLUS",
    /*  176 */ "UMINUS",
    /*  177 */ "TRUTH",
    /*  178 */ "REGISTER",
    /*  179 */ "VECTOR",
    /*  180 */ "SELECT_COLUMN",
    /*  181 */ "IF_NULL_ROW",
    /*  182 */ "ASTERISK",
    /*  183 */ "SPAN",
    /*  184 */ "ERROR",
    /*  185 */ "SPACE",
    /*  186 */ "COMMENT",
    /*  187 */ "ILLEGAL",
    /*  188 */ "input",
    /*  189 */ "cmdlist",
    /*  190 */ "ecmd",
    /*  191 */ "cmdx",
    /*  192 */ "error",
    /*  193 */ "cmd",
    /*  194 */ "expr",
    /*  195 */ "distinct",
    /*  196 */ "exprlist",
    /*  197 */ "sortlist",
    /*  198 */ "filter_over",
    /*  199 */ "typetoken",
    /*  200 */ "typename",
    /*  201 */ "signed",
    /*  202 */ "selcollist",
    /*  203 */ "sclp",
    /*  204 */ "scanpt",
    /*  205 */ "nm",
    /*  206 */ "multiselect_op",
    /*  207 */ "in_op",
    /*  208 */ "dbnm",
    /*  209 */ "selectnowith",
    /*  210 */ "oneselect",
    /*  211 */ "select",
    /*  212 */ "paren_exprlist",
    /*  213 */ "likeop",
    /*  214 */ "between_op",
    /*  215 */ "case_operand",
    /*  216 */ "case_exprlist",
    /*  217 */ "case_else",
    /*  218 */ "scantok",
    /*  219 */ "autoinc",
    /*  220 */ "refargs",
    /*  221 */ "refarg",
    /*  222 */ "refact",
    /*  223 */ "defer_subclause",
    /*  224 */ "init_deferred_pred_opt",
    /*  225 */ "defer_subclause_opt",
    /*  226 */ "table_option_set",
    /*  227 */ "table_option",
    /*  228 */ "tconscomma",
    /*  229 */ "onconf",
    /*  230 */ "ccons",
    /*  231 */ "carglist",
    /*  232 */ "tcons",
    /*  233 */ "conslist",
    /*  234 */ "generated",
    /*  235 */ "create_table",
    /*  236 */ "create_table_args",
    /*  237 */ "createkw",
    /*  238 */ "temp",
    /*  239 */ "ifnotexists",
    /*  240 */ "columnlist",
    /*  241 */ "conslist_opt",
    /*  242 */ "columnname",
    /*  243 */ "term",
    /*  244 */ "sortorder",
    /*  245 */ "eidlist_opt",
    /*  246 */ "eidlist",
    /*  247 */ "resolvetype",
    /*  248 */ "withnm",
    /*  249 */ "wqas",
    /*  250 */ "collate",
    /*  251 */ "wqlist",
    /*  252 */ "wqitem",
    /*  253 */ "with",
    /*  254 */ "insert_cmd",
    /*  255 */ "orconf",
    /*  256 */ "indexed_opt",
    /*  257 */ "xfullname",
    /*  258 */ "where_opt_ret",
    /*  259 */ "orderby_opt",
    /*  260 */ "limit_opt",
    /*  261 */ "setlist",
    /*  262 */ "from",
    /*  263 */ "idlist_opt",
    /*  264 */ "upsert",
    /*  265 */ "returning",
    /*  266 */ "raisetype",
    /*  267 */ "indexed_by",
    /*  268 */ "idlist",
    /*  269 */ "where_opt",
    /*  270 */ "nexprlist",
    /*  271 */ "nulls",
    /*  272 */ "ifexists",
    /*  273 */ "transtype",
    /*  274 */ "trans_opt",
    /*  275 */ "savepoint_opt",
    /*  276 */ "kwcolumn_opt",
    /*  277 */ "fullname",
    /*  278 */ "add_column_fullname",
    /*  279 */ "as",
    /*  280 */ "groupby_opt",
    /*  281 */ "having_opt",
    /*  282 */ "window_clause",
    /*  283 */ "seltablist",
    /*  284 */ "on_using",
    /*  285 */ "joinop",
    /*  286 */ "stl_prefix",
    /*  287 */ "trigger_time",
    /*  288 */ "trnm",
    /*  289 */ "trigger_decl",
    /*  290 */ "trigger_cmd_list",
    /*  291 */ "trigger_event",
    /*  292 */ "foreach_clause",
    /*  293 */ "when_clause",
    /*  294 */ "trigger_cmd",
    /*  295 */ "tridxby",
    /*  296 */ "plus_num",
    /*  297 */ "minus_num",
    /*  298 */ "nmnum",
    /*  299 */ "uniqueflag",
    /*  300 */ "explain",
    /*  301 */ "database_kw_opt",
    /*  302 */ "key_opt",
    /*  303 */ "vinto",
    /*  304 */ "values",
    /*  305 */ "mvalues",
    /*  306 */ "create_vtab",
    /*  307 */ "vtabarglist",
    /*  308 */ "vtabarg",
    /*  309 */ "vtabargtoken",
    /*  310 */ "lp",
    /*  311 */ "anylist",
    /*  312 */ "range_or_rows",
    /*  313 */ "frame_exclude_opt",
    /*  314 */ "frame_exclude",
    /*  315 */ "windowdefn_list",
    /*  316 */ "windowdefn",
    /*  317 */ "window",
    /*  318 */ "frame_opt",
    /*  319 */ "frame_bound_s",
    /*  320 */ "frame_bound_e",
    /*  321 */ "frame_bound",
    /*  322 */ "filter_clause",
    /*  323 */ "over_clause",
};
#endif /* defined(YYCOVERAGE) || !defined(NDEBUG) */

#ifndef NDEBUG
/* For tracing reduce actions, the names of all rules are required.
 */
static const char* const yyRuleName[] = {
    /*   0 */ "input ::= cmdlist",
    /*   1 */ "cmdlist ::= cmdlist ecmd",
    /*   2 */ "cmdlist ::= ecmd",
    /*   3 */ "ecmd ::= SEMI",
    /*   4 */ "ecmd ::= cmdx SEMI",
    /*   5 */ "ecmd ::= error SEMI",
    /*   6 */ "cmdx ::= cmd",
    /*   7 */
    "expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist ORDER BY sortlist RP",
    /*   8 */
    "expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist ORDER BY sortlist RP "
    "filter_over",
    /*   9 */
    "expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist RP WITHIN GROUP LP ORDER "
    "BY expr RP",
    /*  10 */
    "expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist RP WITHIN GROUP LP ORDER "
    "BY expr RP filter_over",
    /*  11 */ "expr ::= CAST LP expr AS typetoken RP",
    /*  12 */ "typetoken ::=",
    /*  13 */ "typetoken ::= typename",
    /*  14 */ "typetoken ::= typename LP signed RP",
    /*  15 */ "typetoken ::= typename LP signed COMMA signed RP",
    /*  16 */ "typename ::= ID|STRING",
    /*  17 */ "typename ::= typename ID|STRING",
    /*  18 */ "selcollist ::= sclp scanpt nm DOT STAR",
    /*  19 */ "expr ::= ID|INDEXED|JOIN_KW",
    /*  20 */ "expr ::= nm DOT nm",
    /*  21 */ "expr ::= nm DOT nm DOT nm",
    /*  22 */ "selectnowith ::= selectnowith multiselect_op oneselect",
    /*  23 */ "multiselect_op ::= UNION",
    /*  24 */ "multiselect_op ::= UNION ALL",
    /*  25 */ "multiselect_op ::= EXCEPT|INTERSECT",
    /*  26 */ "expr ::= LP select RP",
    /*  27 */ "expr ::= EXISTS LP select RP",
    /*  28 */ "in_op ::= IN",
    /*  29 */ "in_op ::= NOT IN",
    /*  30 */ "expr ::= expr in_op LP exprlist RP",
    /*  31 */ "expr ::= expr in_op LP select RP",
    /*  32 */ "expr ::= expr in_op nm dbnm paren_exprlist",
    /*  33 */ "dbnm ::=",
    /*  34 */ "dbnm ::= DOT nm",
    /*  35 */ "paren_exprlist ::=",
    /*  36 */ "paren_exprlist ::= LP exprlist RP",
    /*  37 */ "expr ::= expr ISNULL|NOTNULL",
    /*  38 */ "expr ::= expr NOT NULL",
    /*  39 */ "expr ::= expr IS expr",
    /*  40 */ "expr ::= expr IS NOT expr",
    /*  41 */ "expr ::= expr IS NOT DISTINCT FROM expr",
    /*  42 */ "expr ::= expr IS DISTINCT FROM expr",
    /*  43 */ "between_op ::= BETWEEN",
    /*  44 */ "between_op ::= NOT BETWEEN",
    /*  45 */ "expr ::= expr between_op expr AND expr",
    /*  46 */ "likeop ::= LIKE_KW|MATCH",
    /*  47 */ "likeop ::= NOT LIKE_KW|MATCH",
    /*  48 */ "expr ::= expr likeop expr",
    /*  49 */ "expr ::= expr likeop expr ESCAPE expr",
    /*  50 */ "expr ::= CASE case_operand case_exprlist case_else END",
    /*  51 */ "case_exprlist ::= case_exprlist WHEN expr THEN expr",
    /*  52 */ "case_exprlist ::= WHEN expr THEN expr",
    /*  53 */ "case_else ::= ELSE expr",
    /*  54 */ "case_else ::=",
    /*  55 */ "case_operand ::= expr",
    /*  56 */ "case_operand ::=",
    /*  57 */ "cmd ::= create_table create_table_args",
    /*  58 */ "create_table ::= createkw temp TABLE ifnotexists nm dbnm",
    /*  59 */
    "create_table_args ::= LP columnlist conslist_opt RP table_option_set",
    /*  60 */ "create_table_args ::= AS select",
    /*  61 */ "table_option_set ::=",
    /*  62 */ "table_option_set ::= table_option",
    /*  63 */ "table_option_set ::= table_option_set COMMA table_option",
    /*  64 */ "table_option ::= WITHOUT nm",
    /*  65 */ "table_option ::= nm",
    /*  66 */ "columnlist ::= columnlist COMMA columnname carglist",
    /*  67 */ "columnlist ::= columnname carglist",
    /*  68 */ "carglist ::= carglist ccons",
    /*  69 */ "carglist ::=",
    /*  70 */ "ccons ::= CONSTRAINT nm",
    /*  71 */ "ccons ::= DEFAULT scantok term",
    /*  72 */ "ccons ::= DEFAULT LP expr RP",
    /*  73 */ "ccons ::= DEFAULT PLUS scantok term",
    /*  74 */ "ccons ::= DEFAULT MINUS scantok term",
    /*  75 */ "ccons ::= DEFAULT scantok ID|INDEXED",
    /*  76 */ "ccons ::= NULL onconf",
    /*  77 */ "ccons ::= NOT NULL onconf",
    /*  78 */ "ccons ::= PRIMARY KEY sortorder onconf autoinc",
    /*  79 */ "ccons ::= UNIQUE onconf",
    /*  80 */ "ccons ::= CHECK LP expr RP",
    /*  81 */ "ccons ::= REFERENCES nm eidlist_opt refargs",
    /*  82 */ "ccons ::= defer_subclause",
    /*  83 */ "ccons ::= COLLATE ID|STRING",
    /*  84 */ "ccons ::= GENERATED ALWAYS AS generated",
    /*  85 */ "ccons ::= AS generated",
    /*  86 */ "generated ::= LP expr RP",
    /*  87 */ "generated ::= LP expr RP ID",
    /*  88 */ "autoinc ::=",
    /*  89 */ "autoinc ::= AUTOINCR",
    /*  90 */ "refargs ::=",
    /*  91 */ "refargs ::= refargs refarg",
    /*  92 */ "refarg ::= MATCH nm",
    /*  93 */ "refarg ::= ON INSERT refact",
    /*  94 */ "refarg ::= ON DELETE refact",
    /*  95 */ "refarg ::= ON UPDATE refact",
    /*  96 */ "refact ::= SET NULL",
    /*  97 */ "refact ::= SET DEFAULT",
    /*  98 */ "refact ::= CASCADE",
    /*  99 */ "refact ::= RESTRICT",
    /* 100 */ "refact ::= NO ACTION",
    /* 101 */ "defer_subclause ::= NOT DEFERRABLE init_deferred_pred_opt",
    /* 102 */ "defer_subclause ::= DEFERRABLE init_deferred_pred_opt",
    /* 103 */ "init_deferred_pred_opt ::=",
    /* 104 */ "init_deferred_pred_opt ::= INITIALLY DEFERRED",
    /* 105 */ "init_deferred_pred_opt ::= INITIALLY IMMEDIATE",
    /* 106 */ "conslist_opt ::=",
    /* 107 */ "conslist_opt ::= COMMA conslist",
    /* 108 */ "conslist ::= conslist tconscomma tcons",
    /* 109 */ "conslist ::= tcons",
    /* 110 */ "tconscomma ::= COMMA",
    /* 111 */ "tconscomma ::=",
    /* 112 */ "tcons ::= CONSTRAINT nm",
    /* 113 */ "tcons ::= PRIMARY KEY LP sortlist autoinc RP onconf",
    /* 114 */ "tcons ::= UNIQUE LP sortlist RP onconf",
    /* 115 */ "tcons ::= CHECK LP expr RP onconf",
    /* 116 */
    "tcons ::= FOREIGN KEY LP eidlist RP REFERENCES nm eidlist_opt refargs "
    "defer_subclause_opt",
    /* 117 */ "defer_subclause_opt ::=",
    /* 118 */ "defer_subclause_opt ::= defer_subclause",
    /* 119 */ "onconf ::=",
    /* 120 */ "onconf ::= ON CONFLICT resolvetype",
    /* 121 */ "scantok ::=",
    /* 122 */ "select ::= WITH wqlist selectnowith",
    /* 123 */ "select ::= WITH RECURSIVE wqlist selectnowith",
    /* 124 */ "wqitem ::= withnm eidlist_opt wqas LP select RP",
    /* 125 */ "wqlist ::= wqitem",
    /* 126 */ "wqlist ::= wqlist COMMA wqitem",
    /* 127 */ "withnm ::= nm",
    /* 128 */ "wqas ::= AS",
    /* 129 */ "wqas ::= AS MATERIALIZED",
    /* 130 */ "wqas ::= AS NOT MATERIALIZED",
    /* 131 */ "eidlist_opt ::=",
    /* 132 */ "eidlist_opt ::= LP eidlist RP",
    /* 133 */ "eidlist ::= nm collate sortorder",
    /* 134 */ "eidlist ::= eidlist COMMA nm collate sortorder",
    /* 135 */ "collate ::=",
    /* 136 */ "collate ::= COLLATE ID|STRING",
    /* 137 */ "with ::=",
    /* 138 */ "with ::= WITH wqlist",
    /* 139 */ "with ::= WITH RECURSIVE wqlist",
    /* 140 */
    "cmd ::= with DELETE FROM xfullname indexed_opt where_opt_ret orderby_opt "
    "limit_opt",
    /* 141 */
    "cmd ::= with UPDATE orconf xfullname indexed_opt SET setlist from "
    "where_opt_ret orderby_opt limit_opt",
    /* 142 */ "cmd ::= with insert_cmd INTO xfullname idlist_opt select upsert",
    /* 143 */
    "cmd ::= with insert_cmd INTO xfullname idlist_opt DEFAULT VALUES "
    "returning",
    /* 144 */ "insert_cmd ::= INSERT orconf",
    /* 145 */ "insert_cmd ::= REPLACE",
    /* 146 */ "orconf ::=",
    /* 147 */ "orconf ::= OR resolvetype",
    /* 148 */ "resolvetype ::= raisetype",
    /* 149 */ "resolvetype ::= IGNORE",
    /* 150 */ "resolvetype ::= REPLACE",
    /* 151 */ "xfullname ::= nm",
    /* 152 */ "xfullname ::= nm DOT nm",
    /* 153 */ "xfullname ::= nm DOT nm AS nm",
    /* 154 */ "xfullname ::= nm AS nm",
    /* 155 */ "indexed_opt ::=",
    /* 156 */ "indexed_opt ::= indexed_by",
    /* 157 */ "where_opt_ret ::=",
    /* 158 */ "where_opt_ret ::= WHERE expr",
    /* 159 */ "where_opt_ret ::= RETURNING selcollist",
    /* 160 */ "where_opt_ret ::= WHERE expr RETURNING selcollist",
    /* 161 */ "setlist ::= setlist COMMA nm EQ expr",
    /* 162 */ "setlist ::= setlist COMMA LP idlist RP EQ expr",
    /* 163 */ "setlist ::= nm EQ expr",
    /* 164 */ "setlist ::= LP idlist RP EQ expr",
    /* 165 */ "idlist_opt ::=",
    /* 166 */ "idlist_opt ::= LP idlist RP",
    /* 167 */ "upsert ::=",
    /* 168 */ "upsert ::= RETURNING selcollist",
    /* 169 */
    "upsert ::= ON CONFLICT LP sortlist RP where_opt DO UPDATE SET setlist "
    "where_opt upsert",
    /* 170 */
    "upsert ::= ON CONFLICT LP sortlist RP where_opt DO NOTHING upsert",
    /* 171 */ "upsert ::= ON CONFLICT DO NOTHING returning",
    /* 172 */
    "upsert ::= ON CONFLICT DO UPDATE SET setlist where_opt returning",
    /* 173 */ "returning ::= RETURNING selcollist",
    /* 174 */ "returning ::=",
    /* 175 */ "expr ::= term",
    /* 176 */ "expr ::= LP expr RP",
    /* 177 */ "expr ::= expr PLUS|MINUS expr",
    /* 178 */ "expr ::= expr STAR|SLASH|REM expr",
    /* 179 */ "expr ::= expr LT|GT|GE|LE expr",
    /* 180 */ "expr ::= expr EQ|NE expr",
    /* 181 */ "expr ::= expr AND expr",
    /* 182 */ "expr ::= expr OR expr",
    /* 183 */ "expr ::= expr BITAND|BITOR|LSHIFT|RSHIFT expr",
    /* 184 */ "expr ::= expr CONCAT expr",
    /* 185 */ "expr ::= expr PTR expr",
    /* 186 */ "expr ::= PLUS|MINUS expr",
    /* 187 */ "expr ::= BITNOT expr",
    /* 188 */ "expr ::= NOT expr",
    /* 189 */ "exprlist ::= nexprlist",
    /* 190 */ "exprlist ::=",
    /* 191 */ "nexprlist ::= nexprlist COMMA expr",
    /* 192 */ "nexprlist ::= expr",
    /* 193 */ "expr ::= LP nexprlist COMMA expr RP",
    /* 194 */ "expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist RP",
    /* 195 */ "expr ::= ID|INDEXED|JOIN_KW LP STAR RP",
    /* 196 */ "expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist RP filter_over",
    /* 197 */ "expr ::= ID|INDEXED|JOIN_KW LP STAR RP filter_over",
    /* 198 */ "nm ::= ID|INDEXED|JOIN_KW",
    /* 199 */ "nm ::= STRING",
    /* 200 */ "term ::= INTEGER",
    /* 201 */ "term ::= STRING",
    /* 202 */ "term ::= NULL|FLOAT|BLOB",
    /* 203 */ "term ::= QNUMBER",
    /* 204 */ "term ::= CTIME_KW",
    /* 205 */ "expr ::= VARIABLE",
    /* 206 */ "expr ::= expr COLLATE ID|STRING",
    /* 207 */ "sortlist ::= sortlist COMMA expr sortorder nulls",
    /* 208 */ "sortlist ::= expr sortorder nulls",
    /* 209 */ "sortorder ::= ASC",
    /* 210 */ "sortorder ::= DESC",
    /* 211 */ "sortorder ::=",
    /* 212 */ "nulls ::= NULLS FIRST",
    /* 213 */ "nulls ::= NULLS LAST",
    /* 214 */ "nulls ::=",
    /* 215 */ "expr ::= RAISE LP IGNORE RP",
    /* 216 */ "expr ::= RAISE LP raisetype COMMA expr RP",
    /* 217 */ "raisetype ::= ROLLBACK",
    /* 218 */ "raisetype ::= ABORT",
    /* 219 */ "raisetype ::= FAIL",
    /* 220 */ "fullname ::= nm",
    /* 221 */ "fullname ::= nm DOT nm",
    /* 222 */ "ifexists ::= IF EXISTS",
    /* 223 */ "ifexists ::=",
    /* 224 */ "cmd ::= DROP TABLE ifexists fullname",
    /* 225 */ "cmd ::= DROP VIEW ifexists fullname",
    /* 226 */ "cmd ::= DROP INDEX ifexists fullname",
    /* 227 */ "cmd ::= DROP TRIGGER ifexists fullname",
    /* 228 */ "cmd ::= ALTER TABLE fullname RENAME TO nm",
    /* 229 */ "cmd ::= ALTER TABLE fullname RENAME kwcolumn_opt nm TO nm",
    /* 230 */ "cmd ::= ALTER TABLE fullname DROP kwcolumn_opt nm",
    /* 231 */
    "cmd ::= ALTER TABLE add_column_fullname ADD kwcolumn_opt columnname "
    "carglist",
    /* 232 */ "add_column_fullname ::= fullname",
    /* 233 */ "kwcolumn_opt ::=",
    /* 234 */ "kwcolumn_opt ::= COLUMNKW",
    /* 235 */ "columnname ::= nm typetoken",
    /* 236 */ "cmd ::= BEGIN transtype trans_opt",
    /* 237 */ "cmd ::= COMMIT|END trans_opt",
    /* 238 */ "cmd ::= ROLLBACK trans_opt",
    /* 239 */ "transtype ::=",
    /* 240 */ "transtype ::= DEFERRED",
    /* 241 */ "transtype ::= IMMEDIATE",
    /* 242 */ "transtype ::= EXCLUSIVE",
    /* 243 */ "trans_opt ::=",
    /* 244 */ "trans_opt ::= TRANSACTION",
    /* 245 */ "trans_opt ::= TRANSACTION nm",
    /* 246 */ "savepoint_opt ::= SAVEPOINT",
    /* 247 */ "savepoint_opt ::=",
    /* 248 */ "cmd ::= SAVEPOINT nm",
    /* 249 */ "cmd ::= RELEASE savepoint_opt nm",
    /* 250 */ "cmd ::= ROLLBACK trans_opt TO savepoint_opt nm",
    /* 251 */ "cmd ::= select",
    /* 252 */ "select ::= selectnowith",
    /* 253 */ "selectnowith ::= oneselect",
    /* 254 */
    "oneselect ::= SELECT distinct selcollist from where_opt groupby_opt "
    "having_opt orderby_opt limit_opt",
    /* 255 */
    "oneselect ::= SELECT distinct selcollist from where_opt groupby_opt "
    "having_opt window_clause orderby_opt limit_opt",
    /* 256 */ "selcollist ::= sclp scanpt expr scanpt as",
    /* 257 */ "selcollist ::= sclp scanpt STAR",
    /* 258 */ "sclp ::= selcollist COMMA",
    /* 259 */ "sclp ::=",
    /* 260 */ "scanpt ::=",
    /* 261 */ "as ::= AS nm",
    /* 262 */ "as ::= ID|STRING",
    /* 263 */ "as ::=",
    /* 264 */ "distinct ::= DISTINCT",
    /* 265 */ "distinct ::= ALL",
    /* 266 */ "distinct ::=",
    /* 267 */ "from ::=",
    /* 268 */ "from ::= FROM seltablist",
    /* 269 */ "where_opt ::=",
    /* 270 */ "where_opt ::= WHERE expr",
    /* 271 */ "groupby_opt ::=",
    /* 272 */ "groupby_opt ::= GROUP BY nexprlist",
    /* 273 */ "having_opt ::=",
    /* 274 */ "having_opt ::= HAVING expr",
    /* 275 */ "orderby_opt ::=",
    /* 276 */ "orderby_opt ::= ORDER BY sortlist",
    /* 277 */ "limit_opt ::=",
    /* 278 */ "limit_opt ::= LIMIT expr",
    /* 279 */ "limit_opt ::= LIMIT expr OFFSET expr",
    /* 280 */ "limit_opt ::= LIMIT expr COMMA expr",
    /* 281 */ "stl_prefix ::= seltablist joinop",
    /* 282 */ "stl_prefix ::=",
    /* 283 */ "seltablist ::= stl_prefix nm dbnm as on_using",
    /* 284 */ "seltablist ::= stl_prefix nm dbnm as indexed_by on_using",
    /* 285 */ "seltablist ::= stl_prefix nm dbnm LP exprlist RP as on_using",
    /* 286 */ "seltablist ::= stl_prefix LP select RP as on_using",
    /* 287 */ "seltablist ::= stl_prefix LP seltablist RP as on_using",
    /* 288 */ "joinop ::= COMMA|JOIN",
    /* 289 */ "joinop ::= JOIN_KW JOIN",
    /* 290 */ "joinop ::= JOIN_KW nm JOIN",
    /* 291 */ "joinop ::= JOIN_KW nm nm JOIN",
    /* 292 */ "on_using ::= ON expr",
    /* 293 */ "on_using ::= USING LP idlist RP",
    /* 294 */ "on_using ::=",
    /* 295 */ "indexed_by ::= INDEXED BY nm",
    /* 296 */ "indexed_by ::= NOT INDEXED",
    /* 297 */ "idlist ::= idlist COMMA nm",
    /* 298 */ "idlist ::= nm",
    /* 299 */ "cmd ::= createkw trigger_decl BEGIN trigger_cmd_list END",
    /* 300 */
    "trigger_decl ::= temp TRIGGER ifnotexists nm dbnm trigger_time "
    "trigger_event ON fullname foreach_clause when_clause",
    /* 301 */ "trigger_time ::= BEFORE|AFTER",
    /* 302 */ "trigger_time ::= INSTEAD OF",
    /* 303 */ "trigger_time ::=",
    /* 304 */ "trigger_event ::= DELETE|INSERT",
    /* 305 */ "trigger_event ::= UPDATE",
    /* 306 */ "trigger_event ::= UPDATE OF idlist",
    /* 307 */ "foreach_clause ::=",
    /* 308 */ "foreach_clause ::= FOR EACH ROW",
    /* 309 */ "when_clause ::=",
    /* 310 */ "when_clause ::= WHEN expr",
    /* 311 */ "trigger_cmd_list ::= trigger_cmd_list trigger_cmd SEMI",
    /* 312 */ "trigger_cmd_list ::= trigger_cmd SEMI",
    /* 313 */ "trnm ::= nm",
    /* 314 */ "trnm ::= nm DOT nm",
    /* 315 */ "tridxby ::=",
    /* 316 */ "tridxby ::= INDEXED BY nm",
    /* 317 */ "tridxby ::= NOT INDEXED",
    /* 318 */
    "trigger_cmd ::= UPDATE orconf trnm tridxby SET setlist from where_opt "
    "scanpt",
    /* 319 */
    "trigger_cmd ::= scanpt insert_cmd INTO trnm idlist_opt select upsert "
    "scanpt",
    /* 320 */ "trigger_cmd ::= DELETE FROM trnm tridxby where_opt scanpt",
    /* 321 */ "trigger_cmd ::= scanpt select scanpt",
    /* 322 */ "cmd ::= PRAGMA nm dbnm",
    /* 323 */ "cmd ::= PRAGMA nm dbnm EQ nmnum",
    /* 324 */ "cmd ::= PRAGMA nm dbnm LP nmnum RP",
    /* 325 */ "cmd ::= PRAGMA nm dbnm EQ minus_num",
    /* 326 */ "cmd ::= PRAGMA nm dbnm LP minus_num RP",
    /* 327 */ "nmnum ::= plus_num",
    /* 328 */ "nmnum ::= nm",
    /* 329 */ "nmnum ::= ON",
    /* 330 */ "nmnum ::= DELETE",
    /* 331 */ "nmnum ::= DEFAULT",
    /* 332 */ "plus_num ::= PLUS INTEGER|FLOAT",
    /* 333 */ "plus_num ::= INTEGER|FLOAT",
    /* 334 */ "minus_num ::= MINUS INTEGER|FLOAT",
    /* 335 */ "signed ::= plus_num",
    /* 336 */ "signed ::= minus_num",
    /* 337 */ "cmd ::= ANALYZE",
    /* 338 */ "cmd ::= ANALYZE nm dbnm",
    /* 339 */ "cmd ::= REINDEX",
    /* 340 */ "cmd ::= REINDEX nm dbnm",
    /* 341 */ "cmd ::= ATTACH database_kw_opt expr AS expr key_opt",
    /* 342 */ "cmd ::= DETACH database_kw_opt expr",
    /* 343 */ "database_kw_opt ::= DATABASE",
    /* 344 */ "database_kw_opt ::=",
    /* 345 */ "key_opt ::=",
    /* 346 */ "key_opt ::= KEY expr",
    /* 347 */ "cmd ::= VACUUM vinto",
    /* 348 */ "cmd ::= VACUUM nm vinto",
    /* 349 */ "vinto ::= INTO expr",
    /* 350 */ "vinto ::=",
    /* 351 */ "ecmd ::= explain cmdx SEMI",
    /* 352 */ "explain ::= EXPLAIN",
    /* 353 */ "explain ::= EXPLAIN QUERY PLAN",
    /* 354 */
    "cmd ::= createkw uniqueflag INDEX ifnotexists nm dbnm ON nm LP sortlist "
    "RP where_opt",
    /* 355 */ "uniqueflag ::= UNIQUE",
    /* 356 */ "uniqueflag ::=",
    /* 357 */ "ifnotexists ::=",
    /* 358 */ "ifnotexists ::= IF NOT EXISTS",
    /* 359 */
    "cmd ::= createkw temp VIEW ifnotexists nm dbnm eidlist_opt AS select",
    /* 360 */ "createkw ::= CREATE",
    /* 361 */ "temp ::= TEMP",
    /* 362 */ "temp ::=",
    /* 363 */ "values ::= VALUES LP nexprlist RP",
    /* 364 */ "mvalues ::= values COMMA LP nexprlist RP",
    /* 365 */ "mvalues ::= mvalues COMMA LP nexprlist RP",
    /* 366 */ "oneselect ::= values",
    /* 367 */ "oneselect ::= mvalues",
    /* 368 */ "cmd ::= create_vtab",
    /* 369 */ "cmd ::= create_vtab LP vtabarglist RP",
    /* 370 */
    "create_vtab ::= createkw VIRTUAL TABLE ifnotexists nm dbnm USING nm",
    /* 371 */ "vtabarglist ::= vtabarg",
    /* 372 */ "vtabarglist ::= vtabarglist COMMA vtabarg",
    /* 373 */ "vtabarg ::=",
    /* 374 */ "vtabarg ::= vtabarg vtabargtoken",
    /* 375 */ "vtabargtoken ::= ANY",
    /* 376 */ "vtabargtoken ::= lp anylist RP",
    /* 377 */ "lp ::= LP",
    /* 378 */ "anylist ::=",
    /* 379 */ "anylist ::= anylist LP anylist RP",
    /* 380 */ "anylist ::= anylist ANY",
    /* 381 */ "windowdefn_list ::= windowdefn",
    /* 382 */ "windowdefn_list ::= windowdefn_list COMMA windowdefn",
    /* 383 */ "windowdefn ::= nm AS LP window RP",
    /* 384 */ "window ::= PARTITION BY nexprlist orderby_opt frame_opt",
    /* 385 */ "window ::= nm PARTITION BY nexprlist orderby_opt frame_opt",
    /* 386 */ "window ::= ORDER BY sortlist frame_opt",
    /* 387 */ "window ::= nm ORDER BY sortlist frame_opt",
    /* 388 */ "window ::= frame_opt",
    /* 389 */ "window ::= nm frame_opt",
    /* 390 */ "frame_opt ::=",
    /* 391 */ "frame_opt ::= range_or_rows frame_bound_s frame_exclude_opt",
    /* 392 */
    "frame_opt ::= range_or_rows BETWEEN frame_bound_s AND frame_bound_e "
    "frame_exclude_opt",
    /* 393 */ "range_or_rows ::= RANGE|ROWS|GROUPS",
    /* 394 */ "frame_bound_s ::= frame_bound",
    /* 395 */ "frame_bound_s ::= UNBOUNDED PRECEDING",
    /* 396 */ "frame_bound_e ::= frame_bound",
    /* 397 */ "frame_bound_e ::= UNBOUNDED FOLLOWING",
    /* 398 */ "frame_bound ::= expr PRECEDING|FOLLOWING",
    /* 399 */ "frame_bound ::= CURRENT ROW",
    /* 400 */ "frame_exclude_opt ::=",
    /* 401 */ "frame_exclude_opt ::= EXCLUDE frame_exclude",
    /* 402 */ "frame_exclude ::= NO OTHERS",
    /* 403 */ "frame_exclude ::= CURRENT ROW",
    /* 404 */ "frame_exclude ::= GROUP|TIES",
    /* 405 */ "window_clause ::= WINDOW windowdefn_list",
    /* 406 */ "filter_over ::= filter_clause over_clause",
    /* 407 */ "filter_over ::= over_clause",
    /* 408 */ "filter_over ::= filter_clause",
    /* 409 */ "over_clause ::= OVER LP window RP",
    /* 410 */ "over_clause ::= OVER nm",
    /* 411 */ "filter_clause ::= FILTER LP WHERE expr RP",
};
#endif /* NDEBUG */

#if YYGROWABLESTACK
/*
** Try to increase the size of the parser stack.  Return the number
** of errors.  Return 0 on success.
*/
static int yyGrowStack(yyParser* p) {
  int oldSize = 1 + (int)(p->yystackEnd - p->yystack);
  int newSize;
  int idx;
  yyStackEntry* pNew;

  newSize = oldSize * 2 + 100;
  idx = (int)(p->yytos - p->yystack);
  if (p->yystack == p->yystk0) {
    pNew = YYREALLOC(0, newSize * sizeof(pNew[0]));
    if (pNew == 0)
      return 1;
    memcpy(pNew, p->yystack, oldSize * sizeof(pNew[0]));
  } else {
    pNew = YYREALLOC(p->yystack, newSize * sizeof(pNew[0]));
    if (pNew == 0)
      return 1;
  }
  p->yystack = pNew;
  p->yytos = &p->yystack[idx];
#ifndef NDEBUG
  if (yyTraceFILE) {
    fprintf(yyTraceFILE, "%sStack grows from %d to %d entries.\n",
            yyTracePrompt, oldSize, newSize);
  }
#endif
  p->yystackEnd = &p->yystack[newSize - 1];
  return 0;
}
#endif /* YYGROWABLESTACK */

#if !YYGROWABLESTACK
/* For builds that do no have a growable stack, yyGrowStack always
** returns an error.
*/
#define yyGrowStack(X) 1
#endif

/* Datatype of the argument to the memory allocated passed as the
** second argument to SynqSqliteParseAlloc() below.  This can be changed by
** putting an appropriate #define in the %include section of the input
** grammar.
*/
#ifndef YYMALLOCARGTYPE
#define YYMALLOCARGTYPE size_t
#endif

/* Initialize a new parser that has already been allocated.
 */
void SynqSqliteParseInit(void* yypRawParser SynqSqliteParseCTX_PDECL) {
  yyParser* yypParser = (yyParser*)yypRawParser;
  SynqSqliteParseCTX_STORE
#ifdef YYTRACKMAXSTACKDEPTH
      yypParser->yyhwm = 0;
#endif
  yypParser->yystack = yypParser->yystk0;
  yypParser->yystackEnd = &yypParser->yystack[YYSTACKDEPTH - 1];
#ifndef YYNOERRORRECOVERY
  yypParser->yyerrcnt = -1;
#endif
  yypParser->yytos = yypParser->yystack;
  yypParser->yystack[0].stateno = 0;
  yypParser->yystack[0].major = 0;
}

#ifndef SynqSqliteParse_ENGINEALWAYSONSTACK
/*
** This function allocates a new parser.
** The only argument is a pointer to a function which works like
** malloc.
**
** Inputs:
** A pointer to the function used to allocate memory.
**
** Outputs:
** A pointer to a parser.  This pointer is used in subsequent calls
** to SynqSqliteParse and SynqSqliteParseFree.
*/
void* SynqSqliteParseAlloc(void* (*mallocProc)(YYMALLOCARGTYPE)
                               SynqSqliteParseCTX_PDECL) {
  yyParser* yypParser;
  yypParser = (yyParser*)(*mallocProc)((YYMALLOCARGTYPE)sizeof(yyParser));
  if (yypParser) {
    SynqSqliteParseCTX_STORE SynqSqliteParseInit(
        yypParser SynqSqliteParseCTX_PARAM);
  }
  return (void*)yypParser;
}
#endif /* SynqSqliteParse_ENGINEALWAYSONSTACK */

/* The following function deletes the "minor type" or semantic value
** associated with a symbol.  The symbol can be either a terminal
** or nonterminal. "yymajor" is the symbol code, and "yypminor" is
** a pointer to the value to be deleted.  The code used to do the
** deletions is derived from the %destructor and/or %token_destructor
** directives of the input grammar.
*/
static void yy_destructor(
    yyParser* yypParser,  /* The parser */
    YYCODETYPE yymajor,   /* Type code for object to destroy */
    YYMINORTYPE* yypminor /* The object to be destroyed */
) {
  SynqSqliteParseARG_FETCH SynqSqliteParseCTX_FETCH switch (yymajor) {
      /* Here is inserted the actions which take place when a
      ** terminal or non-terminal is destroyed.  This can happen
      ** when the symbol is popped from the stack during a
      ** reduce or during error processing or when a parser is
      ** being destroyed before it is finished parsing.
      **
      ** Note: during a reduce, the only symbols destroyed are those
      ** which appear on the RHS of the rule, but which are *not* used
      ** inside the C code.
      */
      /********* Begin destructor definitions
       * ***************************************/
      /********* End destructor definitions
       * *****************************************/
    default:
      break; /* If no destructor action specified: do nothing */
  }
}

/*
** Pop the parser's stack once.
**
** If there is a destructor routine associated with the token which
** is popped from the stack, then call it.
*/
static void yy_pop_parser_stack(yyParser* pParser) {
  yyStackEntry* yytos;
  assert(pParser->yytos != 0);
  assert(pParser->yytos > pParser->yystack);
  yytos = pParser->yytos--;
#ifndef NDEBUG
  if (yyTraceFILE) {
    fprintf(yyTraceFILE, "%sPopping %s\n", yyTracePrompt,
            yyTokenName[yytos->major]);
  }
#endif
  yy_destructor(pParser, yytos->major, &yytos->minor);
}

/*
** Clear all secondary memory allocations from the parser
*/
void SynqSqliteParseFinalize(void* p) {
  yyParser* pParser = (yyParser*)p;

  /* In-lined version of calling yy_pop_parser_stack() for each
  ** element left in the stack */
  yyStackEntry* yytos = pParser->yytos;
  while (yytos > pParser->yystack) {
#ifndef NDEBUG
    if (yyTraceFILE) {
      fprintf(yyTraceFILE, "%sPopping %s\n", yyTracePrompt,
              yyTokenName[yytos->major]);
    }
#endif
    if (yytos->major >= YY_MIN_DSTRCTR) {
      yy_destructor(pParser, yytos->major, &yytos->minor);
    }
    yytos--;
  }

#if YYGROWABLESTACK
  if (pParser->yystack != pParser->yystk0)
    YYFREE(pParser->yystack);
#endif
}

#ifndef SynqSqliteParse_ENGINEALWAYSONSTACK
/*
** Deallocate and destroy a parser.  Destructors are called for
** all stack elements before shutting the parser down.
**
** If the YYPARSEFREENEVERNULL macro exists (for example because it
** is defined in a %include section of the input grammar) then it is
** assumed that the input pointer is never NULL.
*/
void SynqSqliteParseFree(
    void* p,                /* The parser to be deleted */
    void (*freeProc)(void*) /* Function used to reclaim memory */
) {
#ifndef YYPARSEFREENEVERNULL
  if (p == 0)
    return;
#endif
  SynqSqliteParseFinalize(p);
  (*freeProc)(p);
}
#endif /* SynqSqliteParse_ENGINEALWAYSONSTACK */

/*
** Return the peak depth of the stack for a parser.
*/
#ifdef YYTRACKMAXSTACKDEPTH
int SynqSqliteParseStackPeak(void* p) {
  yyParser* pParser = (yyParser*)p;
  return pParser->yyhwm;
}
#endif

/* This array of booleans keeps track of the parser statement
** coverage.  The element yycoverage[X][Y] is set when the parser
** is in state X and has a lookahead token Y.  In a well-tested
** systems, every element of this matrix should end up being set.
*/
#if defined(YYCOVERAGE)
static unsigned char yycoverage[YYNSTATE][YYNTOKEN];
#endif

/*
** Write into out a description of every state/lookahead combination that
**
**   (1)  has not been used by the parser, and
**   (2)  is not a syntax error.
**
** Return the number of missed state/lookahead combinations.
*/
#if defined(YYCOVERAGE)
int SynqSqliteParseCoverage(FILE* out) {
  int stateno, iLookAhead, i;
  int nMissed = 0;
  for (stateno = 0; stateno < YYNSTATE; stateno++) {
    i = yy_shift_ofst[stateno];
    for (iLookAhead = 0; iLookAhead < YYNTOKEN; iLookAhead++) {
      if (yy_lookahead[i + iLookAhead] != iLookAhead)
        continue;
      if (yycoverage[stateno][iLookAhead] == 0)
        nMissed++;
      if (out) {
        fprintf(out, "State %d lookahead %s %s\n", stateno,
                yyTokenName[iLookAhead],
                yycoverage[stateno][iLookAhead] ? "ok" : "missed");
      }
    }
  }
  return nMissed;
}
#endif

/*
** Find the appropriate action for a parser given the terminal
** look-ahead token iLookAhead.
*/
static YYACTIONTYPE yy_find_shift_action(
    YYCODETYPE iLookAhead, /* The look-ahead token */
    YYACTIONTYPE stateno   /* Current state number */
) {
  int i;

  if (stateno > YY_MAX_SHIFT)
    return stateno;
  assert(stateno <= YY_SHIFT_COUNT);
#if defined(YYCOVERAGE)
  yycoverage[stateno][iLookAhead] = 1;
#endif
  do {
    i = yy_shift_ofst[stateno];
    assert(i >= 0);
    assert(i <= YY_ACTTAB_COUNT);
    assert(i + YYNTOKEN <= (int)YY_NLOOKAHEAD);
    assert(iLookAhead != YYNOCODE);
    assert(iLookAhead < YYNTOKEN);
    i += iLookAhead;
    assert(i < (int)YY_NLOOKAHEAD);
    if (yy_lookahead[i] != iLookAhead) {
#ifdef YYFALLBACK
      YYCODETYPE iFallback; /* Fallback token */
      assert(iLookAhead < sizeof(yyFallback) / sizeof(yyFallback[0]));
      iFallback = yyFallback[iLookAhead];
      if (iFallback != 0) {
#ifndef NDEBUG
        if (yyTraceFILE) {
          fprintf(yyTraceFILE, "%sFALLBACK %s => %s\n", yyTracePrompt,
                  yyTokenName[iLookAhead], yyTokenName[iFallback]);
        }
#endif
        assert(yyFallback[iFallback] == 0); /* Fallback loop must terminate */
        iLookAhead = iFallback;
        continue;
      }
#endif
#ifdef YYWILDCARD
      {
        int j = i - iLookAhead + YYWILDCARD;
        assert(j < (int)(sizeof(yy_lookahead) / sizeof(yy_lookahead[0])));
        if (yy_lookahead[j] == YYWILDCARD && iLookAhead > 0) {
#ifndef NDEBUG
          if (yyTraceFILE) {
            fprintf(yyTraceFILE, "%sWILDCARD %s => %s\n", yyTracePrompt,
                    yyTokenName[iLookAhead], yyTokenName[YYWILDCARD]);
          }
#endif /* NDEBUG */
          return yy_action[j];
        }
      }
#endif /* YYWILDCARD */
      return yy_default[stateno];
    } else {
      assert(i >= 0 && i < (int)(sizeof(yy_action) / sizeof(yy_action[0])));
      return yy_action[i];
    }
  } while (1);
}

/*
** Find the appropriate action for a parser given the non-terminal
** look-ahead token iLookAhead.
*/
static YYACTIONTYPE yy_find_reduce_action(
    YYACTIONTYPE stateno, /* Current state number */
    YYCODETYPE iLookAhead /* The look-ahead token */
) {
  int i;
#ifdef YYERRORSYMBOL
  if (stateno > YY_REDUCE_COUNT) {
    return yy_default[stateno];
  }
#else
  assert(stateno <= YY_REDUCE_COUNT);
#endif
  i = yy_reduce_ofst[stateno];
  assert(iLookAhead != YYNOCODE);
  i += iLookAhead;
#ifdef YYERRORSYMBOL
  if (i < 0 || i >= YY_ACTTAB_COUNT || yy_lookahead[i] != iLookAhead) {
    return yy_default[stateno];
  }
#else
  assert(i >= 0 && i < YY_ACTTAB_COUNT);
  assert(yy_lookahead[i] == iLookAhead);
#endif
  return yy_action[i];
}

/*
** The following routine is called if the stack overflows.
*/
static void yyStackOverflow(yyParser* yypParser) {
  SynqSqliteParseARG_FETCH SynqSqliteParseCTX_FETCH
#ifndef NDEBUG
      if (yyTraceFILE) {
    fprintf(yyTraceFILE, "%sStack Overflow!\n", yyTracePrompt);
  }
#endif
  while (yypParser->yytos > yypParser->yystack)
    yy_pop_parser_stack(yypParser);
  /* Here code is inserted which will execute if the parser
  ** stack every overflows */
  /******** Begin %stack_overflow code
   * ******************************************/

  if (pCtx) {
    pCtx->error = 1;
  }
  /******** End %stack_overflow code
   * ********************************************/
  SynqSqliteParseARG_STORE /* Suppress warning about unused %extra_argument var
                            */
      SynqSqliteParseCTX_STORE
}

/*
** Print tracing information for a SHIFT action
*/
#ifndef NDEBUG
static void yyTraceShift(yyParser* yypParser,
                         int yyNewState,
                         const char* zTag) {
  if (yyTraceFILE) {
    if (yyNewState < YYNSTATE) {
      fprintf(yyTraceFILE, "%s%s '%s', go to state %d\n", yyTracePrompt, zTag,
              yyTokenName[yypParser->yytos->major], yyNewState);
    } else {
      fprintf(yyTraceFILE, "%s%s '%s', pending reduce %d\n", yyTracePrompt,
              zTag, yyTokenName[yypParser->yytos->major],
              yyNewState - YY_MIN_REDUCE);
    }
  }
}
#else
#define yyTraceShift(X, Y, Z)
#endif

/*
** Perform a shift action.
*/
static void yy_shift(
    yyParser* yypParser,             /* The parser to be shifted */
    YYACTIONTYPE yyNewState,         /* The new state to shift in */
    YYCODETYPE yyMajor,              /* The major token to shift in */
    SynqSqliteParseTOKENTYPE yyMinor /* The minor token to shift in */
) {
  yyStackEntry* yytos;
  yypParser->yytos++;
#ifdef YYTRACKMAXSTACKDEPTH
  if ((int)(yypParser->yytos - yypParser->yystack) > yypParser->yyhwm) {
    yypParser->yyhwm++;
    assert(yypParser->yyhwm == (int)(yypParser->yytos - yypParser->yystack));
  }
#endif
  yytos = yypParser->yytos;
  if (yytos > yypParser->yystackEnd) {
    if (yyGrowStack(yypParser)) {
      yypParser->yytos--;
      yyStackOverflow(yypParser);
      return;
    }
    yytos = yypParser->yytos;
    assert(yytos <= yypParser->yystackEnd);
  }
  if (yyNewState > YY_MAX_SHIFT) {
    yyNewState += YY_MIN_REDUCE - YY_MIN_SHIFTREDUCE;
  }
  yytos->stateno = yyNewState;
  yytos->major = yyMajor;
  yytos->minor.yy0 = yyMinor;
  yyTraceShift(yypParser, yyNewState, "Shift");
}

/* For rule J, yyRuleInfoLhs[J] contains the symbol on the left-hand side
** of that rule */
static const YYCODETYPE yyRuleInfoLhs[] = {
    188, /* (0) input ::= cmdlist */
    189, /* (1) cmdlist ::= cmdlist ecmd */
    189, /* (2) cmdlist ::= ecmd */
    190, /* (3) ecmd ::= SEMI */
    190, /* (4) ecmd ::= cmdx SEMI */
    190, /* (5) ecmd ::= error SEMI */
    191, /* (6) cmdx ::= cmd */
    194, /* (7) expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist ORDER BY
            sortlist RP */
    194, /* (8) expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist ORDER BY
            sortlist RP filter_over */
    194, /* (9) expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist RP WITHIN GROUP
            LP ORDER BY expr RP */
    194, /* (10) expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist RP WITHIN
            GROUP LP ORDER BY expr RP filter_over */
    194, /* (11) expr ::= CAST LP expr AS typetoken RP */
    199, /* (12) typetoken ::= */
    199, /* (13) typetoken ::= typename */
    199, /* (14) typetoken ::= typename LP signed RP */
    199, /* (15) typetoken ::= typename LP signed COMMA signed RP */
    200, /* (16) typename ::= ID|STRING */
    200, /* (17) typename ::= typename ID|STRING */
    202, /* (18) selcollist ::= sclp scanpt nm DOT STAR */
    194, /* (19) expr ::= ID|INDEXED|JOIN_KW */
    194, /* (20) expr ::= nm DOT nm */
    194, /* (21) expr ::= nm DOT nm DOT nm */
    209, /* (22) selectnowith ::= selectnowith multiselect_op oneselect */
    206, /* (23) multiselect_op ::= UNION */
    206, /* (24) multiselect_op ::= UNION ALL */
    206, /* (25) multiselect_op ::= EXCEPT|INTERSECT */
    194, /* (26) expr ::= LP select RP */
    194, /* (27) expr ::= EXISTS LP select RP */
    207, /* (28) in_op ::= IN */
    207, /* (29) in_op ::= NOT IN */
    194, /* (30) expr ::= expr in_op LP exprlist RP */
    194, /* (31) expr ::= expr in_op LP select RP */
    194, /* (32) expr ::= expr in_op nm dbnm paren_exprlist */
    208, /* (33) dbnm ::= */
    208, /* (34) dbnm ::= DOT nm */
    212, /* (35) paren_exprlist ::= */
    212, /* (36) paren_exprlist ::= LP exprlist RP */
    194, /* (37) expr ::= expr ISNULL|NOTNULL */
    194, /* (38) expr ::= expr NOT NULL */
    194, /* (39) expr ::= expr IS expr */
    194, /* (40) expr ::= expr IS NOT expr */
    194, /* (41) expr ::= expr IS NOT DISTINCT FROM expr */
    194, /* (42) expr ::= expr IS DISTINCT FROM expr */
    214, /* (43) between_op ::= BETWEEN */
    214, /* (44) between_op ::= NOT BETWEEN */
    194, /* (45) expr ::= expr between_op expr AND expr */
    213, /* (46) likeop ::= LIKE_KW|MATCH */
    213, /* (47) likeop ::= NOT LIKE_KW|MATCH */
    194, /* (48) expr ::= expr likeop expr */
    194, /* (49) expr ::= expr likeop expr ESCAPE expr */
    194, /* (50) expr ::= CASE case_operand case_exprlist case_else END */
    216, /* (51) case_exprlist ::= case_exprlist WHEN expr THEN expr */
    216, /* (52) case_exprlist ::= WHEN expr THEN expr */
    217, /* (53) case_else ::= ELSE expr */
    217, /* (54) case_else ::= */
    215, /* (55) case_operand ::= expr */
    215, /* (56) case_operand ::= */
    193, /* (57) cmd ::= create_table create_table_args */
    235, /* (58) create_table ::= createkw temp TABLE ifnotexists nm dbnm */
    236, /* (59) create_table_args ::= LP columnlist conslist_opt RP
            table_option_set */
    236, /* (60) create_table_args ::= AS select */
    226, /* (61) table_option_set ::= */
    226, /* (62) table_option_set ::= table_option */
    226, /* (63) table_option_set ::= table_option_set COMMA table_option */
    227, /* (64) table_option ::= WITHOUT nm */
    227, /* (65) table_option ::= nm */
    240, /* (66) columnlist ::= columnlist COMMA columnname carglist */
    240, /* (67) columnlist ::= columnname carglist */
    231, /* (68) carglist ::= carglist ccons */
    231, /* (69) carglist ::= */
    230, /* (70) ccons ::= CONSTRAINT nm */
    230, /* (71) ccons ::= DEFAULT scantok term */
    230, /* (72) ccons ::= DEFAULT LP expr RP */
    230, /* (73) ccons ::= DEFAULT PLUS scantok term */
    230, /* (74) ccons ::= DEFAULT MINUS scantok term */
    230, /* (75) ccons ::= DEFAULT scantok ID|INDEXED */
    230, /* (76) ccons ::= NULL onconf */
    230, /* (77) ccons ::= NOT NULL onconf */
    230, /* (78) ccons ::= PRIMARY KEY sortorder onconf autoinc */
    230, /* (79) ccons ::= UNIQUE onconf */
    230, /* (80) ccons ::= CHECK LP expr RP */
    230, /* (81) ccons ::= REFERENCES nm eidlist_opt refargs */
    230, /* (82) ccons ::= defer_subclause */
    230, /* (83) ccons ::= COLLATE ID|STRING */
    230, /* (84) ccons ::= GENERATED ALWAYS AS generated */
    230, /* (85) ccons ::= AS generated */
    234, /* (86) generated ::= LP expr RP */
    234, /* (87) generated ::= LP expr RP ID */
    219, /* (88) autoinc ::= */
    219, /* (89) autoinc ::= AUTOINCR */
    220, /* (90) refargs ::= */
    220, /* (91) refargs ::= refargs refarg */
    221, /* (92) refarg ::= MATCH nm */
    221, /* (93) refarg ::= ON INSERT refact */
    221, /* (94) refarg ::= ON DELETE refact */
    221, /* (95) refarg ::= ON UPDATE refact */
    222, /* (96) refact ::= SET NULL */
    222, /* (97) refact ::= SET DEFAULT */
    222, /* (98) refact ::= CASCADE */
    222, /* (99) refact ::= RESTRICT */
    222, /* (100) refact ::= NO ACTION */
    223, /* (101) defer_subclause ::= NOT DEFERRABLE init_deferred_pred_opt */
    223, /* (102) defer_subclause ::= DEFERRABLE init_deferred_pred_opt */
    224, /* (103) init_deferred_pred_opt ::= */
    224, /* (104) init_deferred_pred_opt ::= INITIALLY DEFERRED */
    224, /* (105) init_deferred_pred_opt ::= INITIALLY IMMEDIATE */
    241, /* (106) conslist_opt ::= */
    241, /* (107) conslist_opt ::= COMMA conslist */
    233, /* (108) conslist ::= conslist tconscomma tcons */
    233, /* (109) conslist ::= tcons */
    228, /* (110) tconscomma ::= COMMA */
    228, /* (111) tconscomma ::= */
    232, /* (112) tcons ::= CONSTRAINT nm */
    232, /* (113) tcons ::= PRIMARY KEY LP sortlist autoinc RP onconf */
    232, /* (114) tcons ::= UNIQUE LP sortlist RP onconf */
    232, /* (115) tcons ::= CHECK LP expr RP onconf */
    232, /* (116) tcons ::= FOREIGN KEY LP eidlist RP REFERENCES nm eidlist_opt
            refargs defer_subclause_opt */
    225, /* (117) defer_subclause_opt ::= */
    225, /* (118) defer_subclause_opt ::= defer_subclause */
    229, /* (119) onconf ::= */
    229, /* (120) onconf ::= ON CONFLICT resolvetype */
    218, /* (121) scantok ::= */
    211, /* (122) select ::= WITH wqlist selectnowith */
    211, /* (123) select ::= WITH RECURSIVE wqlist selectnowith */
    252, /* (124) wqitem ::= withnm eidlist_opt wqas LP select RP */
    251, /* (125) wqlist ::= wqitem */
    251, /* (126) wqlist ::= wqlist COMMA wqitem */
    248, /* (127) withnm ::= nm */
    249, /* (128) wqas ::= AS */
    249, /* (129) wqas ::= AS MATERIALIZED */
    249, /* (130) wqas ::= AS NOT MATERIALIZED */
    245, /* (131) eidlist_opt ::= */
    245, /* (132) eidlist_opt ::= LP eidlist RP */
    246, /* (133) eidlist ::= nm collate sortorder */
    246, /* (134) eidlist ::= eidlist COMMA nm collate sortorder */
    250, /* (135) collate ::= */
    250, /* (136) collate ::= COLLATE ID|STRING */
    253, /* (137) with ::= */
    253, /* (138) with ::= WITH wqlist */
    253, /* (139) with ::= WITH RECURSIVE wqlist */
    193, /* (140) cmd ::= with DELETE FROM xfullname indexed_opt where_opt_ret
            orderby_opt limit_opt */
    193, /* (141) cmd ::= with UPDATE orconf xfullname indexed_opt SET setlist
            from where_opt_ret orderby_opt limit_opt */
    193, /* (142) cmd ::= with insert_cmd INTO xfullname idlist_opt select
            upsert */
    193, /* (143) cmd ::= with insert_cmd INTO xfullname idlist_opt DEFAULT
            VALUES returning */
    254, /* (144) insert_cmd ::= INSERT orconf */
    254, /* (145) insert_cmd ::= REPLACE */
    255, /* (146) orconf ::= */
    255, /* (147) orconf ::= OR resolvetype */
    247, /* (148) resolvetype ::= raisetype */
    247, /* (149) resolvetype ::= IGNORE */
    247, /* (150) resolvetype ::= REPLACE */
    257, /* (151) xfullname ::= nm */
    257, /* (152) xfullname ::= nm DOT nm */
    257, /* (153) xfullname ::= nm DOT nm AS nm */
    257, /* (154) xfullname ::= nm AS nm */
    256, /* (155) indexed_opt ::= */
    256, /* (156) indexed_opt ::= indexed_by */
    258, /* (157) where_opt_ret ::= */
    258, /* (158) where_opt_ret ::= WHERE expr */
    258, /* (159) where_opt_ret ::= RETURNING selcollist */
    258, /* (160) where_opt_ret ::= WHERE expr RETURNING selcollist */
    261, /* (161) setlist ::= setlist COMMA nm EQ expr */
    261, /* (162) setlist ::= setlist COMMA LP idlist RP EQ expr */
    261, /* (163) setlist ::= nm EQ expr */
    261, /* (164) setlist ::= LP idlist RP EQ expr */
    263, /* (165) idlist_opt ::= */
    263, /* (166) idlist_opt ::= LP idlist RP */
    264, /* (167) upsert ::= */
    264, /* (168) upsert ::= RETURNING selcollist */
    264, /* (169) upsert ::= ON CONFLICT LP sortlist RP where_opt DO UPDATE SET
            setlist where_opt upsert */
    264, /* (170) upsert ::= ON CONFLICT LP sortlist RP where_opt DO NOTHING
            upsert */
    264, /* (171) upsert ::= ON CONFLICT DO NOTHING returning */
    264, /* (172) upsert ::= ON CONFLICT DO UPDATE SET setlist where_opt
            returning */
    265, /* (173) returning ::= RETURNING selcollist */
    265, /* (174) returning ::= */
    194, /* (175) expr ::= term */
    194, /* (176) expr ::= LP expr RP */
    194, /* (177) expr ::= expr PLUS|MINUS expr */
    194, /* (178) expr ::= expr STAR|SLASH|REM expr */
    194, /* (179) expr ::= expr LT|GT|GE|LE expr */
    194, /* (180) expr ::= expr EQ|NE expr */
    194, /* (181) expr ::= expr AND expr */
    194, /* (182) expr ::= expr OR expr */
    194, /* (183) expr ::= expr BITAND|BITOR|LSHIFT|RSHIFT expr */
    194, /* (184) expr ::= expr CONCAT expr */
    194, /* (185) expr ::= expr PTR expr */
    194, /* (186) expr ::= PLUS|MINUS expr */
    194, /* (187) expr ::= BITNOT expr */
    194, /* (188) expr ::= NOT expr */
    196, /* (189) exprlist ::= nexprlist */
    196, /* (190) exprlist ::= */
    270, /* (191) nexprlist ::= nexprlist COMMA expr */
    270, /* (192) nexprlist ::= expr */
    194, /* (193) expr ::= LP nexprlist COMMA expr RP */
    194, /* (194) expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist RP */
    194, /* (195) expr ::= ID|INDEXED|JOIN_KW LP STAR RP */
    194, /* (196) expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist RP
            filter_over */
    194, /* (197) expr ::= ID|INDEXED|JOIN_KW LP STAR RP filter_over */
    205, /* (198) nm ::= ID|INDEXED|JOIN_KW */
    205, /* (199) nm ::= STRING */
    243, /* (200) term ::= INTEGER */
    243, /* (201) term ::= STRING */
    243, /* (202) term ::= NULL|FLOAT|BLOB */
    243, /* (203) term ::= QNUMBER */
    243, /* (204) term ::= CTIME_KW */
    194, /* (205) expr ::= VARIABLE */
    194, /* (206) expr ::= expr COLLATE ID|STRING */
    197, /* (207) sortlist ::= sortlist COMMA expr sortorder nulls */
    197, /* (208) sortlist ::= expr sortorder nulls */
    244, /* (209) sortorder ::= ASC */
    244, /* (210) sortorder ::= DESC */
    244, /* (211) sortorder ::= */
    271, /* (212) nulls ::= NULLS FIRST */
    271, /* (213) nulls ::= NULLS LAST */
    271, /* (214) nulls ::= */
    194, /* (215) expr ::= RAISE LP IGNORE RP */
    194, /* (216) expr ::= RAISE LP raisetype COMMA expr RP */
    266, /* (217) raisetype ::= ROLLBACK */
    266, /* (218) raisetype ::= ABORT */
    266, /* (219) raisetype ::= FAIL */
    277, /* (220) fullname ::= nm */
    277, /* (221) fullname ::= nm DOT nm */
    272, /* (222) ifexists ::= IF EXISTS */
    272, /* (223) ifexists ::= */
    193, /* (224) cmd ::= DROP TABLE ifexists fullname */
    193, /* (225) cmd ::= DROP VIEW ifexists fullname */
    193, /* (226) cmd ::= DROP INDEX ifexists fullname */
    193, /* (227) cmd ::= DROP TRIGGER ifexists fullname */
    193, /* (228) cmd ::= ALTER TABLE fullname RENAME TO nm */
    193, /* (229) cmd ::= ALTER TABLE fullname RENAME kwcolumn_opt nm TO nm */
    193, /* (230) cmd ::= ALTER TABLE fullname DROP kwcolumn_opt nm */
    193, /* (231) cmd ::= ALTER TABLE add_column_fullname ADD kwcolumn_opt
            columnname carglist */
    278, /* (232) add_column_fullname ::= fullname */
    276, /* (233) kwcolumn_opt ::= */
    276, /* (234) kwcolumn_opt ::= COLUMNKW */
    242, /* (235) columnname ::= nm typetoken */
    193, /* (236) cmd ::= BEGIN transtype trans_opt */
    193, /* (237) cmd ::= COMMIT|END trans_opt */
    193, /* (238) cmd ::= ROLLBACK trans_opt */
    273, /* (239) transtype ::= */
    273, /* (240) transtype ::= DEFERRED */
    273, /* (241) transtype ::= IMMEDIATE */
    273, /* (242) transtype ::= EXCLUSIVE */
    274, /* (243) trans_opt ::= */
    274, /* (244) trans_opt ::= TRANSACTION */
    274, /* (245) trans_opt ::= TRANSACTION nm */
    275, /* (246) savepoint_opt ::= SAVEPOINT */
    275, /* (247) savepoint_opt ::= */
    193, /* (248) cmd ::= SAVEPOINT nm */
    193, /* (249) cmd ::= RELEASE savepoint_opt nm */
    193, /* (250) cmd ::= ROLLBACK trans_opt TO savepoint_opt nm */
    193, /* (251) cmd ::= select */
    211, /* (252) select ::= selectnowith */
    209, /* (253) selectnowith ::= oneselect */
    210, /* (254) oneselect ::= SELECT distinct selcollist from where_opt
            groupby_opt having_opt orderby_opt limit_opt */
    210, /* (255) oneselect ::= SELECT distinct selcollist from where_opt
            groupby_opt having_opt window_clause orderby_opt limit_opt */
    202, /* (256) selcollist ::= sclp scanpt expr scanpt as */
    202, /* (257) selcollist ::= sclp scanpt STAR */
    203, /* (258) sclp ::= selcollist COMMA */
    203, /* (259) sclp ::= */
    204, /* (260) scanpt ::= */
    279, /* (261) as ::= AS nm */
    279, /* (262) as ::= ID|STRING */
    279, /* (263) as ::= */
    195, /* (264) distinct ::= DISTINCT */
    195, /* (265) distinct ::= ALL */
    195, /* (266) distinct ::= */
    262, /* (267) from ::= */
    262, /* (268) from ::= FROM seltablist */
    269, /* (269) where_opt ::= */
    269, /* (270) where_opt ::= WHERE expr */
    280, /* (271) groupby_opt ::= */
    280, /* (272) groupby_opt ::= GROUP BY nexprlist */
    281, /* (273) having_opt ::= */
    281, /* (274) having_opt ::= HAVING expr */
    259, /* (275) orderby_opt ::= */
    259, /* (276) orderby_opt ::= ORDER BY sortlist */
    260, /* (277) limit_opt ::= */
    260, /* (278) limit_opt ::= LIMIT expr */
    260, /* (279) limit_opt ::= LIMIT expr OFFSET expr */
    260, /* (280) limit_opt ::= LIMIT expr COMMA expr */
    286, /* (281) stl_prefix ::= seltablist joinop */
    286, /* (282) stl_prefix ::= */
    283, /* (283) seltablist ::= stl_prefix nm dbnm as on_using */
    283, /* (284) seltablist ::= stl_prefix nm dbnm as indexed_by on_using */
    283, /* (285) seltablist ::= stl_prefix nm dbnm LP exprlist RP as on_using
          */
    283, /* (286) seltablist ::= stl_prefix LP select RP as on_using */
    283, /* (287) seltablist ::= stl_prefix LP seltablist RP as on_using */
    285, /* (288) joinop ::= COMMA|JOIN */
    285, /* (289) joinop ::= JOIN_KW JOIN */
    285, /* (290) joinop ::= JOIN_KW nm JOIN */
    285, /* (291) joinop ::= JOIN_KW nm nm JOIN */
    284, /* (292) on_using ::= ON expr */
    284, /* (293) on_using ::= USING LP idlist RP */
    284, /* (294) on_using ::= */
    267, /* (295) indexed_by ::= INDEXED BY nm */
    267, /* (296) indexed_by ::= NOT INDEXED */
    268, /* (297) idlist ::= idlist COMMA nm */
    268, /* (298) idlist ::= nm */
    193, /* (299) cmd ::= createkw trigger_decl BEGIN trigger_cmd_list END */
    289, /* (300) trigger_decl ::= temp TRIGGER ifnotexists nm dbnm trigger_time
            trigger_event ON fullname foreach_clause when_clause */
    287, /* (301) trigger_time ::= BEFORE|AFTER */
    287, /* (302) trigger_time ::= INSTEAD OF */
    287, /* (303) trigger_time ::= */
    291, /* (304) trigger_event ::= DELETE|INSERT */
    291, /* (305) trigger_event ::= UPDATE */
    291, /* (306) trigger_event ::= UPDATE OF idlist */
    292, /* (307) foreach_clause ::= */
    292, /* (308) foreach_clause ::= FOR EACH ROW */
    293, /* (309) when_clause ::= */
    293, /* (310) when_clause ::= WHEN expr */
    290, /* (311) trigger_cmd_list ::= trigger_cmd_list trigger_cmd SEMI */
    290, /* (312) trigger_cmd_list ::= trigger_cmd SEMI */
    288, /* (313) trnm ::= nm */
    288, /* (314) trnm ::= nm DOT nm */
    295, /* (315) tridxby ::= */
    295, /* (316) tridxby ::= INDEXED BY nm */
    295, /* (317) tridxby ::= NOT INDEXED */
    294, /* (318) trigger_cmd ::= UPDATE orconf trnm tridxby SET setlist from
            where_opt scanpt */
    294, /* (319) trigger_cmd ::= scanpt insert_cmd INTO trnm idlist_opt select
            upsert scanpt */
    294, /* (320) trigger_cmd ::= DELETE FROM trnm tridxby where_opt scanpt */
    294, /* (321) trigger_cmd ::= scanpt select scanpt */
    193, /* (322) cmd ::= PRAGMA nm dbnm */
    193, /* (323) cmd ::= PRAGMA nm dbnm EQ nmnum */
    193, /* (324) cmd ::= PRAGMA nm dbnm LP nmnum RP */
    193, /* (325) cmd ::= PRAGMA nm dbnm EQ minus_num */
    193, /* (326) cmd ::= PRAGMA nm dbnm LP minus_num RP */
    298, /* (327) nmnum ::= plus_num */
    298, /* (328) nmnum ::= nm */
    298, /* (329) nmnum ::= ON */
    298, /* (330) nmnum ::= DELETE */
    298, /* (331) nmnum ::= DEFAULT */
    296, /* (332) plus_num ::= PLUS INTEGER|FLOAT */
    296, /* (333) plus_num ::= INTEGER|FLOAT */
    297, /* (334) minus_num ::= MINUS INTEGER|FLOAT */
    201, /* (335) signed ::= plus_num */
    201, /* (336) signed ::= minus_num */
    193, /* (337) cmd ::= ANALYZE */
    193, /* (338) cmd ::= ANALYZE nm dbnm */
    193, /* (339) cmd ::= REINDEX */
    193, /* (340) cmd ::= REINDEX nm dbnm */
    193, /* (341) cmd ::= ATTACH database_kw_opt expr AS expr key_opt */
    193, /* (342) cmd ::= DETACH database_kw_opt expr */
    301, /* (343) database_kw_opt ::= DATABASE */
    301, /* (344) database_kw_opt ::= */
    302, /* (345) key_opt ::= */
    302, /* (346) key_opt ::= KEY expr */
    193, /* (347) cmd ::= VACUUM vinto */
    193, /* (348) cmd ::= VACUUM nm vinto */
    303, /* (349) vinto ::= INTO expr */
    303, /* (350) vinto ::= */
    190, /* (351) ecmd ::= explain cmdx SEMI */
    300, /* (352) explain ::= EXPLAIN */
    300, /* (353) explain ::= EXPLAIN QUERY PLAN */
    193, /* (354) cmd ::= createkw uniqueflag INDEX ifnotexists nm dbnm ON nm LP
            sortlist RP where_opt */
    299, /* (355) uniqueflag ::= UNIQUE */
    299, /* (356) uniqueflag ::= */
    239, /* (357) ifnotexists ::= */
    239, /* (358) ifnotexists ::= IF NOT EXISTS */
    193, /* (359) cmd ::= createkw temp VIEW ifnotexists nm dbnm eidlist_opt AS
            select */
    237, /* (360) createkw ::= CREATE */
    238, /* (361) temp ::= TEMP */
    238, /* (362) temp ::= */
    304, /* (363) values ::= VALUES LP nexprlist RP */
    305, /* (364) mvalues ::= values COMMA LP nexprlist RP */
    305, /* (365) mvalues ::= mvalues COMMA LP nexprlist RP */
    210, /* (366) oneselect ::= values */
    210, /* (367) oneselect ::= mvalues */
    193, /* (368) cmd ::= create_vtab */
    193, /* (369) cmd ::= create_vtab LP vtabarglist RP */
    306, /* (370) create_vtab ::= createkw VIRTUAL TABLE ifnotexists nm dbnm
            USING nm */
    307, /* (371) vtabarglist ::= vtabarg */
    307, /* (372) vtabarglist ::= vtabarglist COMMA vtabarg */
    308, /* (373) vtabarg ::= */
    308, /* (374) vtabarg ::= vtabarg vtabargtoken */
    309, /* (375) vtabargtoken ::= ANY */
    309, /* (376) vtabargtoken ::= lp anylist RP */
    310, /* (377) lp ::= LP */
    311, /* (378) anylist ::= */
    311, /* (379) anylist ::= anylist LP anylist RP */
    311, /* (380) anylist ::= anylist ANY */
    315, /* (381) windowdefn_list ::= windowdefn */
    315, /* (382) windowdefn_list ::= windowdefn_list COMMA windowdefn */
    316, /* (383) windowdefn ::= nm AS LP window RP */
    317, /* (384) window ::= PARTITION BY nexprlist orderby_opt frame_opt */
    317, /* (385) window ::= nm PARTITION BY nexprlist orderby_opt frame_opt */
    317, /* (386) window ::= ORDER BY sortlist frame_opt */
    317, /* (387) window ::= nm ORDER BY sortlist frame_opt */
    317, /* (388) window ::= frame_opt */
    317, /* (389) window ::= nm frame_opt */
    318, /* (390) frame_opt ::= */
    318, /* (391) frame_opt ::= range_or_rows frame_bound_s frame_exclude_opt */
    318, /* (392) frame_opt ::= range_or_rows BETWEEN frame_bound_s AND
            frame_bound_e frame_exclude_opt */
    312, /* (393) range_or_rows ::= RANGE|ROWS|GROUPS */
    319, /* (394) frame_bound_s ::= frame_bound */
    319, /* (395) frame_bound_s ::= UNBOUNDED PRECEDING */
    320, /* (396) frame_bound_e ::= frame_bound */
    320, /* (397) frame_bound_e ::= UNBOUNDED FOLLOWING */
    321, /* (398) frame_bound ::= expr PRECEDING|FOLLOWING */
    321, /* (399) frame_bound ::= CURRENT ROW */
    313, /* (400) frame_exclude_opt ::= */
    313, /* (401) frame_exclude_opt ::= EXCLUDE frame_exclude */
    314, /* (402) frame_exclude ::= NO OTHERS */
    314, /* (403) frame_exclude ::= CURRENT ROW */
    314, /* (404) frame_exclude ::= GROUP|TIES */
    282, /* (405) window_clause ::= WINDOW windowdefn_list */
    198, /* (406) filter_over ::= filter_clause over_clause */
    198, /* (407) filter_over ::= over_clause */
    198, /* (408) filter_over ::= filter_clause */
    323, /* (409) over_clause ::= OVER LP window RP */
    323, /* (410) over_clause ::= OVER nm */
    322, /* (411) filter_clause ::= FILTER LP WHERE expr RP */
};

/* For rule J, yyRuleInfoNRhs[J] contains the negative of the number
** of symbols on the right-hand side of that rule. */
static const signed char yyRuleInfoNRhs[] = {
    -1,  /* (0) input ::= cmdlist */
    -2,  /* (1) cmdlist ::= cmdlist ecmd */
    -1,  /* (2) cmdlist ::= ecmd */
    -1,  /* (3) ecmd ::= SEMI */
    -2,  /* (4) ecmd ::= cmdx SEMI */
    -2,  /* (5) ecmd ::= error SEMI */
    -1,  /* (6) cmdx ::= cmd */
    -8,  /* (7) expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist ORDER BY
            sortlist RP */
    -9,  /* (8) expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist ORDER BY
            sortlist RP filter_over */
    -12, /* (9) expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist RP WITHIN GROUP
            LP ORDER BY expr RP */
    -13, /* (10) expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist RP WITHIN
            GROUP LP ORDER BY expr RP filter_over */
    -6,  /* (11) expr ::= CAST LP expr AS typetoken RP */
    0,   /* (12) typetoken ::= */
    -1,  /* (13) typetoken ::= typename */
    -4,  /* (14) typetoken ::= typename LP signed RP */
    -6,  /* (15) typetoken ::= typename LP signed COMMA signed RP */
    -1,  /* (16) typename ::= ID|STRING */
    -2,  /* (17) typename ::= typename ID|STRING */
    -5,  /* (18) selcollist ::= sclp scanpt nm DOT STAR */
    -1,  /* (19) expr ::= ID|INDEXED|JOIN_KW */
    -3,  /* (20) expr ::= nm DOT nm */
    -5,  /* (21) expr ::= nm DOT nm DOT nm */
    -3,  /* (22) selectnowith ::= selectnowith multiselect_op oneselect */
    -1,  /* (23) multiselect_op ::= UNION */
    -2,  /* (24) multiselect_op ::= UNION ALL */
    -1,  /* (25) multiselect_op ::= EXCEPT|INTERSECT */
    -3,  /* (26) expr ::= LP select RP */
    -4,  /* (27) expr ::= EXISTS LP select RP */
    -1,  /* (28) in_op ::= IN */
    -2,  /* (29) in_op ::= NOT IN */
    -5,  /* (30) expr ::= expr in_op LP exprlist RP */
    -5,  /* (31) expr ::= expr in_op LP select RP */
    -5,  /* (32) expr ::= expr in_op nm dbnm paren_exprlist */
    0,   /* (33) dbnm ::= */
    -2,  /* (34) dbnm ::= DOT nm */
    0,   /* (35) paren_exprlist ::= */
    -3,  /* (36) paren_exprlist ::= LP exprlist RP */
    -2,  /* (37) expr ::= expr ISNULL|NOTNULL */
    -3,  /* (38) expr ::= expr NOT NULL */
    -3,  /* (39) expr ::= expr IS expr */
    -4,  /* (40) expr ::= expr IS NOT expr */
    -6,  /* (41) expr ::= expr IS NOT DISTINCT FROM expr */
    -5,  /* (42) expr ::= expr IS DISTINCT FROM expr */
    -1,  /* (43) between_op ::= BETWEEN */
    -2,  /* (44) between_op ::= NOT BETWEEN */
    -5,  /* (45) expr ::= expr between_op expr AND expr */
    -1,  /* (46) likeop ::= LIKE_KW|MATCH */
    -2,  /* (47) likeop ::= NOT LIKE_KW|MATCH */
    -3,  /* (48) expr ::= expr likeop expr */
    -5,  /* (49) expr ::= expr likeop expr ESCAPE expr */
    -5,  /* (50) expr ::= CASE case_operand case_exprlist case_else END */
    -5,  /* (51) case_exprlist ::= case_exprlist WHEN expr THEN expr */
    -4,  /* (52) case_exprlist ::= WHEN expr THEN expr */
    -2,  /* (53) case_else ::= ELSE expr */
    0,   /* (54) case_else ::= */
    -1,  /* (55) case_operand ::= expr */
    0,   /* (56) case_operand ::= */
    -2,  /* (57) cmd ::= create_table create_table_args */
    -6,  /* (58) create_table ::= createkw temp TABLE ifnotexists nm dbnm */
    -5,  /* (59) create_table_args ::= LP columnlist conslist_opt RP
            table_option_set */
    -2,  /* (60) create_table_args ::= AS select */
    0,   /* (61) table_option_set ::= */
    -1,  /* (62) table_option_set ::= table_option */
    -3,  /* (63) table_option_set ::= table_option_set COMMA table_option */
    -2,  /* (64) table_option ::= WITHOUT nm */
    -1,  /* (65) table_option ::= nm */
    -4,  /* (66) columnlist ::= columnlist COMMA columnname carglist */
    -2,  /* (67) columnlist ::= columnname carglist */
    -2,  /* (68) carglist ::= carglist ccons */
    0,   /* (69) carglist ::= */
    -2,  /* (70) ccons ::= CONSTRAINT nm */
    -3,  /* (71) ccons ::= DEFAULT scantok term */
    -4,  /* (72) ccons ::= DEFAULT LP expr RP */
    -4,  /* (73) ccons ::= DEFAULT PLUS scantok term */
    -4,  /* (74) ccons ::= DEFAULT MINUS scantok term */
    -3,  /* (75) ccons ::= DEFAULT scantok ID|INDEXED */
    -2,  /* (76) ccons ::= NULL onconf */
    -3,  /* (77) ccons ::= NOT NULL onconf */
    -5,  /* (78) ccons ::= PRIMARY KEY sortorder onconf autoinc */
    -2,  /* (79) ccons ::= UNIQUE onconf */
    -4,  /* (80) ccons ::= CHECK LP expr RP */
    -4,  /* (81) ccons ::= REFERENCES nm eidlist_opt refargs */
    -1,  /* (82) ccons ::= defer_subclause */
    -2,  /* (83) ccons ::= COLLATE ID|STRING */
    -4,  /* (84) ccons ::= GENERATED ALWAYS AS generated */
    -2,  /* (85) ccons ::= AS generated */
    -3,  /* (86) generated ::= LP expr RP */
    -4,  /* (87) generated ::= LP expr RP ID */
    0,   /* (88) autoinc ::= */
    -1,  /* (89) autoinc ::= AUTOINCR */
    0,   /* (90) refargs ::= */
    -2,  /* (91) refargs ::= refargs refarg */
    -2,  /* (92) refarg ::= MATCH nm */
    -3,  /* (93) refarg ::= ON INSERT refact */
    -3,  /* (94) refarg ::= ON DELETE refact */
    -3,  /* (95) refarg ::= ON UPDATE refact */
    -2,  /* (96) refact ::= SET NULL */
    -2,  /* (97) refact ::= SET DEFAULT */
    -1,  /* (98) refact ::= CASCADE */
    -1,  /* (99) refact ::= RESTRICT */
    -2,  /* (100) refact ::= NO ACTION */
    -3,  /* (101) defer_subclause ::= NOT DEFERRABLE init_deferred_pred_opt */
    -2,  /* (102) defer_subclause ::= DEFERRABLE init_deferred_pred_opt */
    0,   /* (103) init_deferred_pred_opt ::= */
    -2,  /* (104) init_deferred_pred_opt ::= INITIALLY DEFERRED */
    -2,  /* (105) init_deferred_pred_opt ::= INITIALLY IMMEDIATE */
    0,   /* (106) conslist_opt ::= */
    -2,  /* (107) conslist_opt ::= COMMA conslist */
    -3,  /* (108) conslist ::= conslist tconscomma tcons */
    -1,  /* (109) conslist ::= tcons */
    -1,  /* (110) tconscomma ::= COMMA */
    0,   /* (111) tconscomma ::= */
    -2,  /* (112) tcons ::= CONSTRAINT nm */
    -7,  /* (113) tcons ::= PRIMARY KEY LP sortlist autoinc RP onconf */
    -5,  /* (114) tcons ::= UNIQUE LP sortlist RP onconf */
    -5,  /* (115) tcons ::= CHECK LP expr RP onconf */
    -10, /* (116) tcons ::= FOREIGN KEY LP eidlist RP REFERENCES nm eidlist_opt
            refargs defer_subclause_opt */
    0,   /* (117) defer_subclause_opt ::= */
    -1,  /* (118) defer_subclause_opt ::= defer_subclause */
    0,   /* (119) onconf ::= */
    -3,  /* (120) onconf ::= ON CONFLICT resolvetype */
    0,   /* (121) scantok ::= */
    -3,  /* (122) select ::= WITH wqlist selectnowith */
    -4,  /* (123) select ::= WITH RECURSIVE wqlist selectnowith */
    -6,  /* (124) wqitem ::= withnm eidlist_opt wqas LP select RP */
    -1,  /* (125) wqlist ::= wqitem */
    -3,  /* (126) wqlist ::= wqlist COMMA wqitem */
    -1,  /* (127) withnm ::= nm */
    -1,  /* (128) wqas ::= AS */
    -2,  /* (129) wqas ::= AS MATERIALIZED */
    -3,  /* (130) wqas ::= AS NOT MATERIALIZED */
    0,   /* (131) eidlist_opt ::= */
    -3,  /* (132) eidlist_opt ::= LP eidlist RP */
    -3,  /* (133) eidlist ::= nm collate sortorder */
    -5,  /* (134) eidlist ::= eidlist COMMA nm collate sortorder */
    0,   /* (135) collate ::= */
    -2,  /* (136) collate ::= COLLATE ID|STRING */
    0,   /* (137) with ::= */
    -2,  /* (138) with ::= WITH wqlist */
    -3,  /* (139) with ::= WITH RECURSIVE wqlist */
    -8,  /* (140) cmd ::= with DELETE FROM xfullname indexed_opt where_opt_ret
            orderby_opt limit_opt */
    -11, /* (141) cmd ::= with UPDATE orconf xfullname indexed_opt SET setlist
            from where_opt_ret orderby_opt limit_opt */
    -7, /* (142) cmd ::= with insert_cmd INTO xfullname idlist_opt select upsert
         */
    -8, /* (143) cmd ::= with insert_cmd INTO xfullname idlist_opt DEFAULT
           VALUES returning */
    -2, /* (144) insert_cmd ::= INSERT orconf */
    -1, /* (145) insert_cmd ::= REPLACE */
    0,  /* (146) orconf ::= */
    -2, /* (147) orconf ::= OR resolvetype */
    -1, /* (148) resolvetype ::= raisetype */
    -1, /* (149) resolvetype ::= IGNORE */
    -1, /* (150) resolvetype ::= REPLACE */
    -1, /* (151) xfullname ::= nm */
    -3, /* (152) xfullname ::= nm DOT nm */
    -5, /* (153) xfullname ::= nm DOT nm AS nm */
    -3, /* (154) xfullname ::= nm AS nm */
    0,  /* (155) indexed_opt ::= */
    -1, /* (156) indexed_opt ::= indexed_by */
    0,  /* (157) where_opt_ret ::= */
    -2, /* (158) where_opt_ret ::= WHERE expr */
    -2, /* (159) where_opt_ret ::= RETURNING selcollist */
    -4, /* (160) where_opt_ret ::= WHERE expr RETURNING selcollist */
    -5, /* (161) setlist ::= setlist COMMA nm EQ expr */
    -7, /* (162) setlist ::= setlist COMMA LP idlist RP EQ expr */
    -3, /* (163) setlist ::= nm EQ expr */
    -5, /* (164) setlist ::= LP idlist RP EQ expr */
    0,  /* (165) idlist_opt ::= */
    -3, /* (166) idlist_opt ::= LP idlist RP */
    0,  /* (167) upsert ::= */
    -2, /* (168) upsert ::= RETURNING selcollist */
    -12, /* (169) upsert ::= ON CONFLICT LP sortlist RP where_opt DO UPDATE SET
            setlist where_opt upsert */
    -9,  /* (170) upsert ::= ON CONFLICT LP sortlist RP where_opt DO NOTHING
            upsert */
    -5,  /* (171) upsert ::= ON CONFLICT DO NOTHING returning */
    -8,  /* (172) upsert ::= ON CONFLICT DO UPDATE SET setlist where_opt
            returning */
    -2,  /* (173) returning ::= RETURNING selcollist */
    0,   /* (174) returning ::= */
    -1,  /* (175) expr ::= term */
    -3,  /* (176) expr ::= LP expr RP */
    -3,  /* (177) expr ::= expr PLUS|MINUS expr */
    -3,  /* (178) expr ::= expr STAR|SLASH|REM expr */
    -3,  /* (179) expr ::= expr LT|GT|GE|LE expr */
    -3,  /* (180) expr ::= expr EQ|NE expr */
    -3,  /* (181) expr ::= expr AND expr */
    -3,  /* (182) expr ::= expr OR expr */
    -3,  /* (183) expr ::= expr BITAND|BITOR|LSHIFT|RSHIFT expr */
    -3,  /* (184) expr ::= expr CONCAT expr */
    -3,  /* (185) expr ::= expr PTR expr */
    -2,  /* (186) expr ::= PLUS|MINUS expr */
    -2,  /* (187) expr ::= BITNOT expr */
    -2,  /* (188) expr ::= NOT expr */
    -1,  /* (189) exprlist ::= nexprlist */
    0,   /* (190) exprlist ::= */
    -3,  /* (191) nexprlist ::= nexprlist COMMA expr */
    -1,  /* (192) nexprlist ::= expr */
    -5,  /* (193) expr ::= LP nexprlist COMMA expr RP */
    -5,  /* (194) expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist RP */
    -4,  /* (195) expr ::= ID|INDEXED|JOIN_KW LP STAR RP */
    -6, /* (196) expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist RP filter_over
         */
    -5, /* (197) expr ::= ID|INDEXED|JOIN_KW LP STAR RP filter_over */
    -1, /* (198) nm ::= ID|INDEXED|JOIN_KW */
    -1, /* (199) nm ::= STRING */
    -1, /* (200) term ::= INTEGER */
    -1, /* (201) term ::= STRING */
    -1, /* (202) term ::= NULL|FLOAT|BLOB */
    -1, /* (203) term ::= QNUMBER */
    -1, /* (204) term ::= CTIME_KW */
    -1, /* (205) expr ::= VARIABLE */
    -3, /* (206) expr ::= expr COLLATE ID|STRING */
    -5, /* (207) sortlist ::= sortlist COMMA expr sortorder nulls */
    -3, /* (208) sortlist ::= expr sortorder nulls */
    -1, /* (209) sortorder ::= ASC */
    -1, /* (210) sortorder ::= DESC */
    0,  /* (211) sortorder ::= */
    -2, /* (212) nulls ::= NULLS FIRST */
    -2, /* (213) nulls ::= NULLS LAST */
    0,  /* (214) nulls ::= */
    -4, /* (215) expr ::= RAISE LP IGNORE RP */
    -6, /* (216) expr ::= RAISE LP raisetype COMMA expr RP */
    -1, /* (217) raisetype ::= ROLLBACK */
    -1, /* (218) raisetype ::= ABORT */
    -1, /* (219) raisetype ::= FAIL */
    -1, /* (220) fullname ::= nm */
    -3, /* (221) fullname ::= nm DOT nm */
    -2, /* (222) ifexists ::= IF EXISTS */
    0,  /* (223) ifexists ::= */
    -4, /* (224) cmd ::= DROP TABLE ifexists fullname */
    -4, /* (225) cmd ::= DROP VIEW ifexists fullname */
    -4, /* (226) cmd ::= DROP INDEX ifexists fullname */
    -4, /* (227) cmd ::= DROP TRIGGER ifexists fullname */
    -6, /* (228) cmd ::= ALTER TABLE fullname RENAME TO nm */
    -8, /* (229) cmd ::= ALTER TABLE fullname RENAME kwcolumn_opt nm TO nm */
    -6, /* (230) cmd ::= ALTER TABLE fullname DROP kwcolumn_opt nm */
    -7, /* (231) cmd ::= ALTER TABLE add_column_fullname ADD kwcolumn_opt
           columnname carglist */
    -1, /* (232) add_column_fullname ::= fullname */
    0,  /* (233) kwcolumn_opt ::= */
    -1, /* (234) kwcolumn_opt ::= COLUMNKW */
    -2, /* (235) columnname ::= nm typetoken */
    -3, /* (236) cmd ::= BEGIN transtype trans_opt */
    -2, /* (237) cmd ::= COMMIT|END trans_opt */
    -2, /* (238) cmd ::= ROLLBACK trans_opt */
    0,  /* (239) transtype ::= */
    -1, /* (240) transtype ::= DEFERRED */
    -1, /* (241) transtype ::= IMMEDIATE */
    -1, /* (242) transtype ::= EXCLUSIVE */
    0,  /* (243) trans_opt ::= */
    -1, /* (244) trans_opt ::= TRANSACTION */
    -2, /* (245) trans_opt ::= TRANSACTION nm */
    -1, /* (246) savepoint_opt ::= SAVEPOINT */
    0,  /* (247) savepoint_opt ::= */
    -2, /* (248) cmd ::= SAVEPOINT nm */
    -3, /* (249) cmd ::= RELEASE savepoint_opt nm */
    -5, /* (250) cmd ::= ROLLBACK trans_opt TO savepoint_opt nm */
    -1, /* (251) cmd ::= select */
    -1, /* (252) select ::= selectnowith */
    -1, /* (253) selectnowith ::= oneselect */
    -9, /* (254) oneselect ::= SELECT distinct selcollist from where_opt
           groupby_opt having_opt orderby_opt limit_opt */
    -10, /* (255) oneselect ::= SELECT distinct selcollist from where_opt
            groupby_opt having_opt window_clause orderby_opt limit_opt */
    -5,  /* (256) selcollist ::= sclp scanpt expr scanpt as */
    -3,  /* (257) selcollist ::= sclp scanpt STAR */
    -2,  /* (258) sclp ::= selcollist COMMA */
    0,   /* (259) sclp ::= */
    0,   /* (260) scanpt ::= */
    -2,  /* (261) as ::= AS nm */
    -1,  /* (262) as ::= ID|STRING */
    0,   /* (263) as ::= */
    -1,  /* (264) distinct ::= DISTINCT */
    -1,  /* (265) distinct ::= ALL */
    0,   /* (266) distinct ::= */
    0,   /* (267) from ::= */
    -2,  /* (268) from ::= FROM seltablist */
    0,   /* (269) where_opt ::= */
    -2,  /* (270) where_opt ::= WHERE expr */
    0,   /* (271) groupby_opt ::= */
    -3,  /* (272) groupby_opt ::= GROUP BY nexprlist */
    0,   /* (273) having_opt ::= */
    -2,  /* (274) having_opt ::= HAVING expr */
    0,   /* (275) orderby_opt ::= */
    -3,  /* (276) orderby_opt ::= ORDER BY sortlist */
    0,   /* (277) limit_opt ::= */
    -2,  /* (278) limit_opt ::= LIMIT expr */
    -4,  /* (279) limit_opt ::= LIMIT expr OFFSET expr */
    -4,  /* (280) limit_opt ::= LIMIT expr COMMA expr */
    -2,  /* (281) stl_prefix ::= seltablist joinop */
    0,   /* (282) stl_prefix ::= */
    -5,  /* (283) seltablist ::= stl_prefix nm dbnm as on_using */
    -6,  /* (284) seltablist ::= stl_prefix nm dbnm as indexed_by on_using */
    -8, /* (285) seltablist ::= stl_prefix nm dbnm LP exprlist RP as on_using */
    -6, /* (286) seltablist ::= stl_prefix LP select RP as on_using */
    -6, /* (287) seltablist ::= stl_prefix LP seltablist RP as on_using */
    -1, /* (288) joinop ::= COMMA|JOIN */
    -2, /* (289) joinop ::= JOIN_KW JOIN */
    -3, /* (290) joinop ::= JOIN_KW nm JOIN */
    -4, /* (291) joinop ::= JOIN_KW nm nm JOIN */
    -2, /* (292) on_using ::= ON expr */
    -4, /* (293) on_using ::= USING LP idlist RP */
    0,  /* (294) on_using ::= */
    -3, /* (295) indexed_by ::= INDEXED BY nm */
    -2, /* (296) indexed_by ::= NOT INDEXED */
    -3, /* (297) idlist ::= idlist COMMA nm */
    -1, /* (298) idlist ::= nm */
    -5, /* (299) cmd ::= createkw trigger_decl BEGIN trigger_cmd_list END */
    -11, /* (300) trigger_decl ::= temp TRIGGER ifnotexists nm dbnm trigger_time
            trigger_event ON fullname foreach_clause when_clause */
    -1,  /* (301) trigger_time ::= BEFORE|AFTER */
    -2,  /* (302) trigger_time ::= INSTEAD OF */
    0,   /* (303) trigger_time ::= */
    -1,  /* (304) trigger_event ::= DELETE|INSERT */
    -1,  /* (305) trigger_event ::= UPDATE */
    -3,  /* (306) trigger_event ::= UPDATE OF idlist */
    0,   /* (307) foreach_clause ::= */
    -3,  /* (308) foreach_clause ::= FOR EACH ROW */
    0,   /* (309) when_clause ::= */
    -2,  /* (310) when_clause ::= WHEN expr */
    -3,  /* (311) trigger_cmd_list ::= trigger_cmd_list trigger_cmd SEMI */
    -2,  /* (312) trigger_cmd_list ::= trigger_cmd SEMI */
    -1,  /* (313) trnm ::= nm */
    -3,  /* (314) trnm ::= nm DOT nm */
    0,   /* (315) tridxby ::= */
    -3,  /* (316) tridxby ::= INDEXED BY nm */
    -2,  /* (317) tridxby ::= NOT INDEXED */
    -9,  /* (318) trigger_cmd ::= UPDATE orconf trnm tridxby SET setlist from
            where_opt scanpt */
    -8,  /* (319) trigger_cmd ::= scanpt insert_cmd INTO trnm idlist_opt select
            upsert scanpt */
    -6,  /* (320) trigger_cmd ::= DELETE FROM trnm tridxby where_opt scanpt */
    -3,  /* (321) trigger_cmd ::= scanpt select scanpt */
    -3,  /* (322) cmd ::= PRAGMA nm dbnm */
    -5,  /* (323) cmd ::= PRAGMA nm dbnm EQ nmnum */
    -6,  /* (324) cmd ::= PRAGMA nm dbnm LP nmnum RP */
    -5,  /* (325) cmd ::= PRAGMA nm dbnm EQ minus_num */
    -6,  /* (326) cmd ::= PRAGMA nm dbnm LP minus_num RP */
    -1,  /* (327) nmnum ::= plus_num */
    -1,  /* (328) nmnum ::= nm */
    -1,  /* (329) nmnum ::= ON */
    -1,  /* (330) nmnum ::= DELETE */
    -1,  /* (331) nmnum ::= DEFAULT */
    -2,  /* (332) plus_num ::= PLUS INTEGER|FLOAT */
    -1,  /* (333) plus_num ::= INTEGER|FLOAT */
    -2,  /* (334) minus_num ::= MINUS INTEGER|FLOAT */
    -1,  /* (335) signed ::= plus_num */
    -1,  /* (336) signed ::= minus_num */
    -1,  /* (337) cmd ::= ANALYZE */
    -3,  /* (338) cmd ::= ANALYZE nm dbnm */
    -1,  /* (339) cmd ::= REINDEX */
    -3,  /* (340) cmd ::= REINDEX nm dbnm */
    -6,  /* (341) cmd ::= ATTACH database_kw_opt expr AS expr key_opt */
    -3,  /* (342) cmd ::= DETACH database_kw_opt expr */
    -1,  /* (343) database_kw_opt ::= DATABASE */
    0,   /* (344) database_kw_opt ::= */
    0,   /* (345) key_opt ::= */
    -2,  /* (346) key_opt ::= KEY expr */
    -2,  /* (347) cmd ::= VACUUM vinto */
    -3,  /* (348) cmd ::= VACUUM nm vinto */
    -2,  /* (349) vinto ::= INTO expr */
    0,   /* (350) vinto ::= */
    -3,  /* (351) ecmd ::= explain cmdx SEMI */
    -1,  /* (352) explain ::= EXPLAIN */
    -3,  /* (353) explain ::= EXPLAIN QUERY PLAN */
    -12, /* (354) cmd ::= createkw uniqueflag INDEX ifnotexists nm dbnm ON nm LP
            sortlist RP where_opt */
    -1,  /* (355) uniqueflag ::= UNIQUE */
    0,   /* (356) uniqueflag ::= */
    0,   /* (357) ifnotexists ::= */
    -3,  /* (358) ifnotexists ::= IF NOT EXISTS */
    -9,  /* (359) cmd ::= createkw temp VIEW ifnotexists nm dbnm eidlist_opt AS
            select */
    -1,  /* (360) createkw ::= CREATE */
    -1,  /* (361) temp ::= TEMP */
    0,   /* (362) temp ::= */
    -4,  /* (363) values ::= VALUES LP nexprlist RP */
    -5,  /* (364) mvalues ::= values COMMA LP nexprlist RP */
    -5,  /* (365) mvalues ::= mvalues COMMA LP nexprlist RP */
    -1,  /* (366) oneselect ::= values */
    -1,  /* (367) oneselect ::= mvalues */
    -1,  /* (368) cmd ::= create_vtab */
    -4,  /* (369) cmd ::= create_vtab LP vtabarglist RP */
    -8,  /* (370) create_vtab ::= createkw VIRTUAL TABLE ifnotexists nm dbnm
            USING nm */
    -1,  /* (371) vtabarglist ::= vtabarg */
    -3,  /* (372) vtabarglist ::= vtabarglist COMMA vtabarg */
    0,   /* (373) vtabarg ::= */
    -2,  /* (374) vtabarg ::= vtabarg vtabargtoken */
    -1,  /* (375) vtabargtoken ::= ANY */
    -3,  /* (376) vtabargtoken ::= lp anylist RP */
    -1,  /* (377) lp ::= LP */
    0,   /* (378) anylist ::= */
    -4,  /* (379) anylist ::= anylist LP anylist RP */
    -2,  /* (380) anylist ::= anylist ANY */
    -1,  /* (381) windowdefn_list ::= windowdefn */
    -3,  /* (382) windowdefn_list ::= windowdefn_list COMMA windowdefn */
    -5,  /* (383) windowdefn ::= nm AS LP window RP */
    -5,  /* (384) window ::= PARTITION BY nexprlist orderby_opt frame_opt */
    -6,  /* (385) window ::= nm PARTITION BY nexprlist orderby_opt frame_opt */
    -4,  /* (386) window ::= ORDER BY sortlist frame_opt */
    -5,  /* (387) window ::= nm ORDER BY sortlist frame_opt */
    -1,  /* (388) window ::= frame_opt */
    -2,  /* (389) window ::= nm frame_opt */
    0,   /* (390) frame_opt ::= */
    -3,  /* (391) frame_opt ::= range_or_rows frame_bound_s frame_exclude_opt */
    -6,  /* (392) frame_opt ::= range_or_rows BETWEEN frame_bound_s AND
            frame_bound_e frame_exclude_opt */
    -1,  /* (393) range_or_rows ::= RANGE|ROWS|GROUPS */
    -1,  /* (394) frame_bound_s ::= frame_bound */
    -2,  /* (395) frame_bound_s ::= UNBOUNDED PRECEDING */
    -1,  /* (396) frame_bound_e ::= frame_bound */
    -2,  /* (397) frame_bound_e ::= UNBOUNDED FOLLOWING */
    -2,  /* (398) frame_bound ::= expr PRECEDING|FOLLOWING */
    -2,  /* (399) frame_bound ::= CURRENT ROW */
    0,   /* (400) frame_exclude_opt ::= */
    -2,  /* (401) frame_exclude_opt ::= EXCLUDE frame_exclude */
    -2,  /* (402) frame_exclude ::= NO OTHERS */
    -2,  /* (403) frame_exclude ::= CURRENT ROW */
    -1,  /* (404) frame_exclude ::= GROUP|TIES */
    -2,  /* (405) window_clause ::= WINDOW windowdefn_list */
    -2,  /* (406) filter_over ::= filter_clause over_clause */
    -1,  /* (407) filter_over ::= over_clause */
    -1,  /* (408) filter_over ::= filter_clause */
    -4,  /* (409) over_clause ::= OVER LP window RP */
    -2,  /* (410) over_clause ::= OVER nm */
    -5,  /* (411) filter_clause ::= FILTER LP WHERE expr RP */
};

static void yy_accept(yyParser*); /* Forward Declaration */

/*
** Perform a reduce action and the shift that must immediately
** follow the reduce.
**
** The yyLookahead and yyLookaheadToken parameters provide reduce actions
** access to the lookahead token (if any).  The yyLookahead will be YYNOCODE
** if the lookahead token has already been consumed.  As this procedure is
** only called from one place, optimizing compilers will in-line it, which
** means that the extra parameters have no performance impact.
*/
static YYACTIONTYPE yy_reduce(
    yyParser* yypParser,   /* The parser */
    unsigned int yyruleno, /* Number of the rule by which to reduce */
    int yyLookahead,       /* Lookahead token, or YYNOCODE if none */
    SynqSqliteParseTOKENTYPE yyLookaheadToken /* Value of the lookahead token */
        SynqSqliteParseCTX_PDECL              /* %extra_context */
) {
  int yygoto;          /* The next state */
  YYACTIONTYPE yyact;  /* The next action */
  yyStackEntry* yymsp; /* The top of the parser's stack */
  int yysize;          /* Amount to pop the stack */
  SynqSqliteParseARG_FETCH(void) yyLookahead;
  (void)yyLookaheadToken;
  yymsp = yypParser->yytos;

  switch (yyruleno) {
    /* Beginning here are the reduction cases.  A typical example
    ** follows:
    **   case 0:
    **  #line <lineno> <grammarfile>
    **     { ... }           // User supplied code
    **  #line <lineno> <thisfile>
    **     break;
    */
    /********** Begin reduce actions
     * **********************************************/
    YYMINORTYPE yylhsminor;
    case 0: /* input ::= cmdlist */
    {
      pCtx->root = yymsp[0].minor.yy213;
    } break;
    case 1: /* cmdlist ::= cmdlist ecmd */
    {
      yymsp[-1].minor.yy213 =
          yymsp[0].minor.yy213;  // Just use the last command for now
    } break;
    case 2: /* cmdlist ::= ecmd */
    case 6: /* cmdx ::= cmd */
      yytestcase(yyruleno == 6);
    case 55: /* case_operand ::= expr */
      yytestcase(yyruleno == 55);
    case 175: /* expr ::= term */
      yytestcase(yyruleno == 175);
    case 189: /* exprlist ::= nexprlist */
      yytestcase(yyruleno == 189);
    case 251: /* cmd ::= select */
      yytestcase(yyruleno == 251);
    case 252: /* select ::= selectnowith */
      yytestcase(yyruleno == 252);
    case 253: /* selectnowith ::= oneselect */
      yytestcase(yyruleno == 253);
    case 368: /* cmd ::= create_vtab */
      yytestcase(yyruleno == 368);
    case 394: /* frame_bound_s ::= frame_bound */
      yytestcase(yyruleno == 394);
    case 396: /* frame_bound_e ::= frame_bound */
      yytestcase(yyruleno == 396);
    case 407: /* filter_over ::= over_clause */
      yytestcase(yyruleno == 407);
      {
        yylhsminor.yy213 = yymsp[0].minor.yy213;
      }
      yymsp[0].minor.yy213 = yylhsminor.yy213;
      break;
    case 3: /* ecmd ::= SEMI */
    {
      yymsp[0].minor.yy213 = SYNTAQLITE_NULL_NODE;
      pCtx->stmt_completed = 1;
    } break;
    case 4: /* ecmd ::= cmdx SEMI */
    {
      yylhsminor.yy213 = yymsp[-1].minor.yy213;
      pCtx->root = yymsp[-1].minor.yy213;
      synq_parse_list_flush(pCtx);
      pCtx->stmt_completed = 1;
    }
      yymsp[-1].minor.yy213 = yylhsminor.yy213;
      break;
    case 5: /* ecmd ::= error SEMI */
    {
      yymsp[-1].minor.yy213 = SYNTAQLITE_NULL_NODE;
      pCtx->root = SYNTAQLITE_NULL_NODE;
      pCtx->stmt_completed = 1;
    } break;
    case 7: /* expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist ORDER BY
               sortlist RP */
    {
      synq_mark_as_function(pCtx, yymsp[-7].minor.yy0);
      yylhsminor.yy213 = synq_parse_aggregate_function_call(
          pCtx, synq_span(pCtx, yymsp[-7].minor.yy0),
          (SyntaqliteAggregateFunctionCallFlags){
              .raw = (uint8_t)yymsp[-5].minor.yy213},
          yymsp[-4].minor.yy213, yymsp[-1].minor.yy213, SYNTAQLITE_NULL_NODE,
          SYNTAQLITE_NULL_NODE);
    }
      yymsp[-7].minor.yy213 = yylhsminor.yy213;
      break;
    case 8: /* expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist ORDER BY
               sortlist RP filter_over */
    {
      SyntaqliteFilterOver* fo = (SyntaqliteFilterOver*)synq_arena_ptr(
          &pCtx->ast, yymsp[0].minor.yy213);
      synq_mark_as_function(pCtx, yymsp[-8].minor.yy0);
      yylhsminor.yy213 = synq_parse_aggregate_function_call(
          pCtx, synq_span(pCtx, yymsp[-8].minor.yy0),
          (SyntaqliteAggregateFunctionCallFlags){
              .raw = (uint8_t)yymsp[-6].minor.yy213},
          yymsp[-5].minor.yy213, yymsp[-2].minor.yy213, fo->filter_expr,
          fo->over_def);
    }
      yymsp[-8].minor.yy213 = yylhsminor.yy213;
      break;
    case 9: /* expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist RP WITHIN GROUP
               LP ORDER BY expr RP */
    {
      synq_mark_as_function(pCtx, yymsp[-11].minor.yy0);
      yylhsminor.yy213 = synq_parse_ordered_set_function_call(
          pCtx, synq_span(pCtx, yymsp[-11].minor.yy0),
          (SyntaqliteAggregateFunctionCallFlags){
              .raw = (uint8_t)yymsp[-9].minor.yy213},
          yymsp[-8].minor.yy213, yymsp[-1].minor.yy213, SYNTAQLITE_NULL_NODE,
          SYNTAQLITE_NULL_NODE);
    }
      yymsp[-11].minor.yy213 = yylhsminor.yy213;
      break;
    case 10: /* expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist RP WITHIN GROUP
                LP ORDER BY expr RP filter_over */
    {
      SyntaqliteFilterOver* fo = (SyntaqliteFilterOver*)synq_arena_ptr(
          &pCtx->ast, yymsp[0].minor.yy213);
      synq_mark_as_function(pCtx, yymsp[-12].minor.yy0);
      yylhsminor.yy213 = synq_parse_ordered_set_function_call(
          pCtx, synq_span(pCtx, yymsp[-12].minor.yy0),
          (SyntaqliteAggregateFunctionCallFlags){
              .raw = (uint8_t)yymsp[-10].minor.yy213},
          yymsp[-9].minor.yy213, yymsp[-2].minor.yy213, fo->filter_expr,
          fo->over_def);
    }
      yymsp[-12].minor.yy213 = yylhsminor.yy213;
      break;
    case 11: /* expr ::= CAST LP expr AS typetoken RP */
    {
      yymsp[-5].minor.yy213 = synq_parse_cast_expr(
          pCtx, yymsp[-3].minor.yy213, synq_span(pCtx, yymsp[-1].minor.yy0));
    } break;
    case 12: /* typetoken ::= */
    {
      yymsp[1].minor.yy0.n = 0;
      yymsp[1].minor.yy0.z = 0;
    } break;
    case 13: /* typetoken ::= typename */
    {
      (void)yymsp[0].minor.yy0;
    } break;
    case 14: /* typetoken ::= typename LP signed RP */
    {
      yymsp[-3].minor.yy0.n =
          (int)(&yymsp[0].minor.yy0.z[yymsp[0].minor.yy0.n] -
                yymsp[-3].minor.yy0.z);
    } break;
    case 15: /* typetoken ::= typename LP signed COMMA signed RP */
    {
      yymsp[-5].minor.yy0.n =
          (int)(&yymsp[0].minor.yy0.z[yymsp[0].minor.yy0.n] -
                yymsp[-5].minor.yy0.z);
    } break;
    case 16: /* typename ::= ID|STRING */
    {
      synq_mark_as_type(pCtx, yymsp[0].minor.yy0);
      yylhsminor.yy0 = yymsp[0].minor.yy0;
    }
      yymsp[0].minor.yy0 = yylhsminor.yy0;
      break;
    case 17: /* typename ::= typename ID|STRING */
    {
      synq_mark_as_type(pCtx, yymsp[0].minor.yy0);
      yymsp[-1].minor.yy0.n =
          yymsp[0].minor.yy0.n +
          (int)(yymsp[0].minor.yy0.z - yymsp[-1].minor.yy0.z);
    } break;
    case 18: /* selcollist ::= sclp scanpt nm DOT STAR */
    {
      uint32_t col = synq_parse_result_column(
          pCtx, (SyntaqliteResultColumnFlags){.bits = {.star = 1}},
          synq_span(pCtx, yymsp[-2].minor.yy0), SYNTAQLITE_NULL_NODE);
      yylhsminor.yy213 =
          synq_parse_result_column_list(pCtx, yymsp[-4].minor.yy213, col);
    }
      yymsp[-4].minor.yy213 = yylhsminor.yy213;
      break;
    case 19: /* expr ::= ID|INDEXED|JOIN_KW */
    {
      synq_mark_as_id(pCtx, yymsp[0].minor.yy0);
      yylhsminor.yy213 =
          synq_parse_column_ref(pCtx, synq_span(pCtx, yymsp[0].minor.yy0),
                                SYNQ_NO_SPAN, SYNQ_NO_SPAN);
    }
      yymsp[0].minor.yy213 = yylhsminor.yy213;
      break;
    case 20: /* expr ::= nm DOT nm */
    {
      yylhsminor.yy213 = synq_parse_column_ref(
          pCtx, synq_span(pCtx, yymsp[0].minor.yy0),
          synq_span(pCtx, yymsp[-2].minor.yy0), SYNQ_NO_SPAN);
    }
      yymsp[-2].minor.yy213 = yylhsminor.yy213;
      break;
    case 21: /* expr ::= nm DOT nm DOT nm */
    {
      yylhsminor.yy213 =
          synq_parse_column_ref(pCtx, synq_span(pCtx, yymsp[0].minor.yy0),
                                synq_span(pCtx, yymsp[-2].minor.yy0),
                                synq_span(pCtx, yymsp[-4].minor.yy0));
    }
      yymsp[-4].minor.yy213 = yylhsminor.yy213;
      break;
    case 22: /* selectnowith ::= selectnowith multiselect_op oneselect */
    {
      yymsp[-2].minor.yy213 = synq_parse_compound_select(
          pCtx, (SyntaqliteCompoundOp)yymsp[-1].minor.yy220,
          yymsp[-2].minor.yy213, yymsp[0].minor.yy213);
    } break;
    case 23: /* multiselect_op ::= UNION */
    {
      yylhsminor.yy220 = 0;
      (void)yymsp[0].minor.yy0;
    }
      yymsp[0].minor.yy220 = yylhsminor.yy220;
      break;
    case 24: /* multiselect_op ::= UNION ALL */
    case 29: /* in_op ::= NOT IN */
      yytestcase(yyruleno == 29);
      {
        yymsp[-1].minor.yy220 = 1;
      }
      break;
    case 25: /* multiselect_op ::= EXCEPT|INTERSECT */
    {
      yylhsminor.yy220 =
          (yymsp[0].minor.yy0.type == SYNTAQLITE_TK_INTERSECT) ? 2 : 3;
    }
      yymsp[0].minor.yy220 = yylhsminor.yy220;
      break;
    case 26: /* expr ::= LP select RP */
    {
      pCtx->saw_subquery = 1;
      yymsp[-2].minor.yy213 =
          synq_parse_subquery_expr(pCtx, yymsp[-1].minor.yy213);
    } break;
    case 27: /* expr ::= EXISTS LP select RP */
    {
      pCtx->saw_subquery = 1;
      yymsp[-3].minor.yy213 =
          synq_parse_exists_expr(pCtx, yymsp[-1].minor.yy213);
    } break;
    case 28: /* in_op ::= IN */
    {
      yymsp[0].minor.yy220 = 0;
    } break;
    case 30: /* expr ::= expr in_op LP exprlist RP */
    {
      yymsp[-4].minor.yy213 =
          synq_parse_in_expr(pCtx, (SyntaqliteBool)yymsp[-3].minor.yy220,
                             yymsp[-4].minor.yy213, yymsp[-1].minor.yy213);
    } break;
    case 31: /* expr ::= expr in_op LP select RP */
    {
      pCtx->saw_subquery = 1;
      uint32_t sub = synq_parse_subquery_expr(pCtx, yymsp[-1].minor.yy213);
      yymsp[-4].minor.yy213 =
          synq_parse_in_expr(pCtx, (SyntaqliteBool)yymsp[-3].minor.yy220,
                             yymsp[-4].minor.yy213, sub);
    } break;
    case 32: /* expr ::= expr in_op nm dbnm paren_exprlist */
    {
      // Table-valued function IN expression - stub for now
      (void)yymsp[-2].minor.yy0;
      (void)yymsp[-1].minor.yy0;
      (void)yymsp[0].minor.yy213;
      yymsp[-4].minor.yy213 =
          synq_parse_in_expr(pCtx, (SyntaqliteBool)yymsp[-3].minor.yy220,
                             yymsp[-4].minor.yy213, SYNTAQLITE_NULL_NODE);
    } break;
    case 33: /* dbnm ::= */
    {
      yymsp[1].minor.yy0.z = NULL;
      yymsp[1].minor.yy0.n = 0;
    } break;
    case 34: /* dbnm ::= DOT nm */
    {
      yymsp[-1].minor.yy0 = yymsp[0].minor.yy0;
    } break;
    case 35: /* paren_exprlist ::= */
    {
      yymsp[1].minor.yy213 = SYNTAQLITE_NULL_NODE;
    } break;
    case 36: /* paren_exprlist ::= LP exprlist RP */
    {
      yymsp[-2].minor.yy213 = yymsp[-1].minor.yy213;
    } break;
    case 37: /* expr ::= expr ISNULL|NOTNULL */
    {
      SyntaqliteIsOp op = (yymsp[0].minor.yy0.type == SYNTAQLITE_TK_ISNULL)
                              ? SYNTAQLITE_IS_OP_ISNULL
                              : SYNTAQLITE_IS_OP_NOTNULL;
      yylhsminor.yy213 = synq_parse_is_expr(pCtx, op, yymsp[-1].minor.yy213,
                                            SYNTAQLITE_NULL_NODE);
    }
      yymsp[-1].minor.yy213 = yylhsminor.yy213;
      break;
    case 38: /* expr ::= expr NOT NULL */
    {
      yylhsminor.yy213 =
          synq_parse_is_expr(pCtx, SYNTAQLITE_IS_OP_NOTNULL,
                             yymsp[-2].minor.yy213, SYNTAQLITE_NULL_NODE);
    }
      yymsp[-2].minor.yy213 = yylhsminor.yy213;
      break;
    case 39: /* expr ::= expr IS expr */
    {
      yylhsminor.yy213 =
          synq_parse_is_expr(pCtx, SYNTAQLITE_IS_OP_IS, yymsp[-2].minor.yy213,
                             yymsp[0].minor.yy213);
    }
      yymsp[-2].minor.yy213 = yylhsminor.yy213;
      break;
    case 40: /* expr ::= expr IS NOT expr */
    {
      yylhsminor.yy213 =
          synq_parse_is_expr(pCtx, SYNTAQLITE_IS_OP_IS_NOT,
                             yymsp[-3].minor.yy213, yymsp[0].minor.yy213);
    }
      yymsp[-3].minor.yy213 = yylhsminor.yy213;
      break;
    case 41: /* expr ::= expr IS NOT DISTINCT FROM expr */
    {
      yylhsminor.yy213 =
          synq_parse_is_expr(pCtx, SYNTAQLITE_IS_OP_IS_NOT_DISTINCT,
                             yymsp[-5].minor.yy213, yymsp[0].minor.yy213);
    }
      yymsp[-5].minor.yy213 = yylhsminor.yy213;
      break;
    case 42: /* expr ::= expr IS DISTINCT FROM expr */
    {
      yylhsminor.yy213 =
          synq_parse_is_expr(pCtx, SYNTAQLITE_IS_OP_IS_DISTINCT,
                             yymsp[-4].minor.yy213, yymsp[0].minor.yy213);
    }
      yymsp[-4].minor.yy213 = yylhsminor.yy213;
      break;
    case 43:  /* between_op ::= BETWEEN */
    case 209: /* sortorder ::= ASC */
      yytestcase(yyruleno == 209);
    case 265: /* distinct ::= ALL */
      yytestcase(yyruleno == 265);
      {
        yymsp[0].minor.yy213 = 0;
      }
      break;
    case 44:  /* between_op ::= NOT BETWEEN */
    case 212: /* nulls ::= NULLS FIRST */
      yytestcase(yyruleno == 212);
      {
        yymsp[-1].minor.yy213 = 1;
      }
      break;
    case 45: /* expr ::= expr between_op expr AND expr */
    {
      yylhsminor.yy213 = synq_parse_between_expr(
          pCtx, (SyntaqliteBool)yymsp[-3].minor.yy213, yymsp[-4].minor.yy213,
          yymsp[-2].minor.yy213, yymsp[0].minor.yy213);
    }
      yymsp[-4].minor.yy213 = yylhsminor.yy213;
      break;
    case 46:  /* likeop ::= LIKE_KW|MATCH */
    case 199: /* nm ::= STRING */
      yytestcase(yyruleno == 199);
    case 262: /* as ::= ID|STRING */
      yytestcase(yyruleno == 262);
      {
        yylhsminor.yy0 = yymsp[0].minor.yy0;
      }
      yymsp[0].minor.yy0 = yylhsminor.yy0;
      break;
    case 47: /* likeop ::= NOT LIKE_KW|MATCH */
    {
      yymsp[-1].minor.yy0 = yymsp[0].minor.yy0;
      yymsp[-1].minor.yy0.n |= 0x80000000;
    } break;
    case 48: /* expr ::= expr likeop expr */
    {
      SyntaqliteBool negated = (yymsp[-1].minor.yy0.n & 0x80000000)
                                   ? SYNTAQLITE_BOOL_TRUE
                                   : SYNTAQLITE_BOOL_FALSE;
      yylhsminor.yy213 =
          synq_parse_like_expr(pCtx, negated, yymsp[-2].minor.yy213,
                               yymsp[0].minor.yy213, SYNTAQLITE_NULL_NODE);
    }
      yymsp[-2].minor.yy213 = yylhsminor.yy213;
      break;
    case 49: /* expr ::= expr likeop expr ESCAPE expr */
    {
      SyntaqliteBool negated = (yymsp[-3].minor.yy0.n & 0x80000000)
                                   ? SYNTAQLITE_BOOL_TRUE
                                   : SYNTAQLITE_BOOL_FALSE;
      yylhsminor.yy213 =
          synq_parse_like_expr(pCtx, negated, yymsp[-4].minor.yy213,
                               yymsp[-2].minor.yy213, yymsp[0].minor.yy213);
    }
      yymsp[-4].minor.yy213 = yylhsminor.yy213;
      break;
    case 50: /* expr ::= CASE case_operand case_exprlist case_else END */
    {
      yymsp[-4].minor.yy213 =
          synq_parse_case_expr(pCtx, yymsp[-3].minor.yy213,
                               yymsp[-1].minor.yy213, yymsp[-2].minor.yy213);
    } break;
    case 51: /* case_exprlist ::= case_exprlist WHEN expr THEN expr */
    {
      uint32_t w = synq_parse_case_when(pCtx, yymsp[-2].minor.yy213,
                                        yymsp[0].minor.yy213);
      yylhsminor.yy213 =
          synq_parse_case_when_list(pCtx, yymsp[-4].minor.yy213, w);
    }
      yymsp[-4].minor.yy213 = yylhsminor.yy213;
      break;
    case 52: /* case_exprlist ::= WHEN expr THEN expr */
    {
      uint32_t w = synq_parse_case_when(pCtx, yymsp[-2].minor.yy213,
                                        yymsp[0].minor.yy213);
      yymsp[-3].minor.yy213 =
          synq_parse_case_when_list(pCtx, SYNTAQLITE_NULL_NODE, w);
    } break;
    case 53:  /* case_else ::= ELSE expr */
    case 158: /* where_opt_ret ::= WHERE expr */
      yytestcase(yyruleno == 158);
    case 268: /* from ::= FROM seltablist */
      yytestcase(yyruleno == 268);
    case 270: /* where_opt ::= WHERE expr */
      yytestcase(yyruleno == 270);
    case 274: /* having_opt ::= HAVING expr */
      yytestcase(yyruleno == 274);
    case 310: /* when_clause ::= WHEN expr */
      yytestcase(yyruleno == 310);
    case 346: /* key_opt ::= KEY expr */
      yytestcase(yyruleno == 346);
    case 349: /* vinto ::= INTO expr */
      yytestcase(yyruleno == 349);
    case 405: /* window_clause ::= WINDOW windowdefn_list */
      yytestcase(yyruleno == 405);
      {
        yymsp[-1].minor.yy213 = yymsp[0].minor.yy213;
      }
      break;
    case 54: /* case_else ::= */
    case 56: /* case_operand ::= */
      yytestcase(yyruleno == 56);
    case 106: /* conslist_opt ::= */
      yytestcase(yyruleno == 106);
    case 131: /* eidlist_opt ::= */
      yytestcase(yyruleno == 131);
    case 157: /* where_opt_ret ::= */
      yytestcase(yyruleno == 157);
    case 165: /* idlist_opt ::= */
      yytestcase(yyruleno == 165);
    case 167: /* upsert ::= */
      yytestcase(yyruleno == 167);
    case 190: /* exprlist ::= */
      yytestcase(yyruleno == 190);
    case 259: /* sclp ::= */
      yytestcase(yyruleno == 259);
    case 267: /* from ::= */
      yytestcase(yyruleno == 267);
    case 269: /* where_opt ::= */
      yytestcase(yyruleno == 269);
    case 271: /* groupby_opt ::= */
      yytestcase(yyruleno == 271);
    case 273: /* having_opt ::= */
      yytestcase(yyruleno == 273);
    case 275: /* orderby_opt ::= */
      yytestcase(yyruleno == 275);
    case 277: /* limit_opt ::= */
      yytestcase(yyruleno == 277);
    case 282: /* stl_prefix ::= */
      yytestcase(yyruleno == 282);
    case 309: /* when_clause ::= */
      yytestcase(yyruleno == 309);
    case 345: /* key_opt ::= */
      yytestcase(yyruleno == 345);
    case 350: /* vinto ::= */
      yytestcase(yyruleno == 350);
    case 390: /* frame_opt ::= */
      yytestcase(yyruleno == 390);
      {
        yymsp[1].minor.yy213 = SYNTAQLITE_NULL_NODE;
      }
      break;
    case 57: /* cmd ::= create_table create_table_args */
    {
      // yymsp[0].minor.yy213 is either: (1) a CreateTableStmt node with
      // columns/constraints filled in or: (2) a CreateTableStmt node with
      // as_select filled in yymsp[-1].minor.yy213 has the table
      // name/schema/temp/ifnotexists info packed as a node. We need to merge
      // yymsp[-1].minor.yy213 info into yymsp[0].minor.yy213.
      SyntaqliteNode* ct_node = AST_NODE(&pCtx->ast, yymsp[-1].minor.yy213);
      SyntaqliteNode* args_node = AST_NODE(&pCtx->ast, yymsp[0].minor.yy213);
      args_node->create_table_stmt.table_name =
          ct_node->create_table_stmt.table_name;
      args_node->create_table_stmt.schema = ct_node->create_table_stmt.schema;
      args_node->create_table_stmt.is_temp = ct_node->create_table_stmt.is_temp;
      args_node->create_table_stmt.if_not_exists =
          ct_node->create_table_stmt.if_not_exists;
      yylhsminor.yy213 = yymsp[0].minor.yy213;
    }
      yymsp[-1].minor.yy213 = yylhsminor.yy213;
      break;
    case 58: /* create_table ::= createkw temp TABLE ifnotexists nm dbnm */
    {
      SyntaqliteSourceSpan tbl_name =
          yymsp[0].minor.yy0.z ? synq_span(pCtx, yymsp[0].minor.yy0)
                               : synq_span(pCtx, yymsp[-1].minor.yy0);
      SyntaqliteSourceSpan tbl_schema =
          yymsp[0].minor.yy0.z ? synq_span(pCtx, yymsp[-1].minor.yy0)
                               : SYNQ_NO_SPAN;
      yymsp[-5].minor.yy213 = synq_parse_create_table_stmt(
          pCtx, tbl_name, tbl_schema, (SyntaqliteBool)yymsp[-4].minor.yy220,
          (SyntaqliteBool)yymsp[-2].minor.yy220,
          (SyntaqliteCreateTableStmtFlags){.raw = 0}, SYNTAQLITE_NULL_NODE,
          SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE);
    } break;
    case 59: /* create_table_args ::= LP columnlist conslist_opt RP
                table_option_set */
    {
      yymsp[-4].minor.yy213 = synq_parse_create_table_stmt(
          pCtx, SYNQ_NO_SPAN, SYNQ_NO_SPAN, SYNTAQLITE_BOOL_FALSE,
          SYNTAQLITE_BOOL_FALSE,
          (SyntaqliteCreateTableStmtFlags){.raw =
                                               (uint8_t)yymsp[0].minor.yy220},
          yymsp[-3].minor.yy213, yymsp[-2].minor.yy213, SYNTAQLITE_NULL_NODE);
    } break;
    case 60: /* create_table_args ::= AS select */
    {
      yymsp[-1].minor.yy213 = synq_parse_create_table_stmt(
          pCtx, SYNQ_NO_SPAN, SYNQ_NO_SPAN, SYNTAQLITE_BOOL_FALSE,
          SYNTAQLITE_BOOL_FALSE, (SyntaqliteCreateTableStmtFlags){.raw = 0},
          SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, yymsp[0].minor.yy213);
    } break;
    case 61: /* table_option_set ::= */
    case 88: /* autoinc ::= */
      yytestcase(yyruleno == 88);
    case 103: /* init_deferred_pred_opt ::= */
      yytestcase(yyruleno == 103);
    case 117: /* defer_subclause_opt ::= */
      yytestcase(yyruleno == 117);
    case 135: /* collate ::= */
      yytestcase(yyruleno == 135);
    case 223: /* ifexists ::= */
      yytestcase(yyruleno == 223);
    case 233: /* kwcolumn_opt ::= */
      yytestcase(yyruleno == 233);
    case 243: /* trans_opt ::= */
      yytestcase(yyruleno == 243);
    case 247: /* savepoint_opt ::= */
      yytestcase(yyruleno == 247);
    case 356: /* uniqueflag ::= */
      yytestcase(yyruleno == 356);
    case 357: /* ifnotexists ::= */
      yytestcase(yyruleno == 357);
    case 362: /* temp ::= */
      yytestcase(yyruleno == 362);
      {
        yymsp[1].minor.yy220 = 0;
      }
      break;
    case 62:  /* table_option_set ::= table_option */
    case 118: /* defer_subclause_opt ::= defer_subclause */
      yytestcase(yyruleno == 118);
      {
        // passthrough
      }
      break;
    case 63: /* table_option_set ::= table_option_set COMMA table_option */
    {
      yylhsminor.yy220 = yymsp[-2].minor.yy220 | yymsp[0].minor.yy220;
    }
      yymsp[-2].minor.yy220 = yylhsminor.yy220;
      break;
    case 64: /* table_option ::= WITHOUT nm */
    {
      // WITHOUT ROWID = bit 0
      if (yymsp[0].minor.yy0.n == 5 &&
          strncasecmp(yymsp[0].minor.yy0.z, "rowid", 5) == 0) {
        yymsp[-1].minor.yy220 = 1;
      } else {
        yymsp[-1].minor.yy220 = 0;
      }
    } break;
    case 65: /* table_option ::= nm */
    {
      // STRICT = bit 1
      if (yymsp[0].minor.yy0.n == 6 &&
          strncasecmp(yymsp[0].minor.yy0.z, "strict", 6) == 0) {
        yylhsminor.yy220 = 2;
      } else {
        yylhsminor.yy220 = 0;
      }
    }
      yymsp[0].minor.yy220 = yylhsminor.yy220;
      break;
    case 66: /* columnlist ::= columnlist COMMA columnname carglist */
    {
      uint32_t col = synq_parse_column_def(pCtx, yymsp[-1].minor.yy400.name,
                                           yymsp[-1].minor.yy400.typetoken,
                                           yymsp[0].minor.yy10.list);
      yylhsminor.yy213 =
          synq_parse_column_def_list(pCtx, yymsp[-3].minor.yy213, col);
    }
      yymsp[-3].minor.yy213 = yylhsminor.yy213;
      break;
    case 67: /* columnlist ::= columnname carglist */
    {
      uint32_t col = synq_parse_column_def(pCtx, yymsp[-1].minor.yy400.name,
                                           yymsp[-1].minor.yy400.typetoken,
                                           yymsp[0].minor.yy10.list);
      yylhsminor.yy213 =
          synq_parse_column_def_list(pCtx, SYNTAQLITE_NULL_NODE, col);
    }
      yymsp[-1].minor.yy213 = yylhsminor.yy213;
      break;
    case 68: /* carglist ::= carglist ccons */
    {
      if (yymsp[0].minor.yy34.node != SYNTAQLITE_NULL_NODE) {
        // Apply pending constraint name from the list to this node
        SyntaqliteNode* node = AST_NODE(&pCtx->ast, yymsp[0].minor.yy34.node);
        node->column_constraint.constraint_name =
            yymsp[-1].minor.yy10.pending_name;
        if (yymsp[-1].minor.yy10.list == SYNTAQLITE_NULL_NODE) {
          yylhsminor.yy10.list = synq_parse_column_constraint_list(
              pCtx, SYNTAQLITE_NULL_NODE, yymsp[0].minor.yy34.node);
        } else {
          yylhsminor.yy10.list = synq_parse_column_constraint_list(
              pCtx, yymsp[-1].minor.yy10.list, yymsp[0].minor.yy34.node);
        }
        yylhsminor.yy10.pending_name = SYNQ_NO_SPAN;
      } else if (yymsp[0].minor.yy34.pending_name.length > 0) {
        // CONSTRAINT nm — store pending name for next constraint
        yylhsminor.yy10.list = yymsp[-1].minor.yy10.list;
        yylhsminor.yy10.pending_name = yymsp[0].minor.yy34.pending_name;
      } else {
        yylhsminor.yy10 = yymsp[-1].minor.yy10;
      }
    }
      yymsp[-1].minor.yy10 = yylhsminor.yy10;
      break;
    case 69: /* carglist ::= */
    {
      yymsp[1].minor.yy10.list = SYNTAQLITE_NULL_NODE;
      yymsp[1].minor.yy10.pending_name = SYNQ_NO_SPAN;
    } break;
    case 70:  /* ccons ::= CONSTRAINT nm */
    case 112: /* tcons ::= CONSTRAINT nm */
      yytestcase(yyruleno == 112);
      {
        yymsp[-1].minor.yy34.node = SYNTAQLITE_NULL_NODE;
        yymsp[-1].minor.yy34.pending_name = synq_span(pCtx, yymsp[0].minor.yy0);
      }
      break;
    case 71: /* ccons ::= DEFAULT scantok term */
    {
      yymsp[-2].minor.yy34.node = synq_parse_column_constraint(
          pCtx, SYNTAQLITE_COLUMN_CONSTRAINT_KIND_DEFAULT, SYNQ_NO_SPAN,
          SYNTAQLITE_CONFLICT_ACTION_DEFAULT, SYNTAQLITE_SORT_ORDER_ASC,
          SYNTAQLITE_BOOL_FALSE, SYNQ_NO_SPAN,
          SYNTAQLITE_GENERATED_COLUMN_STORAGE_VIRTUAL, yymsp[0].minor.yy213,
          SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE);
      yymsp[-2].minor.yy34.pending_name = SYNQ_NO_SPAN;
    } break;
    case 72: /* ccons ::= DEFAULT LP expr RP */
    {
      yymsp[-3].minor.yy34.node = synq_parse_column_constraint(
          pCtx, SYNTAQLITE_COLUMN_CONSTRAINT_KIND_DEFAULT, SYNQ_NO_SPAN,
          SYNTAQLITE_CONFLICT_ACTION_DEFAULT, SYNTAQLITE_SORT_ORDER_ASC,
          SYNTAQLITE_BOOL_FALSE, SYNQ_NO_SPAN,
          SYNTAQLITE_GENERATED_COLUMN_STORAGE_VIRTUAL, yymsp[-1].minor.yy213,
          SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE);
      yymsp[-3].minor.yy34.pending_name = SYNQ_NO_SPAN;
    } break;
    case 73: /* ccons ::= DEFAULT PLUS scantok term */
    {
      yymsp[-3].minor.yy34.node = synq_parse_column_constraint(
          pCtx, SYNTAQLITE_COLUMN_CONSTRAINT_KIND_DEFAULT, SYNQ_NO_SPAN,
          SYNTAQLITE_CONFLICT_ACTION_DEFAULT, SYNTAQLITE_SORT_ORDER_ASC,
          SYNTAQLITE_BOOL_FALSE, SYNQ_NO_SPAN,
          SYNTAQLITE_GENERATED_COLUMN_STORAGE_VIRTUAL, yymsp[0].minor.yy213,
          SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE);
      yymsp[-3].minor.yy34.pending_name = SYNQ_NO_SPAN;
    } break;
    case 74: /* ccons ::= DEFAULT MINUS scantok term */
    {
      // Create a unary minus wrapping the term
      uint32_t neg = synq_parse_unary_expr(pCtx, SYNTAQLITE_UNARY_OP_MINUS,
                                           yymsp[0].minor.yy213);
      yymsp[-3].minor.yy34.node = synq_parse_column_constraint(
          pCtx, SYNTAQLITE_COLUMN_CONSTRAINT_KIND_DEFAULT, SYNQ_NO_SPAN,
          SYNTAQLITE_CONFLICT_ACTION_DEFAULT, SYNTAQLITE_SORT_ORDER_ASC,
          SYNTAQLITE_BOOL_FALSE, SYNQ_NO_SPAN,
          SYNTAQLITE_GENERATED_COLUMN_STORAGE_VIRTUAL, neg,
          SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE);
      yymsp[-3].minor.yy34.pending_name = SYNQ_NO_SPAN;
    } break;
    case 75: /* ccons ::= DEFAULT scantok ID|INDEXED */
    {
      // Treat the identifier as a literal expression
      uint32_t lit = synq_parse_literal(pCtx, SYNTAQLITE_LITERAL_TYPE_STRING,
                                        synq_span(pCtx, yymsp[0].minor.yy0));
      yymsp[-2].minor.yy34.node = synq_parse_column_constraint(
          pCtx, SYNTAQLITE_COLUMN_CONSTRAINT_KIND_DEFAULT, SYNQ_NO_SPAN,
          SYNTAQLITE_CONFLICT_ACTION_DEFAULT, SYNTAQLITE_SORT_ORDER_ASC,
          SYNTAQLITE_BOOL_FALSE, SYNQ_NO_SPAN,
          SYNTAQLITE_GENERATED_COLUMN_STORAGE_VIRTUAL, lit,
          SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE);
      yymsp[-2].minor.yy34.pending_name = SYNQ_NO_SPAN;
    } break;
    case 76: /* ccons ::= NULL onconf */
    {
      yymsp[-1].minor.yy34.node = synq_parse_column_constraint(
          pCtx, SYNTAQLITE_COLUMN_CONSTRAINT_KIND_NULL, SYNQ_NO_SPAN,
          (SyntaqliteConflictAction)yymsp[0].minor.yy220,
          SYNTAQLITE_SORT_ORDER_ASC, SYNTAQLITE_BOOL_FALSE, SYNQ_NO_SPAN,
          SYNTAQLITE_GENERATED_COLUMN_STORAGE_VIRTUAL, SYNTAQLITE_NULL_NODE,
          SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE);
      yymsp[-1].minor.yy34.pending_name = SYNQ_NO_SPAN;
    } break;
    case 77: /* ccons ::= NOT NULL onconf */
    {
      yymsp[-2].minor.yy34.node = synq_parse_column_constraint(
          pCtx, SYNTAQLITE_COLUMN_CONSTRAINT_KIND_NOT_NULL, SYNQ_NO_SPAN,
          (SyntaqliteConflictAction)yymsp[0].minor.yy220,
          SYNTAQLITE_SORT_ORDER_ASC, SYNTAQLITE_BOOL_FALSE, SYNQ_NO_SPAN,
          SYNTAQLITE_GENERATED_COLUMN_STORAGE_VIRTUAL, SYNTAQLITE_NULL_NODE,
          SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE);
      yymsp[-2].minor.yy34.pending_name = SYNQ_NO_SPAN;
    } break;
    case 78: /* ccons ::= PRIMARY KEY sortorder onconf autoinc */
    {
      yymsp[-4].minor.yy34.node = synq_parse_column_constraint(
          pCtx, SYNTAQLITE_COLUMN_CONSTRAINT_KIND_PRIMARY_KEY, SYNQ_NO_SPAN,
          (SyntaqliteConflictAction)yymsp[-1].minor.yy220,
          (SyntaqliteSortOrder)yymsp[-2].minor.yy213,
          (SyntaqliteBool)yymsp[0].minor.yy220, SYNQ_NO_SPAN,
          SYNTAQLITE_GENERATED_COLUMN_STORAGE_VIRTUAL, SYNTAQLITE_NULL_NODE,
          SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE);
      yymsp[-4].minor.yy34.pending_name = SYNQ_NO_SPAN;
    } break;
    case 79: /* ccons ::= UNIQUE onconf */
    {
      yymsp[-1].minor.yy34.node = synq_parse_column_constraint(
          pCtx, SYNTAQLITE_COLUMN_CONSTRAINT_KIND_UNIQUE, SYNQ_NO_SPAN,
          (SyntaqliteConflictAction)yymsp[0].minor.yy220,
          SYNTAQLITE_SORT_ORDER_ASC, SYNTAQLITE_BOOL_FALSE, SYNQ_NO_SPAN,
          SYNTAQLITE_GENERATED_COLUMN_STORAGE_VIRTUAL, SYNTAQLITE_NULL_NODE,
          SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE);
      yymsp[-1].minor.yy34.pending_name = SYNQ_NO_SPAN;
    } break;
    case 80: /* ccons ::= CHECK LP expr RP */
    {
      yymsp[-3].minor.yy34.node = synq_parse_column_constraint(
          pCtx, SYNTAQLITE_COLUMN_CONSTRAINT_KIND_CHECK, SYNQ_NO_SPAN,
          SYNTAQLITE_CONFLICT_ACTION_DEFAULT, SYNTAQLITE_SORT_ORDER_ASC,
          SYNTAQLITE_BOOL_FALSE, SYNQ_NO_SPAN,
          SYNTAQLITE_GENERATED_COLUMN_STORAGE_VIRTUAL, SYNTAQLITE_NULL_NODE,
          yymsp[-1].minor.yy213, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE);
      yymsp[-3].minor.yy34.pending_name = SYNQ_NO_SPAN;
    } break;
    case 81: /* ccons ::= REFERENCES nm eidlist_opt refargs */
    {
      // Decode refargs: low byte = on_delete, next byte = on_update
      SyntaqliteForeignKeyAction on_del =
          (SyntaqliteForeignKeyAction)(yymsp[0].minor.yy220 & 0xff);
      SyntaqliteForeignKeyAction on_upd =
          (SyntaqliteForeignKeyAction)((yymsp[0].minor.yy220 >> 8) & 0xff);
      uint32_t fk = synq_parse_foreign_key_clause(
          pCtx, synq_span(pCtx, yymsp[-2].minor.yy0), yymsp[-1].minor.yy213,
          on_del, on_upd, SYNTAQLITE_BOOL_FALSE);
      yymsp[-3].minor.yy34.node = synq_parse_column_constraint(
          pCtx, SYNTAQLITE_COLUMN_CONSTRAINT_KIND_REFERENCES, SYNQ_NO_SPAN,
          SYNTAQLITE_CONFLICT_ACTION_DEFAULT, SYNTAQLITE_SORT_ORDER_ASC,
          SYNTAQLITE_BOOL_FALSE, SYNQ_NO_SPAN,
          SYNTAQLITE_GENERATED_COLUMN_STORAGE_VIRTUAL, SYNTAQLITE_NULL_NODE,
          SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, fk);
      yymsp[-3].minor.yy34.pending_name = SYNQ_NO_SPAN;
    } break;
    case 82: /* ccons ::= defer_subclause */
    {
      // Create a minimal constraint that just marks deferral.
      // In practice, this follows a REFERENCES ccons. We'll handle it
      // by updating the last constraint in the list if possible.
      // For simplicity, we create a separate REFERENCES constraint with just
      // deferral info. The printer will show it as a separate constraint entry.
      uint32_t fk = synq_parse_foreign_key_clause(
          pCtx, SYNQ_NO_SPAN, SYNTAQLITE_NULL_NODE,
          SYNTAQLITE_FOREIGN_KEY_ACTION_NO_ACTION,
          SYNTAQLITE_FOREIGN_KEY_ACTION_NO_ACTION,
          (SyntaqliteBool)yymsp[0].minor.yy220);
      yylhsminor.yy34.node = synq_parse_column_constraint(
          pCtx, SYNTAQLITE_COLUMN_CONSTRAINT_KIND_REFERENCES, SYNQ_NO_SPAN,
          SYNTAQLITE_CONFLICT_ACTION_DEFAULT, SYNTAQLITE_SORT_ORDER_ASC,
          SYNTAQLITE_BOOL_FALSE, SYNQ_NO_SPAN,
          SYNTAQLITE_GENERATED_COLUMN_STORAGE_VIRTUAL, SYNTAQLITE_NULL_NODE,
          SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, fk);
      yylhsminor.yy34.pending_name = SYNQ_NO_SPAN;
    }
      yymsp[0].minor.yy34 = yylhsminor.yy34;
      break;
    case 83: /* ccons ::= COLLATE ID|STRING */
    {
      yymsp[-1].minor.yy34.node = synq_parse_column_constraint(
          pCtx, SYNTAQLITE_COLUMN_CONSTRAINT_KIND_COLLATE, SYNQ_NO_SPAN, 0, 0,
          0, synq_span(pCtx, yymsp[0].minor.yy0),
          SYNTAQLITE_GENERATED_COLUMN_STORAGE_VIRTUAL, SYNTAQLITE_NULL_NODE,
          SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE);
      yymsp[-1].minor.yy34.pending_name = SYNQ_NO_SPAN;
    } break;
    case 84: /* ccons ::= GENERATED ALWAYS AS generated */
    {
      yymsp[-3].minor.yy34 = yymsp[0].minor.yy34;
    } break;
    case 85: /* ccons ::= AS generated */
    {
      yymsp[-1].minor.yy34 = yymsp[0].minor.yy34;
    } break;
    case 86: /* generated ::= LP expr RP */
    {
      yymsp[-2].minor.yy34.node = synq_parse_column_constraint(
          pCtx, SYNTAQLITE_COLUMN_CONSTRAINT_KIND_GENERATED, SYNQ_NO_SPAN,
          SYNTAQLITE_CONFLICT_ACTION_DEFAULT, SYNTAQLITE_SORT_ORDER_ASC,
          SYNTAQLITE_BOOL_FALSE, SYNQ_NO_SPAN,
          SYNTAQLITE_GENERATED_COLUMN_STORAGE_VIRTUAL, SYNTAQLITE_NULL_NODE,
          SYNTAQLITE_NULL_NODE, yymsp[-1].minor.yy213, SYNTAQLITE_NULL_NODE);
      yymsp[-2].minor.yy34.pending_name = SYNQ_NO_SPAN;
    } break;
    case 87: /* generated ::= LP expr RP ID */
    {
      SyntaqliteGeneratedColumnStorage storage =
          SYNTAQLITE_GENERATED_COLUMN_STORAGE_VIRTUAL;
      if (yymsp[0].minor.yy0.n == 6 &&
          strncasecmp(yymsp[0].minor.yy0.z, "stored", 6) == 0) {
        storage = SYNTAQLITE_GENERATED_COLUMN_STORAGE_STORED;
      }
      yymsp[-3].minor.yy34.node = synq_parse_column_constraint(
          pCtx, SYNTAQLITE_COLUMN_CONSTRAINT_KIND_GENERATED, SYNQ_NO_SPAN,
          SYNTAQLITE_CONFLICT_ACTION_DEFAULT, SYNTAQLITE_SORT_ORDER_ASC,
          SYNTAQLITE_BOOL_FALSE, SYNQ_NO_SPAN, storage, SYNTAQLITE_NULL_NODE,
          SYNTAQLITE_NULL_NODE, yymsp[-2].minor.yy213, SYNTAQLITE_NULL_NODE);
      yymsp[-3].minor.yy34.pending_name = SYNQ_NO_SPAN;
    } break;
    case 89:  /* autoinc ::= AUTOINCR */
    case 234: /* kwcolumn_opt ::= COLUMNKW */
      yytestcase(yyruleno == 234);
    case 352: /* explain ::= EXPLAIN */
      yytestcase(yyruleno == 352);
    case 355: /* uniqueflag ::= UNIQUE */
      yytestcase(yyruleno == 355);
    case 361: /* temp ::= TEMP */
      yytestcase(yyruleno == 361);
      {
        yymsp[0].minor.yy220 = 1;
      }
      break;
    case 90: /* refargs ::= */
    {
      yymsp[1].minor.yy220 = 0;  // NO_ACTION for both
    } break;
    case 91: /* refargs ::= refargs refarg */
    {
      // refarg encodes: low byte = value, byte 1 = shift amount (0 or 8)
      int val = yymsp[0].minor.yy220 & 0xff;
      int shift = (yymsp[0].minor.yy220 >> 8) & 0xff;
      // Clear the target byte in yymsp[-1].minor.yy220 and set new value
      yymsp[-1].minor.yy220 =
          (yymsp[-1].minor.yy220 & ~(0xff << shift)) | (val << shift);
    } break;
    case 92: /* refarg ::= MATCH nm */
    {
      yymsp[-1].minor.yy220 = 0;  // MATCH is ignored
    } break;
    case 93: /* refarg ::= ON INSERT refact */
    {
      yymsp[-2].minor.yy220 = 0;  // ON INSERT is ignored
    } break;
    case 94: /* refarg ::= ON DELETE refact */
    {
      yymsp[-2].minor.yy220 = yymsp[0].minor.yy220;  // shift=0 for DELETE
    } break;
    case 95: /* refarg ::= ON UPDATE refact */
    {
      yymsp[-2].minor.yy220 =
          yymsp[0].minor.yy220 | (8 << 8);  // shift=8 for UPDATE
    } break;
    case 96: /* refact ::= SET NULL */
    {
      yymsp[-1].minor.yy220 = (int)SYNTAQLITE_FOREIGN_KEY_ACTION_SET_NULL;
    } break;
    case 97: /* refact ::= SET DEFAULT */
    {
      yymsp[-1].minor.yy220 = (int)SYNTAQLITE_FOREIGN_KEY_ACTION_SET_DEFAULT;
    } break;
    case 98: /* refact ::= CASCADE */
    {
      yymsp[0].minor.yy220 = (int)SYNTAQLITE_FOREIGN_KEY_ACTION_CASCADE;
    } break;
    case 99: /* refact ::= RESTRICT */
    {
      yymsp[0].minor.yy220 = (int)SYNTAQLITE_FOREIGN_KEY_ACTION_RESTRICT;
    } break;
    case 100: /* refact ::= NO ACTION */
    {
      yymsp[-1].minor.yy220 = (int)SYNTAQLITE_FOREIGN_KEY_ACTION_NO_ACTION;
    } break;
    case 101: /* defer_subclause ::= NOT DEFERRABLE init_deferred_pred_opt */
    {
      yymsp[-2].minor.yy220 = 0;
    } break;
    case 102: /* defer_subclause ::= DEFERRABLE init_deferred_pred_opt */
    case 144: /* insert_cmd ::= INSERT orconf */
      yytestcase(yyruleno == 144);
    case 147: /* orconf ::= OR resolvetype */
      yytestcase(yyruleno == 147);
    case 401: /* frame_exclude_opt ::= EXCLUDE frame_exclude */
      yytestcase(yyruleno == 401);
      {
        yymsp[-1].minor.yy220 = yymsp[0].minor.yy220;
      }
      break;
    case 104: /* init_deferred_pred_opt ::= INITIALLY DEFERRED */
    case 136: /* collate ::= COLLATE ID|STRING */
      yytestcase(yyruleno == 136);
    case 222: /* ifexists ::= IF EXISTS */
      yytestcase(yyruleno == 222);
      {
        yymsp[-1].minor.yy220 = 1;
      }
      break;
    case 105: /* init_deferred_pred_opt ::= INITIALLY IMMEDIATE */
    case 245: /* trans_opt ::= TRANSACTION nm */
      yytestcase(yyruleno == 245);
      {
        yymsp[-1].minor.yy220 = 0;
      }
      break;
    case 107: /* conslist_opt ::= COMMA conslist */
    {
      yymsp[-1].minor.yy213 = yymsp[0].minor.yy10.list;
    } break;
    case 108: /* conslist ::= conslist tconscomma tcons */
    {
      // If comma separator was present, clear pending constraint name
      SyntaqliteSourceSpan pending = yymsp[-1].minor.yy220
                                         ? SYNQ_NO_SPAN
                                         : yymsp[-2].minor.yy10.pending_name;
      if (yymsp[0].minor.yy34.node != SYNTAQLITE_NULL_NODE) {
        SyntaqliteNode* node = AST_NODE(&pCtx->ast, yymsp[0].minor.yy34.node);
        node->table_constraint.constraint_name = pending;
        if (yymsp[-2].minor.yy10.list == SYNTAQLITE_NULL_NODE) {
          yylhsminor.yy10.list = synq_parse_table_constraint_list(
              pCtx, SYNTAQLITE_NULL_NODE, yymsp[0].minor.yy34.node);
        } else {
          yylhsminor.yy10.list = synq_parse_table_constraint_list(
              pCtx, yymsp[-2].minor.yy10.list, yymsp[0].minor.yy34.node);
        }
        yylhsminor.yy10.pending_name = SYNQ_NO_SPAN;
      } else if (yymsp[0].minor.yy34.pending_name.length > 0) {
        yylhsminor.yy10.list = yymsp[-2].minor.yy10.list;
        yylhsminor.yy10.pending_name = yymsp[0].minor.yy34.pending_name;
      } else {
        yylhsminor.yy10 = yymsp[-2].minor.yy10;
      }
    }
      yymsp[-2].minor.yy10 = yylhsminor.yy10;
      break;
    case 109: /* conslist ::= tcons */
    {
      if (yymsp[0].minor.yy34.node != SYNTAQLITE_NULL_NODE) {
        yylhsminor.yy10.list = synq_parse_table_constraint_list(
            pCtx, SYNTAQLITE_NULL_NODE, yymsp[0].minor.yy34.node);
        yylhsminor.yy10.pending_name = SYNQ_NO_SPAN;
      } else {
        yylhsminor.yy10.list = SYNTAQLITE_NULL_NODE;
        yylhsminor.yy10.pending_name = yymsp[0].minor.yy34.pending_name;
      }
    }
      yymsp[0].minor.yy10 = yylhsminor.yy10;
      break;
    case 110: /* tconscomma ::= COMMA */
    {
      yymsp[0].minor.yy220 = 1;
    } break;
    case 111: /* tconscomma ::= */
    {
      yymsp[1].minor.yy220 = 0;
    } break;
    case 113: /* tcons ::= PRIMARY KEY LP sortlist autoinc RP onconf */
    {
      yymsp[-6].minor.yy34.node = synq_parse_table_constraint(
          pCtx, SYNTAQLITE_TABLE_CONSTRAINT_KIND_PRIMARY_KEY, SYNQ_NO_SPAN,
          (SyntaqliteConflictAction)yymsp[0].minor.yy220,
          (SyntaqliteBool)yymsp[-2].minor.yy220, yymsp[-3].minor.yy213,
          SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE);
      yymsp[-6].minor.yy34.pending_name = SYNQ_NO_SPAN;
    } break;
    case 114: /* tcons ::= UNIQUE LP sortlist RP onconf */
    {
      yymsp[-4].minor.yy34.node = synq_parse_table_constraint(
          pCtx, SYNTAQLITE_TABLE_CONSTRAINT_KIND_UNIQUE, SYNQ_NO_SPAN,
          (SyntaqliteConflictAction)yymsp[0].minor.yy220, SYNTAQLITE_BOOL_FALSE,
          yymsp[-2].minor.yy213, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE,
          SYNTAQLITE_NULL_NODE);
      yymsp[-4].minor.yy34.pending_name = SYNQ_NO_SPAN;
    } break;
    case 115: /* tcons ::= CHECK LP expr RP onconf */
    {
      yymsp[-4].minor.yy34.node = synq_parse_table_constraint(
          pCtx, SYNTAQLITE_TABLE_CONSTRAINT_KIND_CHECK, SYNQ_NO_SPAN,
          (SyntaqliteConflictAction)yymsp[0].minor.yy220, SYNTAQLITE_BOOL_FALSE,
          SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, yymsp[-2].minor.yy213,
          SYNTAQLITE_NULL_NODE);
      yymsp[-4].minor.yy34.pending_name = SYNQ_NO_SPAN;
    } break;
    case 116: /* tcons ::= FOREIGN KEY LP eidlist RP REFERENCES nm eidlist_opt
                 refargs defer_subclause_opt */
    {
      SyntaqliteForeignKeyAction on_del =
          (SyntaqliteForeignKeyAction)(yymsp[-1].minor.yy220 & 0xff);
      SyntaqliteForeignKeyAction on_upd =
          (SyntaqliteForeignKeyAction)((yymsp[-1].minor.yy220 >> 8) & 0xff);
      uint32_t fk = synq_parse_foreign_key_clause(
          pCtx, synq_span(pCtx, yymsp[-3].minor.yy0), yymsp[-2].minor.yy213,
          on_del, on_upd, (SyntaqliteBool)yymsp[0].minor.yy220);
      yymsp[-9].minor.yy34.node = synq_parse_table_constraint(
          pCtx, SYNTAQLITE_TABLE_CONSTRAINT_KIND_FOREIGN_KEY, SYNQ_NO_SPAN,
          SYNTAQLITE_CONFLICT_ACTION_DEFAULT, SYNTAQLITE_BOOL_FALSE,
          SYNTAQLITE_NULL_NODE, yymsp[-6].minor.yy213, SYNTAQLITE_NULL_NODE,
          fk);
      yymsp[-9].minor.yy34.pending_name = SYNQ_NO_SPAN;
    } break;
    case 119: /* onconf ::= */
    case 146: /* orconf ::= */
      yytestcase(yyruleno == 146);
      {
        yymsp[1].minor.yy220 = (int)SYNTAQLITE_CONFLICT_ACTION_DEFAULT;
      }
      break;
    case 120: /* onconf ::= ON CONFLICT resolvetype */
    {
      yymsp[-2].minor.yy220 = yymsp[0].minor.yy220;
    } break;
    case 121: /* scantok ::= */
    case 155: /* indexed_opt ::= */
      yytestcase(yyruleno == 155);
    case 260: /* scanpt ::= */
      yytestcase(yyruleno == 260);
    case 263: /* as ::= */
      yytestcase(yyruleno == 263);
      {
        yymsp[1].minor.yy0.z = NULL;
        yymsp[1].minor.yy0.n = 0;
      }
      break;
    case 122: /* select ::= WITH wqlist selectnowith */
    {
      yymsp[-2].minor.yy213 = synq_parse_with_clause(
          pCtx, 0, yymsp[-1].minor.yy213, yymsp[0].minor.yy213);
    } break;
    case 123: /* select ::= WITH RECURSIVE wqlist selectnowith */
    {
      yymsp[-3].minor.yy213 = synq_parse_with_clause(
          pCtx, 1, yymsp[-1].minor.yy213, yymsp[0].minor.yy213);
    } break;
    case 124: /* wqitem ::= withnm eidlist_opt wqas LP select RP */
    {
      yylhsminor.yy213 = synq_parse_cte_definition(
          pCtx, synq_span(pCtx, yymsp[-5].minor.yy0),
          (SyntaqliteMaterialized)yymsp[-3].minor.yy220, yymsp[-4].minor.yy213,
          yymsp[-1].minor.yy213);
    }
      yymsp[-5].minor.yy213 = yylhsminor.yy213;
      break;
    case 125: /* wqlist ::= wqitem */
    {
      yylhsminor.yy213 =
          synq_parse_cte_list(pCtx, SYNTAQLITE_NULL_NODE, yymsp[0].minor.yy213);
    }
      yymsp[0].minor.yy213 = yylhsminor.yy213;
      break;
    case 126: /* wqlist ::= wqlist COMMA wqitem */
    {
      yymsp[-2].minor.yy213 = synq_parse_cte_list(pCtx, yymsp[-2].minor.yy213,
                                                  yymsp[0].minor.yy213);
    } break;
    case 127: /* withnm ::= nm */
    {
      // Token passthrough - nm already produces SynqParseToken
    } break;
    case 128: /* wqas ::= AS */
    {
      yymsp[0].minor.yy220 = (int)SYNTAQLITE_MATERIALIZED_DEFAULT;
    } break;
    case 129: /* wqas ::= AS MATERIALIZED */
    {
      yymsp[-1].minor.yy220 = (int)SYNTAQLITE_MATERIALIZED_MATERIALIZED;
    } break;
    case 130: /* wqas ::= AS NOT MATERIALIZED */
    {
      yymsp[-2].minor.yy220 = (int)SYNTAQLITE_MATERIALIZED_NOT_MATERIALIZED;
    } break;
    case 132: /* eidlist_opt ::= LP eidlist RP */
    case 166: /* idlist_opt ::= LP idlist RP */
      yytestcase(yyruleno == 166);
    case 176: /* expr ::= LP expr RP */
      yytestcase(yyruleno == 176);
    case 321: /* trigger_cmd ::= scanpt select scanpt */
      yytestcase(yyruleno == 321);
      {
        yymsp[-2].minor.yy213 = yymsp[-1].minor.yy213;
      }
      break;
    case 133: /* eidlist ::= nm collate sortorder */
    {
      (void)yymsp[-1].minor.yy220;
      (void)yymsp[0].minor.yy213;
      uint32_t col =
          synq_parse_column_ref(pCtx, synq_span(pCtx, yymsp[-2].minor.yy0),
                                SYNQ_NO_SPAN, SYNQ_NO_SPAN);
      yylhsminor.yy213 = synq_parse_expr_list(pCtx, SYNTAQLITE_NULL_NODE, col);
    }
      yymsp[-2].minor.yy213 = yylhsminor.yy213;
      break;
    case 134: /* eidlist ::= eidlist COMMA nm collate sortorder */
    {
      (void)yymsp[-1].minor.yy220;
      (void)yymsp[0].minor.yy213;
      uint32_t col =
          synq_parse_column_ref(pCtx, synq_span(pCtx, yymsp[-2].minor.yy0),
                                SYNQ_NO_SPAN, SYNQ_NO_SPAN);
      yymsp[-4].minor.yy213 =
          synq_parse_expr_list(pCtx, yymsp[-4].minor.yy213, col);
    } break;
    case 137: /* with ::= */
    {
      yymsp[1].minor.yy465.cte_list = SYNTAQLITE_NULL_NODE;
      yymsp[1].minor.yy465.is_recursive = 0;
    } break;
    case 138: /* with ::= WITH wqlist */
    {
      yymsp[-1].minor.yy465.cte_list = yymsp[0].minor.yy213;
      yymsp[-1].minor.yy465.is_recursive = 0;
    } break;
    case 139: /* with ::= WITH RECURSIVE wqlist */
    {
      yymsp[-2].minor.yy465.cte_list = yymsp[0].minor.yy213;
      yymsp[-2].minor.yy465.is_recursive = 1;
    } break;
    case 140: /* cmd ::= with DELETE FROM xfullname indexed_opt where_opt_ret
                 orderby_opt limit_opt */
    {
      (void)yymsp[-3].minor.yy0;
      if (yymsp[-1].minor.yy213 != SYNTAQLITE_NULL_NODE ||
          yymsp[0].minor.yy213 != SYNTAQLITE_NULL_NODE) {
        pCtx->saw_update_delete_limit = 1;
        if (!SYNQ_HAS_CFLAG(pCtx->config,
                            SYNQ_CFLAG_IDX_ENABLE_UPDATE_DELETE_LIMIT)) {
          pCtx->error = 1;
        }
      }
      uint32_t del = synq_parse_delete_stmt(
          pCtx, yymsp[-4].minor.yy213, yymsp[-2].minor.yy213,
          yymsp[-1].minor.yy213, yymsp[0].minor.yy213);
      if (yymsp[-7].minor.yy465.cte_list != SYNTAQLITE_NULL_NODE) {
        yylhsminor.yy213 =
            synq_parse_with_clause(pCtx, yymsp[-7].minor.yy465.is_recursive,
                                   yymsp[-7].minor.yy465.cte_list, del);
      } else {
        yylhsminor.yy213 = del;
      }
    }
      yymsp[-7].minor.yy213 = yylhsminor.yy213;
      break;
    case 141: /* cmd ::= with UPDATE orconf xfullname indexed_opt SET setlist
                 from where_opt_ret orderby_opt limit_opt */
    {
      (void)yymsp[-6].minor.yy0;
      if (yymsp[-1].minor.yy213 != SYNTAQLITE_NULL_NODE ||
          yymsp[0].minor.yy213 != SYNTAQLITE_NULL_NODE) {
        pCtx->saw_update_delete_limit = 1;
        if (!SYNQ_HAS_CFLAG(pCtx->config,
                            SYNQ_CFLAG_IDX_ENABLE_UPDATE_DELETE_LIMIT)) {
          pCtx->error = 1;
        }
      }
      uint32_t upd = synq_parse_update_stmt(
          pCtx, (SyntaqliteConflictAction)yymsp[-8].minor.yy220,
          yymsp[-7].minor.yy213, yymsp[-4].minor.yy213, yymsp[-3].minor.yy213,
          yymsp[-2].minor.yy213, yymsp[-1].minor.yy213, yymsp[0].minor.yy213);
      if (yymsp[-10].minor.yy465.cte_list != SYNTAQLITE_NULL_NODE) {
        yylhsminor.yy213 =
            synq_parse_with_clause(pCtx, yymsp[-10].minor.yy465.is_recursive,
                                   yymsp[-10].minor.yy465.cte_list, upd);
      } else {
        yylhsminor.yy213 = upd;
      }
    }
      yymsp[-10].minor.yy213 = yylhsminor.yy213;
      break;
    case 142: /* cmd ::= with insert_cmd INTO xfullname idlist_opt select upsert
               */
    {
      (void)yymsp[0].minor.yy213;
      uint32_t ins = synq_parse_insert_stmt(
          pCtx, (SyntaqliteConflictAction)yymsp[-5].minor.yy220,
          yymsp[-3].minor.yy213, yymsp[-2].minor.yy213, yymsp[-1].minor.yy213);
      if (yymsp[-6].minor.yy465.cte_list != SYNTAQLITE_NULL_NODE) {
        yylhsminor.yy213 =
            synq_parse_with_clause(pCtx, yymsp[-6].minor.yy465.is_recursive,
                                   yymsp[-6].minor.yy465.cte_list, ins);
      } else {
        yylhsminor.yy213 = ins;
      }
    }
      yymsp[-6].minor.yy213 = yylhsminor.yy213;
      break;
    case 143: /* cmd ::= with insert_cmd INTO xfullname idlist_opt DEFAULT
                 VALUES returning */
    {
      uint32_t ins = synq_parse_insert_stmt(
          pCtx, (SyntaqliteConflictAction)yymsp[-6].minor.yy220,
          yymsp[-4].minor.yy213, yymsp[-3].minor.yy213, SYNTAQLITE_NULL_NODE);
      if (yymsp[-7].minor.yy465.cte_list != SYNTAQLITE_NULL_NODE) {
        yylhsminor.yy213 =
            synq_parse_with_clause(pCtx, yymsp[-7].minor.yy465.is_recursive,
                                   yymsp[-7].minor.yy465.cte_list, ins);
      } else {
        yylhsminor.yy213 = ins;
      }
    }
      yymsp[-7].minor.yy213 = yylhsminor.yy213;
      break;
    case 145: /* insert_cmd ::= REPLACE */
    case 150: /* resolvetype ::= REPLACE */
      yytestcase(yyruleno == 150);
      {
        yymsp[0].minor.yy220 = (int)SYNTAQLITE_CONFLICT_ACTION_REPLACE;
      }
      break;
    case 148: /* resolvetype ::= raisetype */
    {
      // raisetype: ROLLBACK=1, ABORT=2, FAIL=3 (SynqRaiseType enum values)
      // ConflictAction: ROLLBACK=1, ABORT=2, FAIL=3 (same values, direct
      // passthrough)
      yylhsminor.yy220 = yymsp[0].minor.yy220;
    }
      yymsp[0].minor.yy220 = yylhsminor.yy220;
      break;
    case 149: /* resolvetype ::= IGNORE */
    {
      yymsp[0].minor.yy220 = (int)SYNTAQLITE_CONFLICT_ACTION_IGNORE;
    } break;
    case 151: /* xfullname ::= nm */
    {
      yylhsminor.yy213 =
          synq_parse_table_ref(pCtx, synq_span(pCtx, yymsp[0].minor.yy0),
                               SYNQ_NO_SPAN, SYNQ_NO_SPAN);
    }
      yymsp[0].minor.yy213 = yylhsminor.yy213;
      break;
    case 152: /* xfullname ::= nm DOT nm */
    {
      yylhsminor.yy213 = synq_parse_table_ref(
          pCtx, synq_span(pCtx, yymsp[0].minor.yy0),
          synq_span(pCtx, yymsp[-2].minor.yy0), SYNQ_NO_SPAN);
    }
      yymsp[-2].minor.yy213 = yylhsminor.yy213;
      break;
    case 153: /* xfullname ::= nm DOT nm AS nm */
    {
      yylhsminor.yy213 =
          synq_parse_table_ref(pCtx, synq_span(pCtx, yymsp[-2].minor.yy0),
                               synq_span(pCtx, yymsp[-4].minor.yy0),
                               synq_span(pCtx, yymsp[0].minor.yy0));
    }
      yymsp[-4].minor.yy213 = yylhsminor.yy213;
      break;
    case 154: /* xfullname ::= nm AS nm */
    {
      yylhsminor.yy213 = synq_parse_table_ref(
          pCtx, synq_span(pCtx, yymsp[-2].minor.yy0), SYNQ_NO_SPAN,
          synq_span(pCtx, yymsp[0].minor.yy0));
    }
      yymsp[-2].minor.yy213 = yylhsminor.yy213;
      break;
    case 156: /* indexed_opt ::= indexed_by */
    case 313: /* trnm ::= nm */
      yytestcase(yyruleno == 313);
    case 327: /* nmnum ::= plus_num */
      yytestcase(yyruleno == 327);
    case 328: /* nmnum ::= nm */
      yytestcase(yyruleno == 328);
    case 329: /* nmnum ::= ON */
      yytestcase(yyruleno == 329);
    case 330: /* nmnum ::= DELETE */
      yytestcase(yyruleno == 330);
    case 331: /* nmnum ::= DEFAULT */
      yytestcase(yyruleno == 331);
    case 333: /* plus_num ::= INTEGER|FLOAT */
      yytestcase(yyruleno == 333);
    case 335: /* signed ::= plus_num */
      yytestcase(yyruleno == 335);
    case 336: /* signed ::= minus_num */
      yytestcase(yyruleno == 336);
    case 360: /* createkw ::= CREATE */
      yytestcase(yyruleno == 360);
      {
        // Token passthrough
      }
      break;
    case 159: /* where_opt_ret ::= RETURNING selcollist */
    {
      // Ignore RETURNING clause for now (just discard the column list)
      (void)yymsp[0].minor.yy213;
      yymsp[-1].minor.yy213 = SYNTAQLITE_NULL_NODE;
    } break;
    case 160: /* where_opt_ret ::= WHERE expr RETURNING selcollist */
    {
      // Keep WHERE, ignore RETURNING
      (void)yymsp[0].minor.yy213;
      yymsp[-3].minor.yy213 = yymsp[-2].minor.yy213;
    } break;
    case 161: /* setlist ::= setlist COMMA nm EQ expr */
    {
      uint32_t clause =
          synq_parse_set_clause(pCtx, synq_span(pCtx, yymsp[-2].minor.yy0),
                                SYNTAQLITE_NULL_NODE, yymsp[0].minor.yy213);
      yylhsminor.yy213 =
          synq_parse_set_clause_list(pCtx, yymsp[-4].minor.yy213, clause);
    }
      yymsp[-4].minor.yy213 = yylhsminor.yy213;
      break;
    case 162: /* setlist ::= setlist COMMA LP idlist RP EQ expr */
    {
      uint32_t clause = synq_parse_set_clause(
          pCtx, SYNQ_NO_SPAN, yymsp[-3].minor.yy213, yymsp[0].minor.yy213);
      yylhsminor.yy213 =
          synq_parse_set_clause_list(pCtx, yymsp[-6].minor.yy213, clause);
    }
      yymsp[-6].minor.yy213 = yylhsminor.yy213;
      break;
    case 163: /* setlist ::= nm EQ expr */
    {
      uint32_t clause =
          synq_parse_set_clause(pCtx, synq_span(pCtx, yymsp[-2].minor.yy0),
                                SYNTAQLITE_NULL_NODE, yymsp[0].minor.yy213);
      yylhsminor.yy213 =
          synq_parse_set_clause_list(pCtx, SYNTAQLITE_NULL_NODE, clause);
    }
      yymsp[-2].minor.yy213 = yylhsminor.yy213;
      break;
    case 164: /* setlist ::= LP idlist RP EQ expr */
    {
      uint32_t clause = synq_parse_set_clause(
          pCtx, SYNQ_NO_SPAN, yymsp[-3].minor.yy213, yymsp[0].minor.yy213);
      yymsp[-4].minor.yy213 =
          synq_parse_set_clause_list(pCtx, SYNTAQLITE_NULL_NODE, clause);
    } break;
    case 168: /* upsert ::= RETURNING selcollist */
    {
      (void)yymsp[0].minor.yy213;
      yymsp[-1].minor.yy213 = SYNTAQLITE_NULL_NODE;
    } break;
    case 169: /* upsert ::= ON CONFLICT LP sortlist RP where_opt DO UPDATE SET
                 setlist where_opt upsert */
    {
      (void)yymsp[-8].minor.yy213;
      (void)yymsp[-6].minor.yy213;
      (void)yymsp[-2].minor.yy213;
      (void)yymsp[-1].minor.yy213;
      (void)yymsp[0].minor.yy213;
      yymsp[-11].minor.yy213 = SYNTAQLITE_NULL_NODE;
    } break;
    case 170: /* upsert ::= ON CONFLICT LP sortlist RP where_opt DO NOTHING
                 upsert */
    {
      (void)yymsp[-5].minor.yy213;
      (void)yymsp[-3].minor.yy213;
      (void)yymsp[0].minor.yy213;
      yymsp[-8].minor.yy213 = SYNTAQLITE_NULL_NODE;
    } break;
    case 171: /* upsert ::= ON CONFLICT DO NOTHING returning */
    {
      yymsp[-4].minor.yy213 = SYNTAQLITE_NULL_NODE;
    } break;
    case 172: /* upsert ::= ON CONFLICT DO UPDATE SET setlist where_opt
                 returning */
    {
      (void)yymsp[-2].minor.yy213;
      (void)yymsp[-1].minor.yy213;
      yymsp[-7].minor.yy213 = SYNTAQLITE_NULL_NODE;
    } break;
    case 173: /* returning ::= RETURNING selcollist */
    {
      (void)yymsp[0].minor.yy213;
    } break;
    case 174: /* returning ::= */
    case 307: /* foreach_clause ::= */
      yytestcase(yyruleno == 307);
    case 315: /* tridxby ::= */
      yytestcase(yyruleno == 315);
    case 373: /* vtabarg ::= */
      yytestcase(yyruleno == 373);
    case 378: /* anylist ::= */
      yytestcase(yyruleno == 378);
      {
        // empty
      }
      break;
    case 177: /* expr ::= expr PLUS|MINUS expr */
    {
      SyntaqliteBinaryOp op = (yymsp[-1].minor.yy0.type == SYNTAQLITE_TK_PLUS)
                                  ? SYNTAQLITE_BINARY_OP_PLUS
                                  : SYNTAQLITE_BINARY_OP_MINUS;
      yylhsminor.yy213 = synq_parse_binary_expr(pCtx, op, yymsp[-2].minor.yy213,
                                                yymsp[0].minor.yy213);
    }
      yymsp[-2].minor.yy213 = yylhsminor.yy213;
      break;
    case 178: /* expr ::= expr STAR|SLASH|REM expr */
    {
      SyntaqliteBinaryOp op;
      switch (yymsp[-1].minor.yy0.type) {
        case SYNTAQLITE_TK_STAR:
          op = SYNTAQLITE_BINARY_OP_STAR;
          break;
        case SYNTAQLITE_TK_SLASH:
          op = SYNTAQLITE_BINARY_OP_SLASH;
          break;
        default:
          op = SYNTAQLITE_BINARY_OP_REM;
          break;
      }
      yylhsminor.yy213 = synq_parse_binary_expr(pCtx, op, yymsp[-2].minor.yy213,
                                                yymsp[0].minor.yy213);
    }
      yymsp[-2].minor.yy213 = yylhsminor.yy213;
      break;
    case 179: /* expr ::= expr LT|GT|GE|LE expr */
    {
      SyntaqliteBinaryOp op;
      switch (yymsp[-1].minor.yy0.type) {
        case SYNTAQLITE_TK_LT:
          op = SYNTAQLITE_BINARY_OP_LT;
          break;
        case SYNTAQLITE_TK_GT:
          op = SYNTAQLITE_BINARY_OP_GT;
          break;
        case SYNTAQLITE_TK_LE:
          op = SYNTAQLITE_BINARY_OP_LE;
          break;
        default:
          op = SYNTAQLITE_BINARY_OP_GE;
          break;
      }
      yylhsminor.yy213 = synq_parse_binary_expr(pCtx, op, yymsp[-2].minor.yy213,
                                                yymsp[0].minor.yy213);
    }
      yymsp[-2].minor.yy213 = yylhsminor.yy213;
      break;
    case 180: /* expr ::= expr EQ|NE expr */
    {
      SyntaqliteBinaryOp op = (yymsp[-1].minor.yy0.type == SYNTAQLITE_TK_EQ)
                                  ? SYNTAQLITE_BINARY_OP_EQ
                                  : SYNTAQLITE_BINARY_OP_NE;
      yylhsminor.yy213 = synq_parse_binary_expr(pCtx, op, yymsp[-2].minor.yy213,
                                                yymsp[0].minor.yy213);
    }
      yymsp[-2].minor.yy213 = yylhsminor.yy213;
      break;
    case 181: /* expr ::= expr AND expr */
    {
      yylhsminor.yy213 =
          synq_parse_binary_expr(pCtx, SYNTAQLITE_BINARY_OP_AND,
                                 yymsp[-2].minor.yy213, yymsp[0].minor.yy213);
    }
      yymsp[-2].minor.yy213 = yylhsminor.yy213;
      break;
    case 182: /* expr ::= expr OR expr */
    {
      yylhsminor.yy213 =
          synq_parse_binary_expr(pCtx, SYNTAQLITE_BINARY_OP_OR,
                                 yymsp[-2].minor.yy213, yymsp[0].minor.yy213);
    }
      yymsp[-2].minor.yy213 = yylhsminor.yy213;
      break;
    case 183: /* expr ::= expr BITAND|BITOR|LSHIFT|RSHIFT expr */
    {
      SyntaqliteBinaryOp op;
      switch (yymsp[-1].minor.yy0.type) {
        case SYNTAQLITE_TK_BITAND:
          op = SYNTAQLITE_BINARY_OP_BITAND;
          break;
        case SYNTAQLITE_TK_BITOR:
          op = SYNTAQLITE_BINARY_OP_BITOR;
          break;
        case SYNTAQLITE_TK_LSHIFT:
          op = SYNTAQLITE_BINARY_OP_LSHIFT;
          break;
        default:
          op = SYNTAQLITE_BINARY_OP_RSHIFT;
          break;
      }
      yylhsminor.yy213 = synq_parse_binary_expr(pCtx, op, yymsp[-2].minor.yy213,
                                                yymsp[0].minor.yy213);
    }
      yymsp[-2].minor.yy213 = yylhsminor.yy213;
      break;
    case 184: /* expr ::= expr CONCAT expr */
    {
      yylhsminor.yy213 =
          synq_parse_binary_expr(pCtx, SYNTAQLITE_BINARY_OP_CONCAT,
                                 yymsp[-2].minor.yy213, yymsp[0].minor.yy213);
    }
      yymsp[-2].minor.yy213 = yylhsminor.yy213;
      break;
    case 185: /* expr ::= expr PTR expr */
    {
      yylhsminor.yy213 =
          synq_parse_binary_expr(pCtx, SYNTAQLITE_BINARY_OP_PTR,
                                 yymsp[-2].minor.yy213, yymsp[0].minor.yy213);
    }
      yymsp[-2].minor.yy213 = yylhsminor.yy213;
      break;
    case 186: /* expr ::= PLUS|MINUS expr */
    {
      SyntaqliteUnaryOp op = (yymsp[-1].minor.yy0.type == SYNTAQLITE_TK_MINUS)
                                 ? SYNTAQLITE_UNARY_OP_MINUS
                                 : SYNTAQLITE_UNARY_OP_PLUS;
      yylhsminor.yy213 = synq_parse_unary_expr(pCtx, op, yymsp[0].minor.yy213);
    }
      yymsp[-1].minor.yy213 = yylhsminor.yy213;
      break;
    case 187: /* expr ::= BITNOT expr */
    {
      yymsp[-1].minor.yy213 = synq_parse_unary_expr(
          pCtx, SYNTAQLITE_UNARY_OP_BITNOT, yymsp[0].minor.yy213);
    } break;
    case 188: /* expr ::= NOT expr */
    {
      yymsp[-1].minor.yy213 = synq_parse_unary_expr(
          pCtx, SYNTAQLITE_UNARY_OP_NOT, yymsp[0].minor.yy213);
    } break;
    case 191: /* nexprlist ::= nexprlist COMMA expr */
    {
      yylhsminor.yy213 = synq_parse_expr_list(pCtx, yymsp[-2].minor.yy213,
                                              yymsp[0].minor.yy213);
    }
      yymsp[-2].minor.yy213 = yylhsminor.yy213;
      break;
    case 192: /* nexprlist ::= expr */
    {
      yylhsminor.yy213 = synq_parse_expr_list(pCtx, SYNTAQLITE_NULL_NODE,
                                              yymsp[0].minor.yy213);
    }
      yymsp[0].minor.yy213 = yylhsminor.yy213;
      break;
    case 193: /* expr ::= LP nexprlist COMMA expr RP */
    {
      yymsp[-4].minor.yy213 = synq_parse_expr_list(pCtx, yymsp[-3].minor.yy213,
                                                   yymsp[-1].minor.yy213);
    } break;
    case 194: /* expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist RP */
    {
      synq_mark_as_function(pCtx, yymsp[-4].minor.yy0);
      yylhsminor.yy213 = synq_parse_function_call(
          pCtx, synq_span(pCtx, yymsp[-4].minor.yy0),
          (SyntaqliteFunctionCallFlags){.raw = (uint8_t)yymsp[-2].minor.yy213},
          yymsp[-1].minor.yy213, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE);
    }
      yymsp[-4].minor.yy213 = yylhsminor.yy213;
      break;
    case 195: /* expr ::= ID|INDEXED|JOIN_KW LP STAR RP */
    {
      synq_mark_as_function(pCtx, yymsp[-3].minor.yy0);
      yylhsminor.yy213 = synq_parse_function_call(
          pCtx, synq_span(pCtx, yymsp[-3].minor.yy0),
          (SyntaqliteFunctionCallFlags){.bits = {.star = 1}},
          SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE);
    }
      yymsp[-3].minor.yy213 = yylhsminor.yy213;
      break;
    case 196: /* expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist RP filter_over
               */
    {
      SyntaqliteFilterOver* fo = (SyntaqliteFilterOver*)synq_arena_ptr(
          &pCtx->ast, yymsp[0].minor.yy213);
      synq_mark_as_function(pCtx, yymsp[-5].minor.yy0);
      yylhsminor.yy213 = synq_parse_function_call(
          pCtx, synq_span(pCtx, yymsp[-5].minor.yy0),
          (SyntaqliteFunctionCallFlags){.raw = (uint8_t)yymsp[-3].minor.yy213},
          yymsp[-2].minor.yy213, fo->filter_expr, fo->over_def);
    }
      yymsp[-5].minor.yy213 = yylhsminor.yy213;
      break;
    case 197: /* expr ::= ID|INDEXED|JOIN_KW LP STAR RP filter_over */
    {
      SyntaqliteFilterOver* fo = (SyntaqliteFilterOver*)synq_arena_ptr(
          &pCtx->ast, yymsp[0].minor.yy213);
      synq_mark_as_function(pCtx, yymsp[-4].minor.yy0);
      yylhsminor.yy213 = synq_parse_function_call(
          pCtx, synq_span(pCtx, yymsp[-4].minor.yy0),
          (SyntaqliteFunctionCallFlags){.bits = {.star = 1}},
          SYNTAQLITE_NULL_NODE, fo->filter_expr, fo->over_def);
    }
      yymsp[-4].minor.yy213 = yylhsminor.yy213;
      break;
    case 198: /* nm ::= ID|INDEXED|JOIN_KW */
    {
      synq_mark_as_id(pCtx, yymsp[0].minor.yy0);
      yylhsminor.yy0 = yymsp[0].minor.yy0;
    }
      yymsp[0].minor.yy0 = yylhsminor.yy0;
      break;
    case 200: /* term ::= INTEGER */
    {
      yylhsminor.yy213 =
          synq_parse_literal(pCtx, SYNTAQLITE_LITERAL_TYPE_INTEGER,
                             synq_span(pCtx, yymsp[0].minor.yy0));
    }
      yymsp[0].minor.yy213 = yylhsminor.yy213;
      break;
    case 201: /* term ::= STRING */
    {
      yylhsminor.yy213 =
          synq_parse_literal(pCtx, SYNTAQLITE_LITERAL_TYPE_STRING,
                             synq_span(pCtx, yymsp[0].minor.yy0));
    }
      yymsp[0].minor.yy213 = yylhsminor.yy213;
      break;
    case 202: /* term ::= NULL|FLOAT|BLOB */
    {
      SyntaqliteLiteralType lit_type;
      switch (yymsp[0].minor.yy0.type) {
        case SYNTAQLITE_TK_NULL:
          lit_type = SYNTAQLITE_LITERAL_TYPE_NULL;
          break;
        case SYNTAQLITE_TK_FLOAT:
          lit_type = SYNTAQLITE_LITERAL_TYPE_FLOAT;
          break;
        case SYNTAQLITE_TK_BLOB:
          lit_type = SYNTAQLITE_LITERAL_TYPE_BLOB;
          break;
        default:
          lit_type = SYNTAQLITE_LITERAL_TYPE_NULL;
          break;
      }
      yylhsminor.yy213 = synq_parse_literal(
          pCtx, lit_type, synq_span(pCtx, yymsp[0].minor.yy0));
    }
      yymsp[0].minor.yy213 = yylhsminor.yy213;
      break;
    case 203: /* term ::= QNUMBER */
    {
      yylhsminor.yy213 =
          synq_parse_literal(pCtx, SYNTAQLITE_LITERAL_TYPE_QNUMBER,
                             synq_span(pCtx, yymsp[0].minor.yy0));
    }
      yymsp[0].minor.yy213 = yylhsminor.yy213;
      break;
    case 204: /* term ::= CTIME_KW */
    {
      yylhsminor.yy213 =
          synq_parse_literal(pCtx, SYNTAQLITE_LITERAL_TYPE_CURRENT,
                             synq_span(pCtx, yymsp[0].minor.yy0));
    }
      yymsp[0].minor.yy213 = yylhsminor.yy213;
      break;
    case 205: /* expr ::= VARIABLE */
    {
      yylhsminor.yy213 =
          synq_parse_variable(pCtx, synq_span(pCtx, yymsp[0].minor.yy0));
    }
      yymsp[0].minor.yy213 = yylhsminor.yy213;
      break;
    case 206: /* expr ::= expr COLLATE ID|STRING */
    {
      yylhsminor.yy213 = synq_parse_collate_expr(
          pCtx, yymsp[-2].minor.yy213, synq_span(pCtx, yymsp[0].minor.yy0));
    }
      yymsp[-2].minor.yy213 = yylhsminor.yy213;
      break;
    case 207: /* sortlist ::= sortlist COMMA expr sortorder nulls */
    {
      uint32_t term =
          synq_parse_ordering_term(pCtx, yymsp[-2].minor.yy213,
                                   (SyntaqliteSortOrder)yymsp[-1].minor.yy213,
                                   (SyntaqliteNullsOrder)yymsp[0].minor.yy213);
      yylhsminor.yy213 =
          synq_parse_order_by_list(pCtx, yymsp[-4].minor.yy213, term);
    }
      yymsp[-4].minor.yy213 = yylhsminor.yy213;
      break;
    case 208: /* sortlist ::= expr sortorder nulls */
    {
      uint32_t term =
          synq_parse_ordering_term(pCtx, yymsp[-2].minor.yy213,
                                   (SyntaqliteSortOrder)yymsp[-1].minor.yy213,
                                   (SyntaqliteNullsOrder)yymsp[0].minor.yy213);
      yylhsminor.yy213 =
          synq_parse_order_by_list(pCtx, SYNTAQLITE_NULL_NODE, term);
    }
      yymsp[-2].minor.yy213 = yylhsminor.yy213;
      break;
    case 210: /* sortorder ::= DESC */
    case 264: /* distinct ::= DISTINCT */
      yytestcase(yyruleno == 264);
      {
        yymsp[0].minor.yy213 = 1;
      }
      break;
    case 211: /* sortorder ::= */
    case 214: /* nulls ::= */
      yytestcase(yyruleno == 214);
    case 266: /* distinct ::= */
      yytestcase(yyruleno == 266);
      {
        yymsp[1].minor.yy213 = 0;
      }
      break;
    case 213: /* nulls ::= NULLS LAST */
    {
      yymsp[-1].minor.yy213 = 2;
    } break;
    case 215: /* expr ::= RAISE LP IGNORE RP */
    {
      yymsp[-3].minor.yy213 = synq_parse_raise_expr(
          pCtx, SYNTAQLITE_RAISE_TYPE_IGNORE, SYNTAQLITE_NULL_NODE);
    } break;
    case 216: /* expr ::= RAISE LP raisetype COMMA expr RP */
    {
      yymsp[-5].minor.yy213 = synq_parse_raise_expr(
          pCtx, (SyntaqliteRaiseType)yymsp[-3].minor.yy220,
          yymsp[-1].minor.yy213);
    } break;
    case 217: /* raisetype ::= ROLLBACK */
    {
      yymsp[0].minor.yy220 = SYNTAQLITE_RAISE_TYPE_ROLLBACK;
    } break;
    case 218: /* raisetype ::= ABORT */
    {
      yymsp[0].minor.yy220 = SYNTAQLITE_RAISE_TYPE_ABORT;
    } break;
    case 219: /* raisetype ::= FAIL */
    {
      yymsp[0].minor.yy220 = SYNTAQLITE_RAISE_TYPE_FAIL;
    } break;
    case 220: /* fullname ::= nm */
    {
      yylhsminor.yy213 = synq_parse_qualified_name(
          pCtx, synq_span(pCtx, yymsp[0].minor.yy0), SYNQ_NO_SPAN);
    }
      yymsp[0].minor.yy213 = yylhsminor.yy213;
      break;
    case 221: /* fullname ::= nm DOT nm */
    {
      yylhsminor.yy213 =
          synq_parse_qualified_name(pCtx, synq_span(pCtx, yymsp[0].minor.yy0),
                                    synq_span(pCtx, yymsp[-2].minor.yy0));
    }
      yymsp[-2].minor.yy213 = yylhsminor.yy213;
      break;
    case 224: /* cmd ::= DROP TABLE ifexists fullname */
    {
      yymsp[-3].minor.yy213 = synq_parse_drop_stmt(
          pCtx, SYNTAQLITE_DROP_OBJECT_TYPE_TABLE,
          (SyntaqliteBool)yymsp[-1].minor.yy220, yymsp[0].minor.yy213);
    } break;
    case 225: /* cmd ::= DROP VIEW ifexists fullname */
    {
      yymsp[-3].minor.yy213 = synq_parse_drop_stmt(
          pCtx, SYNTAQLITE_DROP_OBJECT_TYPE_VIEW,
          (SyntaqliteBool)yymsp[-1].minor.yy220, yymsp[0].minor.yy213);
    } break;
    case 226: /* cmd ::= DROP INDEX ifexists fullname */
    {
      yymsp[-3].minor.yy213 = synq_parse_drop_stmt(
          pCtx, SYNTAQLITE_DROP_OBJECT_TYPE_INDEX,
          (SyntaqliteBool)yymsp[-1].minor.yy220, yymsp[0].minor.yy213);
    } break;
    case 227: /* cmd ::= DROP TRIGGER ifexists fullname */
    {
      yymsp[-3].minor.yy213 = synq_parse_drop_stmt(
          pCtx, SYNTAQLITE_DROP_OBJECT_TYPE_TRIGGER,
          (SyntaqliteBool)yymsp[-1].minor.yy220, yymsp[0].minor.yy213);
    } break;
    case 228: /* cmd ::= ALTER TABLE fullname RENAME TO nm */
    {
      yymsp[-5].minor.yy213 = synq_parse_alter_table_stmt(
          pCtx, SYNTAQLITE_ALTER_OP_RENAME_TABLE, yymsp[-3].minor.yy213,
          synq_span(pCtx, yymsp[0].minor.yy0), SYNQ_NO_SPAN);
    } break;
    case 229: /* cmd ::= ALTER TABLE fullname RENAME kwcolumn_opt nm TO nm */
    {
      yymsp[-7].minor.yy213 = synq_parse_alter_table_stmt(
          pCtx, SYNTAQLITE_ALTER_OP_RENAME_COLUMN, yymsp[-5].minor.yy213,
          synq_span(pCtx, yymsp[0].minor.yy0),
          synq_span(pCtx, yymsp[-2].minor.yy0));
    } break;
    case 230: /* cmd ::= ALTER TABLE fullname DROP kwcolumn_opt nm */
    {
      yymsp[-5].minor.yy213 = synq_parse_alter_table_stmt(
          pCtx, SYNTAQLITE_ALTER_OP_DROP_COLUMN, yymsp[-3].minor.yy213,
          SYNQ_NO_SPAN, synq_span(pCtx, yymsp[0].minor.yy0));
    } break;
    case 231: /* cmd ::= ALTER TABLE add_column_fullname ADD kwcolumn_opt
                 columnname carglist */
    {
      yymsp[-6].minor.yy213 = synq_parse_alter_table_stmt(
          pCtx, SYNTAQLITE_ALTER_OP_ADD_COLUMN, SYNTAQLITE_NULL_NODE,
          SYNQ_NO_SPAN, yymsp[-1].minor.yy400.name);
    } break;
    case 232: /* add_column_fullname ::= fullname */
    {
      // Passthrough - fullname already produces a node ID but we don't need it
      // for the ADD COLUMN action since add_column_fullname is consumed by cmd
    } break;
    case 235: /* columnname ::= nm typetoken */
    {
      yylhsminor.yy400.name = synq_span(pCtx, yymsp[-1].minor.yy0);
      yylhsminor.yy400.typetoken = yymsp[0].minor.yy0.z
                                       ? synq_span(pCtx, yymsp[0].minor.yy0)
                                       : SYNQ_NO_SPAN;
    }
      yymsp[-1].minor.yy400 = yylhsminor.yy400;
      break;
    case 236: /* cmd ::= BEGIN transtype trans_opt */
    {
      yymsp[-2].minor.yy213 = synq_parse_transaction_stmt(
          pCtx, SYNTAQLITE_TRANSACTION_OP_BEGIN,
          (SyntaqliteTransactionType)yymsp[-1].minor.yy220);
    } break;
    case 237: /* cmd ::= COMMIT|END trans_opt */
    {
      yymsp[-1].minor.yy213 =
          synq_parse_transaction_stmt(pCtx, SYNTAQLITE_TRANSACTION_OP_COMMIT,
                                      SYNTAQLITE_TRANSACTION_TYPE_DEFERRED);
    } break;
    case 238: /* cmd ::= ROLLBACK trans_opt */
    {
      yymsp[-1].minor.yy213 =
          synq_parse_transaction_stmt(pCtx, SYNTAQLITE_TRANSACTION_OP_ROLLBACK,
                                      SYNTAQLITE_TRANSACTION_TYPE_DEFERRED);
    } break;
    case 239: /* transtype ::= */
    {
      yymsp[1].minor.yy220 = (int)SYNTAQLITE_TRANSACTION_TYPE_DEFERRED;
    } break;
    case 240: /* transtype ::= DEFERRED */
    {
      yymsp[0].minor.yy220 = (int)SYNTAQLITE_TRANSACTION_TYPE_DEFERRED;
    } break;
    case 241: /* transtype ::= IMMEDIATE */
    {
      yymsp[0].minor.yy220 = (int)SYNTAQLITE_TRANSACTION_TYPE_IMMEDIATE;
    } break;
    case 242: /* transtype ::= EXCLUSIVE */
    {
      yymsp[0].minor.yy220 = (int)SYNTAQLITE_TRANSACTION_TYPE_EXCLUSIVE;
    } break;
    case 244: /* trans_opt ::= TRANSACTION */
    case 246: /* savepoint_opt ::= SAVEPOINT */
      yytestcase(yyruleno == 246);
      {
        yymsp[0].minor.yy220 = 0;
      }
      break;
    case 248: /* cmd ::= SAVEPOINT nm */
    {
      yymsp[-1].minor.yy213 =
          synq_parse_savepoint_stmt(pCtx, SYNTAQLITE_SAVEPOINT_OP_SAVEPOINT,
                                    synq_span(pCtx, yymsp[0].minor.yy0));
    } break;
    case 249: /* cmd ::= RELEASE savepoint_opt nm */
    {
      yymsp[-2].minor.yy213 =
          synq_parse_savepoint_stmt(pCtx, SYNTAQLITE_SAVEPOINT_OP_RELEASE,
                                    synq_span(pCtx, yymsp[0].minor.yy0));
    } break;
    case 250: /* cmd ::= ROLLBACK trans_opt TO savepoint_opt nm */
    {
      yymsp[-4].minor.yy213 =
          synq_parse_savepoint_stmt(pCtx, SYNTAQLITE_SAVEPOINT_OP_ROLLBACK_TO,
                                    synq_span(pCtx, yymsp[0].minor.yy0));
    } break;
    case 254: /* oneselect ::= SELECT distinct selcollist from where_opt
                 groupby_opt having_opt orderby_opt limit_opt */
    {
      yymsp[-8].minor.yy213 = synq_parse_select_stmt(
          pCtx,
          (SyntaqliteSelectStmtFlags){.raw = (uint8_t)yymsp[-7].minor.yy213},
          yymsp[-6].minor.yy213, yymsp[-5].minor.yy213, yymsp[-4].minor.yy213,
          yymsp[-3].minor.yy213, yymsp[-2].minor.yy213, yymsp[-1].minor.yy213,
          yymsp[0].minor.yy213, SYNTAQLITE_NULL_NODE);
    } break;
    case 255: /* oneselect ::= SELECT distinct selcollist from where_opt
                 groupby_opt having_opt window_clause orderby_opt limit_opt */
    {
      yymsp[-9].minor.yy213 = synq_parse_select_stmt(
          pCtx,
          (SyntaqliteSelectStmtFlags){.raw = (uint8_t)yymsp[-8].minor.yy213},
          yymsp[-7].minor.yy213, yymsp[-6].minor.yy213, yymsp[-5].minor.yy213,
          yymsp[-4].minor.yy213, yymsp[-3].minor.yy213, yymsp[-1].minor.yy213,
          yymsp[0].minor.yy213, yymsp[-2].minor.yy213);
    } break;
    case 256: /* selcollist ::= sclp scanpt expr scanpt as */
    {
      SyntaqliteSourceSpan alias = (yymsp[0].minor.yy0.z)
                                       ? synq_span(pCtx, yymsp[0].minor.yy0)
                                       : SYNQ_NO_SPAN;
      uint32_t col = synq_parse_result_column(
          pCtx, (SyntaqliteResultColumnFlags){0}, alias, yymsp[-2].minor.yy213);
      yylhsminor.yy213 =
          synq_parse_result_column_list(pCtx, yymsp[-4].minor.yy213, col);
    }
      yymsp[-4].minor.yy213 = yylhsminor.yy213;
      break;
    case 257: /* selcollist ::= sclp scanpt STAR */
    {
      uint32_t col = synq_parse_result_column(
          pCtx, (SyntaqliteResultColumnFlags){.bits = {.star = 1}},
          SYNQ_NO_SPAN, SYNTAQLITE_NULL_NODE);
      yylhsminor.yy213 =
          synq_parse_result_column_list(pCtx, yymsp[-2].minor.yy213, col);
    }
      yymsp[-2].minor.yy213 = yylhsminor.yy213;
      break;
    case 258: /* sclp ::= selcollist COMMA */
    {
      yylhsminor.yy213 = yymsp[-1].minor.yy213;
    }
      yymsp[-1].minor.yy213 = yylhsminor.yy213;
      break;
    case 261: /* as ::= AS nm */
    case 332: /* plus_num ::= PLUS INTEGER|FLOAT */
      yytestcase(yyruleno == 332);
      {
        yymsp[-1].minor.yy0 = yymsp[0].minor.yy0;
      }
      break;
    case 272: /* groupby_opt ::= GROUP BY nexprlist */
    case 276: /* orderby_opt ::= ORDER BY sortlist */
      yytestcase(yyruleno == 276);
      {
        yymsp[-2].minor.yy213 = yymsp[0].minor.yy213;
      }
      break;
    case 278: /* limit_opt ::= LIMIT expr */
    {
      yymsp[-1].minor.yy213 = synq_parse_limit_clause(
          pCtx, yymsp[0].minor.yy213, SYNTAQLITE_NULL_NODE);
    } break;
    case 279: /* limit_opt ::= LIMIT expr OFFSET expr */
    {
      yymsp[-3].minor.yy213 = synq_parse_limit_clause(
          pCtx, yymsp[-2].minor.yy213, yymsp[0].minor.yy213);
    } break;
    case 280: /* limit_opt ::= LIMIT expr COMMA expr */
    {
      yymsp[-3].minor.yy213 = synq_parse_limit_clause(
          pCtx, yymsp[0].minor.yy213, yymsp[-2].minor.yy213);
    } break;
    case 281: /* stl_prefix ::= seltablist joinop */
    {
      yymsp[-1].minor.yy213 =
          synq_parse_join_prefix(pCtx, yymsp[-1].minor.yy213,
                                 (SyntaqliteJoinType)yymsp[0].minor.yy220);
    } break;
    case 283: /* seltablist ::= stl_prefix nm dbnm as on_using */
    {
      SyntaqliteSourceSpan alias = (yymsp[-1].minor.yy0.z != NULL)
                                       ? synq_span(pCtx, yymsp[-1].minor.yy0)
                                       : SYNQ_NO_SPAN;
      SyntaqliteSourceSpan table_name;
      SyntaqliteSourceSpan schema;
      if (yymsp[-2].minor.yy0.z != NULL) {
        table_name = synq_span(pCtx, yymsp[-2].minor.yy0);
        schema = synq_span(pCtx, yymsp[-3].minor.yy0);
      } else {
        table_name = synq_span(pCtx, yymsp[-3].minor.yy0);
        schema = SYNQ_NO_SPAN;
      }
      uint32_t tref = synq_parse_table_ref(pCtx, table_name, schema, alias);
      if (yymsp[-4].minor.yy213 == SYNTAQLITE_NULL_NODE) {
        yymsp[-4].minor.yy213 = tref;
      } else {
        SyntaqliteNode* pfx = AST_NODE(&pCtx->ast, yymsp[-4].minor.yy213);
        yymsp[-4].minor.yy213 = synq_parse_join_clause(
            pCtx, pfx->join_prefix.join_type, pfx->join_prefix.source, tref,
            yymsp[0].minor.yy304.on_expr, yymsp[0].minor.yy304.using_cols);
      }
    } break;
    case 284: /* seltablist ::= stl_prefix nm dbnm as indexed_by on_using */
    {
      (void)yymsp[-1].minor.yy0;
      SyntaqliteSourceSpan alias = (yymsp[-2].minor.yy0.z != NULL)
                                       ? synq_span(pCtx, yymsp[-2].minor.yy0)
                                       : SYNQ_NO_SPAN;
      SyntaqliteSourceSpan table_name;
      SyntaqliteSourceSpan schema;
      if (yymsp[-3].minor.yy0.z != NULL) {
        table_name = synq_span(pCtx, yymsp[-3].minor.yy0);
        schema = synq_span(pCtx, yymsp[-4].minor.yy0);
      } else {
        table_name = synq_span(pCtx, yymsp[-4].minor.yy0);
        schema = SYNQ_NO_SPAN;
      }
      uint32_t tref = synq_parse_table_ref(pCtx, table_name, schema, alias);
      if (yymsp[-5].minor.yy213 == SYNTAQLITE_NULL_NODE) {
        yymsp[-5].minor.yy213 = tref;
      } else {
        SyntaqliteNode* pfx = AST_NODE(&pCtx->ast, yymsp[-5].minor.yy213);
        yymsp[-5].minor.yy213 = synq_parse_join_clause(
            pCtx, pfx->join_prefix.join_type, pfx->join_prefix.source, tref,
            yymsp[0].minor.yy304.on_expr, yymsp[0].minor.yy304.using_cols);
      }
    } break;
    case 285: /* seltablist ::= stl_prefix nm dbnm LP exprlist RP as on_using */
    {
      (void)yymsp[-3].minor.yy213;
      SyntaqliteSourceSpan alias = (yymsp[-1].minor.yy0.z != NULL)
                                       ? synq_span(pCtx, yymsp[-1].minor.yy0)
                                       : SYNQ_NO_SPAN;
      SyntaqliteSourceSpan table_name;
      SyntaqliteSourceSpan schema;
      if (yymsp[-5].minor.yy0.z != NULL) {
        table_name = synq_span(pCtx, yymsp[-5].minor.yy0);
        schema = synq_span(pCtx, yymsp[-6].minor.yy0);
      } else {
        table_name = synq_span(pCtx, yymsp[-6].minor.yy0);
        schema = SYNQ_NO_SPAN;
      }
      uint32_t tref = synq_parse_table_ref(pCtx, table_name, schema, alias);
      if (yymsp[-7].minor.yy213 == SYNTAQLITE_NULL_NODE) {
        yymsp[-7].minor.yy213 = tref;
      } else {
        SyntaqliteNode* pfx = AST_NODE(&pCtx->ast, yymsp[-7].minor.yy213);
        yymsp[-7].minor.yy213 = synq_parse_join_clause(
            pCtx, pfx->join_prefix.join_type, pfx->join_prefix.source, tref,
            yymsp[0].minor.yy304.on_expr, yymsp[0].minor.yy304.using_cols);
      }
    } break;
    case 286: /* seltablist ::= stl_prefix LP select RP as on_using */
    {
      pCtx->saw_subquery = 1;
      SyntaqliteSourceSpan alias = (yymsp[-1].minor.yy0.z != NULL)
                                       ? synq_span(pCtx, yymsp[-1].minor.yy0)
                                       : SYNQ_NO_SPAN;
      uint32_t sub =
          synq_parse_subquery_table_source(pCtx, yymsp[-3].minor.yy213, alias);
      if (yymsp[-5].minor.yy213 == SYNTAQLITE_NULL_NODE) {
        yymsp[-5].minor.yy213 = sub;
      } else {
        SyntaqliteNode* pfx = AST_NODE(&pCtx->ast, yymsp[-5].minor.yy213);
        yymsp[-5].minor.yy213 = synq_parse_join_clause(
            pCtx, pfx->join_prefix.join_type, pfx->join_prefix.source, sub,
            yymsp[0].minor.yy304.on_expr, yymsp[0].minor.yy304.using_cols);
      }
    } break;
    case 287: /* seltablist ::= stl_prefix LP seltablist RP as on_using */
    {
      (void)yymsp[-1].minor.yy0;
      (void)yymsp[0].minor.yy304;
      if (yymsp[-5].minor.yy213 == SYNTAQLITE_NULL_NODE) {
        yymsp[-5].minor.yy213 = yymsp[-3].minor.yy213;
      } else {
        SyntaqliteNode* pfx = AST_NODE(&pCtx->ast, yymsp[-5].minor.yy213);
        yymsp[-5].minor.yy213 = synq_parse_join_clause(
            pCtx, pfx->join_prefix.join_type, pfx->join_prefix.source,
            yymsp[-3].minor.yy213, yymsp[0].minor.yy304.on_expr,
            yymsp[0].minor.yy304.using_cols);
      }
    } break;
    case 288: /* joinop ::= COMMA|JOIN */
    {
      yylhsminor.yy220 = (yymsp[0].minor.yy0.type == SYNTAQLITE_TK_COMMA)
                             ? (int)SYNTAQLITE_JOIN_TYPE_COMMA
                             : (int)SYNTAQLITE_JOIN_TYPE_INNER;
    }
      yymsp[0].minor.yy220 = yylhsminor.yy220;
      break;
    case 289: /* joinop ::= JOIN_KW JOIN */
    {
      // Single keyword: LEFT, RIGHT, INNER, OUTER, CROSS, NATURAL, FULL
      if (yymsp[-1].minor.yy0.n == 4 && (yymsp[-1].minor.yy0.z[0] == 'L' ||
                                         yymsp[-1].minor.yy0.z[0] == 'l')) {
        yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_LEFT;
      } else if (yymsp[-1].minor.yy0.n == 5 &&
                 (yymsp[-1].minor.yy0.z[0] == 'R' ||
                  yymsp[-1].minor.yy0.z[0] == 'r')) {
        yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_RIGHT;
      } else if (yymsp[-1].minor.yy0.n == 5 &&
                 (yymsp[-1].minor.yy0.z[0] == 'I' ||
                  yymsp[-1].minor.yy0.z[0] == 'i')) {
        yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_INNER;
      } else if (yymsp[-1].minor.yy0.n == 5 &&
                 (yymsp[-1].minor.yy0.z[0] == 'O' ||
                  yymsp[-1].minor.yy0.z[0] == 'o')) {
        // OUTER alone is not valid but treat as INNER
        yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_INNER;
      } else if (yymsp[-1].minor.yy0.n == 5 &&
                 (yymsp[-1].minor.yy0.z[0] == 'C' ||
                  yymsp[-1].minor.yy0.z[0] == 'c')) {
        yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_CROSS;
      } else if (yymsp[-1].minor.yy0.n == 7 &&
                 (yymsp[-1].minor.yy0.z[0] == 'N' ||
                  yymsp[-1].minor.yy0.z[0] == 'n')) {
        yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_NATURAL_INNER;
      } else if (yymsp[-1].minor.yy0.n == 4 &&
                 (yymsp[-1].minor.yy0.z[0] == 'F' ||
                  yymsp[-1].minor.yy0.z[0] == 'f')) {
        yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_FULL;
      } else {
        yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_INNER;
      }
    }
      yymsp[-1].minor.yy220 = yylhsminor.yy220;
      break;
    case 290: /* joinop ::= JOIN_KW nm JOIN */
    {
      // Two keywords: LEFT OUTER, NATURAL LEFT, NATURAL RIGHT, etc.
      (void)yymsp[-1].minor.yy0;
      if (yymsp[-2].minor.yy0.n == 7 && (yymsp[-2].minor.yy0.z[0] == 'N' ||
                                         yymsp[-2].minor.yy0.z[0] == 'n')) {
        // NATURAL + something
        if (yymsp[-1].minor.yy0.n == 4 && (yymsp[-1].minor.yy0.z[0] == 'L' ||
                                           yymsp[-1].minor.yy0.z[0] == 'l')) {
          yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_NATURAL_LEFT;
        } else if (yymsp[-1].minor.yy0.n == 5 &&
                   (yymsp[-1].minor.yy0.z[0] == 'R' ||
                    yymsp[-1].minor.yy0.z[0] == 'r')) {
          yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_NATURAL_RIGHT;
        } else if (yymsp[-1].minor.yy0.n == 5 &&
                   (yymsp[-1].minor.yy0.z[0] == 'I' ||
                    yymsp[-1].minor.yy0.z[0] == 'i')) {
          yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_NATURAL_INNER;
        } else if (yymsp[-1].minor.yy0.n == 4 &&
                   (yymsp[-1].minor.yy0.z[0] == 'F' ||
                    yymsp[-1].minor.yy0.z[0] == 'f')) {
          yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_NATURAL_FULL;
        } else if (yymsp[-1].minor.yy0.n == 5 &&
                   (yymsp[-1].minor.yy0.z[0] == 'C' ||
                    yymsp[-1].minor.yy0.z[0] == 'c')) {
          // NATURAL CROSS -> just CROSS
          yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_CROSS;
        } else {
          yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_NATURAL_INNER;
        }
      } else if (yymsp[-2].minor.yy0.n == 4 &&
                 (yymsp[-2].minor.yy0.z[0] == 'L' ||
                  yymsp[-2].minor.yy0.z[0] == 'l')) {
        // LEFT OUTER
        yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_LEFT;
      } else if (yymsp[-2].minor.yy0.n == 5 &&
                 (yymsp[-2].minor.yy0.z[0] == 'R' ||
                  yymsp[-2].minor.yy0.z[0] == 'r')) {
        // RIGHT OUTER
        yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_RIGHT;
      } else if (yymsp[-2].minor.yy0.n == 4 &&
                 (yymsp[-2].minor.yy0.z[0] == 'F' ||
                  yymsp[-2].minor.yy0.z[0] == 'f')) {
        // FULL OUTER
        yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_FULL;
      } else {
        yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_INNER;
      }
    }
      yymsp[-2].minor.yy220 = yylhsminor.yy220;
      break;
    case 291: /* joinop ::= JOIN_KW nm nm JOIN */
    {
      // Three keywords: NATURAL LEFT OUTER, NATURAL RIGHT OUTER, etc.
      (void)yymsp[-2].minor.yy0;
      (void)yymsp[-1].minor.yy0;
      if (yymsp[-3].minor.yy0.n == 7 && (yymsp[-3].minor.yy0.z[0] == 'N' ||
                                         yymsp[-3].minor.yy0.z[0] == 'n')) {
        // NATURAL yylhsminor.yy220 OUTER
        if (yymsp[-2].minor.yy0.n == 4 && (yymsp[-2].minor.yy0.z[0] == 'L' ||
                                           yymsp[-2].minor.yy0.z[0] == 'l')) {
          yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_NATURAL_LEFT;
        } else if (yymsp[-2].minor.yy0.n == 5 &&
                   (yymsp[-2].minor.yy0.z[0] == 'R' ||
                    yymsp[-2].minor.yy0.z[0] == 'r')) {
          yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_NATURAL_RIGHT;
        } else if (yymsp[-2].minor.yy0.n == 4 &&
                   (yymsp[-2].minor.yy0.z[0] == 'F' ||
                    yymsp[-2].minor.yy0.z[0] == 'f')) {
          yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_NATURAL_FULL;
        } else {
          yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_NATURAL_INNER;
        }
      } else {
        yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_INNER;
      }
    }
      yymsp[-3].minor.yy220 = yylhsminor.yy220;
      break;
    case 292: /* on_using ::= ON expr */
    {
      yymsp[-1].minor.yy304.on_expr = yymsp[0].minor.yy213;
      yymsp[-1].minor.yy304.using_cols = SYNTAQLITE_NULL_NODE;
    } break;
    case 293: /* on_using ::= USING LP idlist RP */
    {
      yymsp[-3].minor.yy304.on_expr = SYNTAQLITE_NULL_NODE;
      yymsp[-3].minor.yy304.using_cols = yymsp[-1].minor.yy213;
    } break;
    case 294: /* on_using ::= */
    {
      yymsp[1].minor.yy304.on_expr = SYNTAQLITE_NULL_NODE;
      yymsp[1].minor.yy304.using_cols = SYNTAQLITE_NULL_NODE;
    } break;
    case 295: /* indexed_by ::= INDEXED BY nm */
    {
      yymsp[-2].minor.yy0 = yymsp[0].minor.yy0;
    } break;
    case 296: /* indexed_by ::= NOT INDEXED */
    {
      yymsp[-1].minor.yy0.z = NULL;
      yymsp[-1].minor.yy0.n = 1;
    } break;
    case 297: /* idlist ::= idlist COMMA nm */
    {
      uint32_t col =
          synq_parse_column_ref(pCtx, synq_span(pCtx, yymsp[0].minor.yy0),
                                SYNQ_NO_SPAN, SYNQ_NO_SPAN);
      yymsp[-2].minor.yy213 =
          synq_parse_expr_list(pCtx, yymsp[-2].minor.yy213, col);
    } break;
    case 298: /* idlist ::= nm */
    {
      uint32_t col =
          synq_parse_column_ref(pCtx, synq_span(pCtx, yymsp[0].minor.yy0),
                                SYNQ_NO_SPAN, SYNQ_NO_SPAN);
      yylhsminor.yy213 = synq_parse_expr_list(pCtx, SYNTAQLITE_NULL_NODE, col);
    }
      yymsp[0].minor.yy213 = yylhsminor.yy213;
      break;
    case 299: /* cmd ::= createkw trigger_decl BEGIN trigger_cmd_list END */
    {
      // yymsp[-3].minor.yy213 is a partially-built CreateTriggerStmt, fill in
      // the body
      SyntaqliteNode* trig = AST_NODE(&pCtx->ast, yymsp[-3].minor.yy213);
      trig->create_trigger_stmt.body = yymsp[-1].minor.yy213;
      yymsp[-4].minor.yy213 = yymsp[-3].minor.yy213;
    } break;
    case 300: /* trigger_decl ::= temp TRIGGER ifnotexists nm dbnm trigger_time
                 trigger_event ON fullname foreach_clause when_clause */
    {
      SyntaqliteSourceSpan trig_name =
          yymsp[-6].minor.yy0.z ? synq_span(pCtx, yymsp[-6].minor.yy0)
                                : synq_span(pCtx, yymsp[-7].minor.yy0);
      SyntaqliteSourceSpan trig_schema =
          yymsp[-6].minor.yy0.z ? synq_span(pCtx, yymsp[-7].minor.yy0)
                                : SYNQ_NO_SPAN;
      yylhsminor.yy213 = synq_parse_create_trigger_stmt(
          pCtx, trig_name, trig_schema, (SyntaqliteBool)yymsp[-10].minor.yy220,
          (SyntaqliteBool)yymsp[-8].minor.yy220,
          (SyntaqliteTriggerTiming)yymsp[-5].minor.yy220, yymsp[-4].minor.yy213,
          yymsp[-2].minor.yy213, yymsp[0].minor.yy213,
          SYNTAQLITE_NULL_NODE);  // body filled in by cmd rule
    }
      yymsp[-10].minor.yy213 = yylhsminor.yy213;
      break;
    case 301: /* trigger_time ::= BEFORE|AFTER */
    {
      yylhsminor.yy220 = (yymsp[0].minor.yy0.type == SYNTAQLITE_TK_BEFORE)
                             ? (int)SYNTAQLITE_TRIGGER_TIMING_BEFORE
                             : (int)SYNTAQLITE_TRIGGER_TIMING_AFTER;
    }
      yymsp[0].minor.yy220 = yylhsminor.yy220;
      break;
    case 302: /* trigger_time ::= INSTEAD OF */
    {
      yymsp[-1].minor.yy220 = (int)SYNTAQLITE_TRIGGER_TIMING_INSTEAD_OF;
    } break;
    case 303: /* trigger_time ::= */
    {
      yymsp[1].minor.yy220 = (int)SYNTAQLITE_TRIGGER_TIMING_BEFORE;
    } break;
    case 304: /* trigger_event ::= DELETE|INSERT */
    {
      SyntaqliteTriggerEventType evt =
          (yymsp[0].minor.yy0.type == SYNTAQLITE_TK_DELETE)
              ? SYNTAQLITE_TRIGGER_EVENT_TYPE_DELETE
              : SYNTAQLITE_TRIGGER_EVENT_TYPE_INSERT;
      yylhsminor.yy213 =
          synq_parse_trigger_event(pCtx, evt, SYNTAQLITE_NULL_NODE);
    }
      yymsp[0].minor.yy213 = yylhsminor.yy213;
      break;
    case 305: /* trigger_event ::= UPDATE */
    {
      yymsp[0].minor.yy213 = synq_parse_trigger_event(
          pCtx, SYNTAQLITE_TRIGGER_EVENT_TYPE_UPDATE, SYNTAQLITE_NULL_NODE);
    } break;
    case 306: /* trigger_event ::= UPDATE OF idlist */
    {
      yymsp[-2].minor.yy213 = synq_parse_trigger_event(
          pCtx, SYNTAQLITE_TRIGGER_EVENT_TYPE_UPDATE, yymsp[0].minor.yy213);
    } break;
    case 308: /* foreach_clause ::= FOR EACH ROW */
    case 371: /* vtabarglist ::= vtabarg */
      yytestcase(yyruleno == 371);
    case 372: /* vtabarglist ::= vtabarglist COMMA vtabarg */
      yytestcase(yyruleno == 372);
    case 374: /* vtabarg ::= vtabarg vtabargtoken */
      yytestcase(yyruleno == 374);
    case 375: /* vtabargtoken ::= ANY */
      yytestcase(yyruleno == 375);
    case 376: /* vtabargtoken ::= lp anylist RP */
      yytestcase(yyruleno == 376);
    case 377: /* lp ::= LP */
      yytestcase(yyruleno == 377);
    case 379: /* anylist ::= anylist LP anylist RP */
      yytestcase(yyruleno == 379);
    case 380: /* anylist ::= anylist ANY */
      yytestcase(yyruleno == 380);
      {
        // consumed
      }
      break;
    case 311: /* trigger_cmd_list ::= trigger_cmd_list trigger_cmd SEMI */
    {
      yylhsminor.yy213 = synq_parse_trigger_cmd_list(
          pCtx, yymsp[-2].minor.yy213, yymsp[-1].minor.yy213);
    }
      yymsp[-2].minor.yy213 = yylhsminor.yy213;
      break;
    case 312: /* trigger_cmd_list ::= trigger_cmd SEMI */
    {
      yylhsminor.yy213 = synq_parse_trigger_cmd_list(pCtx, SYNTAQLITE_NULL_NODE,
                                                     yymsp[-1].minor.yy213);
    }
      yymsp[-1].minor.yy213 = yylhsminor.yy213;
      break;
    case 314: /* trnm ::= nm DOT nm */
    {
      yymsp[-2].minor.yy0 = yymsp[0].minor.yy0;
      // Qualified names not allowed in triggers, but grammar accepts them
    } break;
    case 316: /* tridxby ::= INDEXED BY nm */
    case 317: /* tridxby ::= NOT INDEXED */
      yytestcase(yyruleno == 317);
      {
        // Not allowed in triggers, but grammar accepts
      }
      break;
    case 318: /* trigger_cmd ::= UPDATE orconf trnm tridxby SET setlist from
                 where_opt scanpt */
    {
      uint32_t tbl =
          synq_parse_table_ref(pCtx, synq_span(pCtx, yymsp[-6].minor.yy0),
                               SYNQ_NO_SPAN, SYNQ_NO_SPAN);
      yymsp[-8].minor.yy213 = synq_parse_update_stmt(
          pCtx, (SyntaqliteConflictAction)yymsp[-7].minor.yy220, tbl,
          yymsp[-3].minor.yy213, yymsp[-2].minor.yy213, yymsp[-1].minor.yy213,
          SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE);
    } break;
    case 319: /* trigger_cmd ::= scanpt insert_cmd INTO trnm idlist_opt select
                 upsert scanpt */
    {
      uint32_t tbl =
          synq_parse_table_ref(pCtx, synq_span(pCtx, yymsp[-4].minor.yy0),
                               SYNQ_NO_SPAN, SYNQ_NO_SPAN);
      yymsp[-7].minor.yy213 = synq_parse_insert_stmt(
          pCtx, (SyntaqliteConflictAction)yymsp[-6].minor.yy220, tbl,
          yymsp[-3].minor.yy213, yymsp[-2].minor.yy213);
    } break;
    case 320: /* trigger_cmd ::= DELETE FROM trnm tridxby where_opt scanpt */
    {
      uint32_t tbl =
          synq_parse_table_ref(pCtx, synq_span(pCtx, yymsp[-3].minor.yy0),
                               SYNQ_NO_SPAN, SYNQ_NO_SPAN);
      yymsp[-5].minor.yy213 =
          synq_parse_delete_stmt(pCtx, tbl, yymsp[-1].minor.yy213,
                                 SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE);
    } break;
    case 322: /* cmd ::= PRAGMA nm dbnm */
    {
      SyntaqliteSourceSpan name_span =
          yymsp[0].minor.yy0.z ? synq_span(pCtx, yymsp[0].minor.yy0)
                               : synq_span(pCtx, yymsp[-1].minor.yy0);
      SyntaqliteSourceSpan schema_span =
          yymsp[0].minor.yy0.z ? synq_span(pCtx, yymsp[-1].minor.yy0)
                               : SYNQ_NO_SPAN;
      yymsp[-2].minor.yy213 =
          synq_parse_pragma_stmt(pCtx, name_span, schema_span, SYNQ_NO_SPAN,
                                 SYNTAQLITE_PRAGMA_FORM_BARE);
    } break;
    case 323: /* cmd ::= PRAGMA nm dbnm EQ nmnum */
    case 325: /* cmd ::= PRAGMA nm dbnm EQ minus_num */
      yytestcase(yyruleno == 325);
      {
        SyntaqliteSourceSpan name_span =
            yymsp[-2].minor.yy0.z ? synq_span(pCtx, yymsp[-2].minor.yy0)
                                  : synq_span(pCtx, yymsp[-3].minor.yy0);
        SyntaqliteSourceSpan schema_span =
            yymsp[-2].minor.yy0.z ? synq_span(pCtx, yymsp[-3].minor.yy0)
                                  : SYNQ_NO_SPAN;
        yymsp[-4].minor.yy213 = synq_parse_pragma_stmt(
            pCtx, name_span, schema_span, synq_span(pCtx, yymsp[0].minor.yy0),
            SYNTAQLITE_PRAGMA_FORM_EQ);
      }
      break;
    case 324: /* cmd ::= PRAGMA nm dbnm LP nmnum RP */
    case 326: /* cmd ::= PRAGMA nm dbnm LP minus_num RP */
      yytestcase(yyruleno == 326);
      {
        SyntaqliteSourceSpan name_span =
            yymsp[-3].minor.yy0.z ? synq_span(pCtx, yymsp[-3].minor.yy0)
                                  : synq_span(pCtx, yymsp[-4].minor.yy0);
        SyntaqliteSourceSpan schema_span =
            yymsp[-3].minor.yy0.z ? synq_span(pCtx, yymsp[-4].minor.yy0)
                                  : SYNQ_NO_SPAN;
        yymsp[-5].minor.yy213 = synq_parse_pragma_stmt(
            pCtx, name_span, schema_span, synq_span(pCtx, yymsp[-1].minor.yy0),
            SYNTAQLITE_PRAGMA_FORM_CALL);
      }
      break;
    case 334: /* minus_num ::= MINUS INTEGER|FLOAT */
    {
      // Build a token that spans from the MINUS sign through the number
      yylhsminor.yy0.z = yymsp[-1].minor.yy0.z;
      yylhsminor.yy0.n = (int)(yymsp[0].minor.yy0.z - yymsp[-1].minor.yy0.z) +
                         yymsp[0].minor.yy0.n;
    }
      yymsp[-1].minor.yy0 = yylhsminor.yy0;
      break;
    case 337: /* cmd ::= ANALYZE */
    {
      yymsp[0].minor.yy213 = synq_parse_analyze_stmt(
          pCtx, SYNQ_NO_SPAN, SYNQ_NO_SPAN, SYNTAQLITE_ANALYZE_KIND_ANALYZE);
    } break;
    case 338: /* cmd ::= ANALYZE nm dbnm */
    {
      SyntaqliteSourceSpan name_span =
          yymsp[0].minor.yy0.z ? synq_span(pCtx, yymsp[0].minor.yy0)
                               : synq_span(pCtx, yymsp[-1].minor.yy0);
      SyntaqliteSourceSpan schema_span =
          yymsp[0].minor.yy0.z ? synq_span(pCtx, yymsp[-1].minor.yy0)
                               : SYNQ_NO_SPAN;
      yymsp[-2].minor.yy213 = synq_parse_analyze_stmt(
          pCtx, name_span, schema_span, SYNTAQLITE_ANALYZE_KIND_ANALYZE);
    } break;
    case 339: /* cmd ::= REINDEX */
    {
      yymsp[0].minor.yy213 = synq_parse_analyze_stmt(
          pCtx, SYNQ_NO_SPAN, SYNQ_NO_SPAN, SYNTAQLITE_ANALYZE_KIND_REINDEX);
    } break;
    case 340: /* cmd ::= REINDEX nm dbnm */
    {
      SyntaqliteSourceSpan name_span =
          yymsp[0].minor.yy0.z ? synq_span(pCtx, yymsp[0].minor.yy0)
                               : synq_span(pCtx, yymsp[-1].minor.yy0);
      SyntaqliteSourceSpan schema_span =
          yymsp[0].minor.yy0.z ? synq_span(pCtx, yymsp[-1].minor.yy0)
                               : SYNQ_NO_SPAN;
      yymsp[-2].minor.yy213 =
          synq_parse_analyze_stmt(pCtx, name_span, schema_span, 1);
    } break;
    case 341: /* cmd ::= ATTACH database_kw_opt expr AS expr key_opt */
    {
      yymsp[-5].minor.yy213 =
          synq_parse_attach_stmt(pCtx, yymsp[-3].minor.yy213,
                                 yymsp[-1].minor.yy213, yymsp[0].minor.yy213);
    } break;
    case 342: /* cmd ::= DETACH database_kw_opt expr */
    {
      yymsp[-2].minor.yy213 =
          synq_parse_detach_stmt(pCtx, yymsp[0].minor.yy213);
    } break;
    case 343: /* database_kw_opt ::= DATABASE */
    {
      // Keyword consumed, no value needed
    } break;
    case 344: /* database_kw_opt ::= */
    {
      // Empty
    } break;
    case 347: /* cmd ::= VACUUM vinto */
    {
      yymsp[-1].minor.yy213 =
          synq_parse_vacuum_stmt(pCtx, SYNQ_NO_SPAN, yymsp[0].minor.yy213);
    } break;
    case 348: /* cmd ::= VACUUM nm vinto */
    {
      yymsp[-2].minor.yy213 = synq_parse_vacuum_stmt(
          pCtx, synq_span(pCtx, yymsp[-1].minor.yy0), yymsp[0].minor.yy213);
    } break;
    case 351: /* ecmd ::= explain cmdx SEMI */
    {
      yylhsminor.yy213 = synq_parse_explain_stmt(
          pCtx, (SyntaqliteExplainMode)(yymsp[-2].minor.yy220 - 1),
          yymsp[-1].minor.yy213);
      pCtx->root = yylhsminor.yy213;
      synq_parse_list_flush(pCtx);
      pCtx->stmt_completed = 1;
    }
      yymsp[-2].minor.yy213 = yylhsminor.yy213;
      break;
    case 353: /* explain ::= EXPLAIN QUERY PLAN */
    {
      yymsp[-2].minor.yy220 = 2;
    } break;
    case 354: /* cmd ::= createkw uniqueflag INDEX ifnotexists nm dbnm ON nm LP
                 sortlist RP where_opt */
    {
      SyntaqliteSourceSpan idx_name =
          yymsp[-6].minor.yy0.z ? synq_span(pCtx, yymsp[-6].minor.yy0)
                                : synq_span(pCtx, yymsp[-7].minor.yy0);
      SyntaqliteSourceSpan idx_schema =
          yymsp[-6].minor.yy0.z ? synq_span(pCtx, yymsp[-7].minor.yy0)
                                : SYNQ_NO_SPAN;
      yymsp[-11].minor.yy213 = synq_parse_create_index_stmt(
          pCtx, idx_name, idx_schema, synq_span(pCtx, yymsp[-4].minor.yy0),
          (SyntaqliteBool)yymsp[-10].minor.yy220,
          (SyntaqliteBool)yymsp[-8].minor.yy220, yymsp[-2].minor.yy213,
          yymsp[0].minor.yy213);
    } break;
    case 358: /* ifnotexists ::= IF NOT EXISTS */
    {
      yymsp[-2].minor.yy220 = 1;
    } break;
    case 359: /* cmd ::= createkw temp VIEW ifnotexists nm dbnm eidlist_opt AS
                 select */
    {
      SyntaqliteSourceSpan view_name =
          yymsp[-3].minor.yy0.z ? synq_span(pCtx, yymsp[-3].minor.yy0)
                                : synq_span(pCtx, yymsp[-4].minor.yy0);
      SyntaqliteSourceSpan view_schema =
          yymsp[-3].minor.yy0.z ? synq_span(pCtx, yymsp[-4].minor.yy0)
                                : SYNQ_NO_SPAN;
      yymsp[-8].minor.yy213 = synq_parse_create_view_stmt(
          pCtx, view_name, view_schema, (SyntaqliteBool)yymsp[-7].minor.yy220,
          (SyntaqliteBool)yymsp[-5].minor.yy220, yymsp[-2].minor.yy213,
          yymsp[0].minor.yy213);
    } break;
    case 363: /* values ::= VALUES LP nexprlist RP */
    {
      yymsp[-3].minor.yy213 = synq_parse_values_row_list(
          pCtx, SYNTAQLITE_NULL_NODE, yymsp[-1].minor.yy213);
    } break;
    case 364: /* mvalues ::= values COMMA LP nexprlist RP */
    case 365: /* mvalues ::= mvalues COMMA LP nexprlist RP */
      yytestcase(yyruleno == 365);
      {
        yymsp[-4].minor.yy213 = synq_parse_values_row_list(
            pCtx, yymsp[-4].minor.yy213, yymsp[-1].minor.yy213);
      }
      break;
    case 366: /* oneselect ::= values */
    case 367: /* oneselect ::= mvalues */
      yytestcase(yyruleno == 367);
      {
        yylhsminor.yy213 = synq_parse_values_clause(pCtx, yymsp[0].minor.yy213);
      }
      yymsp[0].minor.yy213 = yylhsminor.yy213;
      break;
    case 369: /* cmd ::= create_vtab LP vtabarglist RP */
    {
      // Capture module arguments span (content between parens)
      SyntaqliteNode* vtab = AST_NODE(&pCtx->ast, yymsp[-3].minor.yy213);
      const char* args_start = yymsp[-2].minor.yy0.z + yymsp[-2].minor.yy0.n;
      const char* args_end = yymsp[0].minor.yy0.z;
      vtab->create_virtual_table_stmt.module_args =
          (SyntaqliteSourceSpan){(uint32_t)(args_start - pCtx->source),
                                 (uint16_t)(args_end - args_start)};
      yylhsminor.yy213 = yymsp[-3].minor.yy213;
    }
      yymsp[-3].minor.yy213 = yylhsminor.yy213;
      break;
    case 370: /* create_vtab ::= createkw VIRTUAL TABLE ifnotexists nm dbnm
                 USING nm */
    {
      SyntaqliteSourceSpan tbl_name =
          yymsp[-2].minor.yy0.z ? synq_span(pCtx, yymsp[-2].minor.yy0)
                                : synq_span(pCtx, yymsp[-3].minor.yy0);
      SyntaqliteSourceSpan tbl_schema =
          yymsp[-2].minor.yy0.z ? synq_span(pCtx, yymsp[-3].minor.yy0)
                                : SYNQ_NO_SPAN;
      yymsp[-7].minor.yy213 = synq_parse_create_virtual_table_stmt(
          pCtx, tbl_name, tbl_schema, synq_span(pCtx, yymsp[0].minor.yy0),
          (SyntaqliteBool)yymsp[-4].minor.yy220,
          SYNQ_NO_SPAN);  // module_args = none by default
    } break;
    case 381: /* windowdefn_list ::= windowdefn */
    {
      yylhsminor.yy213 = synq_parse_named_window_def_list(
          pCtx, SYNTAQLITE_NULL_NODE, yymsp[0].minor.yy213);
    }
      yymsp[0].minor.yy213 = yylhsminor.yy213;
      break;
    case 382: /* windowdefn_list ::= windowdefn_list COMMA windowdefn */
    {
      yylhsminor.yy213 = synq_parse_named_window_def_list(
          pCtx, yymsp[-2].minor.yy213, yymsp[0].minor.yy213);
    }
      yymsp[-2].minor.yy213 = yylhsminor.yy213;
      break;
    case 383: /* windowdefn ::= nm AS LP window RP */
    {
      yylhsminor.yy213 = synq_parse_named_window_def(
          pCtx, synq_span(pCtx, yymsp[-4].minor.yy0), yymsp[-1].minor.yy213);
    }
      yymsp[-4].minor.yy213 = yylhsminor.yy213;
      break;
    case 384: /* window ::= PARTITION BY nexprlist orderby_opt frame_opt */
    {
      yymsp[-4].minor.yy213 =
          synq_parse_window_def(pCtx, SYNQ_NO_SPAN, yymsp[-2].minor.yy213,
                                yymsp[-1].minor.yy213, yymsp[0].minor.yy213);
    } break;
    case 385: /* window ::= nm PARTITION BY nexprlist orderby_opt frame_opt */
    {
      yylhsminor.yy213 = synq_parse_window_def(
          pCtx, synq_span(pCtx, yymsp[-5].minor.yy0), yymsp[-2].minor.yy213,
          yymsp[-1].minor.yy213, yymsp[0].minor.yy213);
    }
      yymsp[-5].minor.yy213 = yylhsminor.yy213;
      break;
    case 386: /* window ::= ORDER BY sortlist frame_opt */
    {
      yymsp[-3].minor.yy213 =
          synq_parse_window_def(pCtx, SYNQ_NO_SPAN, SYNTAQLITE_NULL_NODE,
                                yymsp[-1].minor.yy213, yymsp[0].minor.yy213);
    } break;
    case 387: /* window ::= nm ORDER BY sortlist frame_opt */
    {
      yylhsminor.yy213 = synq_parse_window_def(
          pCtx, synq_span(pCtx, yymsp[-4].minor.yy0), SYNTAQLITE_NULL_NODE,
          yymsp[-1].minor.yy213, yymsp[0].minor.yy213);
    }
      yymsp[-4].minor.yy213 = yylhsminor.yy213;
      break;
    case 388: /* window ::= frame_opt */
    {
      yylhsminor.yy213 =
          synq_parse_window_def(pCtx, SYNQ_NO_SPAN, SYNTAQLITE_NULL_NODE,
                                SYNTAQLITE_NULL_NODE, yymsp[0].minor.yy213);
    }
      yymsp[0].minor.yy213 = yylhsminor.yy213;
      break;
    case 389: /* window ::= nm frame_opt */
    {
      yylhsminor.yy213 = synq_parse_window_def(
          pCtx, synq_span(pCtx, yymsp[-1].minor.yy0), SYNTAQLITE_NULL_NODE,
          SYNTAQLITE_NULL_NODE, yymsp[0].minor.yy213);
    }
      yymsp[-1].minor.yy213 = yylhsminor.yy213;
      break;
    case 391: /* frame_opt ::= range_or_rows frame_bound_s frame_exclude_opt */
    {
      // Single bound: start=yymsp[-1].minor.yy213, end=CURRENT ROW (implicit)
      uint32_t end_bound = synq_parse_frame_bound(
          pCtx, SYNTAQLITE_FRAME_BOUND_TYPE_CURRENT_ROW, SYNTAQLITE_NULL_NODE);
      yylhsminor.yy213 = synq_parse_frame_spec(
          pCtx, (SyntaqliteFrameType)yymsp[-2].minor.yy220,
          (SyntaqliteFrameExclude)yymsp[0].minor.yy220, yymsp[-1].minor.yy213,
          end_bound);
    }
      yymsp[-2].minor.yy213 = yylhsminor.yy213;
      break;
    case 392: /* frame_opt ::= range_or_rows BETWEEN frame_bound_s AND
                 frame_bound_e frame_exclude_opt */
    {
      yylhsminor.yy213 = synq_parse_frame_spec(
          pCtx, (SyntaqliteFrameType)yymsp[-5].minor.yy220,
          (SyntaqliteFrameExclude)yymsp[0].minor.yy220, yymsp[-3].minor.yy213,
          yymsp[-1].minor.yy213);
    }
      yymsp[-5].minor.yy213 = yylhsminor.yy213;
      break;
    case 393: /* range_or_rows ::= RANGE|ROWS|GROUPS */
    {
      switch (yymsp[0].minor.yy0.type) {
        case SYNTAQLITE_TK_RANGE:
          yylhsminor.yy220 = SYNTAQLITE_FRAME_TYPE_RANGE;
          break;
        case SYNTAQLITE_TK_ROWS:
          yylhsminor.yy220 = SYNTAQLITE_FRAME_TYPE_ROWS;
          break;
        default:
          yylhsminor.yy220 = SYNTAQLITE_FRAME_TYPE_GROUPS;
          break;
      }
    }
      yymsp[0].minor.yy220 = yylhsminor.yy220;
      break;
    case 395: /* frame_bound_s ::= UNBOUNDED PRECEDING */
    {
      yymsp[-1].minor.yy213 = synq_parse_frame_bound(
          pCtx, SYNTAQLITE_FRAME_BOUND_TYPE_UNBOUNDED_PRECEDING,
          SYNTAQLITE_NULL_NODE);
    } break;
    case 397: /* frame_bound_e ::= UNBOUNDED FOLLOWING */
    {
      yymsp[-1].minor.yy213 = synq_parse_frame_bound(
          pCtx, SYNTAQLITE_FRAME_BOUND_TYPE_UNBOUNDED_FOLLOWING,
          SYNTAQLITE_NULL_NODE);
    } break;
    case 398: /* frame_bound ::= expr PRECEDING|FOLLOWING */
    {
      SyntaqliteFrameBoundType bt =
          (yymsp[0].minor.yy0.type == SYNTAQLITE_TK_PRECEDING)
              ? SYNTAQLITE_FRAME_BOUND_TYPE_EXPR_PRECEDING
              : SYNTAQLITE_FRAME_BOUND_TYPE_EXPR_FOLLOWING;
      yylhsminor.yy213 =
          synq_parse_frame_bound(pCtx, bt, yymsp[-1].minor.yy213);
    }
      yymsp[-1].minor.yy213 = yylhsminor.yy213;
      break;
    case 399: /* frame_bound ::= CURRENT ROW */
    {
      yymsp[-1].minor.yy213 = synq_parse_frame_bound(
          pCtx, SYNTAQLITE_FRAME_BOUND_TYPE_CURRENT_ROW, SYNTAQLITE_NULL_NODE);
    } break;
    case 400: /* frame_exclude_opt ::= */
    {
      yymsp[1].minor.yy220 = SYNTAQLITE_FRAME_EXCLUDE_NONE;
    } break;
    case 402: /* frame_exclude ::= NO OTHERS */
    {
      yymsp[-1].minor.yy220 = SYNTAQLITE_FRAME_EXCLUDE_NO_OTHERS;
    } break;
    case 403: /* frame_exclude ::= CURRENT ROW */
    {
      yymsp[-1].minor.yy220 = SYNTAQLITE_FRAME_EXCLUDE_CURRENT_ROW;
    } break;
    case 404: /* frame_exclude ::= GROUP|TIES */
    {
      yylhsminor.yy220 = (yymsp[0].minor.yy0.type == SYNTAQLITE_TK_GROUP)
                             ? SYNTAQLITE_FRAME_EXCLUDE_GROUP
                             : SYNTAQLITE_FRAME_EXCLUDE_TIES;
    }
      yymsp[0].minor.yy220 = yylhsminor.yy220;
      break;
    case 406: /* filter_over ::= filter_clause over_clause */
    {
      // Unpack the over_clause FilterOver to combine with filter expr
      SyntaqliteFilterOver* fo_over = (SyntaqliteFilterOver*)synq_arena_ptr(
          &pCtx->ast, yymsp[0].minor.yy213);
      yylhsminor.yy213 = synq_parse_filter_over(
          pCtx, yymsp[-1].minor.yy213, fo_over->over_def, SYNQ_NO_SPAN);
    }
      yymsp[-1].minor.yy213 = yylhsminor.yy213;
      break;
    case 408: /* filter_over ::= filter_clause */
    {
      yylhsminor.yy213 = synq_parse_filter_over(
          pCtx, yymsp[0].minor.yy213, SYNTAQLITE_NULL_NODE, SYNQ_NO_SPAN);
    }
      yymsp[0].minor.yy213 = yylhsminor.yy213;
      break;
    case 409: /* over_clause ::= OVER LP window RP */
    {
      yymsp[-3].minor.yy213 = synq_parse_filter_over(
          pCtx, SYNTAQLITE_NULL_NODE, yymsp[-1].minor.yy213, SYNQ_NO_SPAN);
    } break;
    case 410: /* over_clause ::= OVER nm */
    {
      // Create a WindowDef with just base_window_name to represent a named
      // window ref
      uint32_t wdef = synq_parse_window_def(
          pCtx, synq_span(pCtx, yymsp[0].minor.yy0), SYNTAQLITE_NULL_NODE,
          SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE);
      yymsp[-1].minor.yy213 = synq_parse_filter_over(pCtx, SYNTAQLITE_NULL_NODE,
                                                     wdef, SYNQ_NO_SPAN);
    } break;
    case 411: /* filter_clause ::= FILTER LP WHERE expr RP */
    {
      yymsp[-4].minor.yy213 = yymsp[-1].minor.yy213;
    } break;
    default:
      break;
      /********** End reduce actions
       * ************************************************/
  };
  assert(yyruleno < sizeof(yyRuleInfoLhs) / sizeof(yyRuleInfoLhs[0]));
  yygoto = yyRuleInfoLhs[yyruleno];
  yysize = yyRuleInfoNRhs[yyruleno];
  yyact = yy_find_reduce_action(yymsp[yysize].stateno, (YYCODETYPE)yygoto);

  /* There are no SHIFTREDUCE actions on nonterminals because the table
  ** generator has simplified them to pure REDUCE actions. */
  assert(!(yyact > YY_MAX_SHIFT && yyact <= YY_MAX_SHIFTREDUCE));

  /* It is not possible for a REDUCE to be followed by an error */
  assert(yyact != YY_ERROR_ACTION);

  yymsp += yysize + 1;
  yypParser->yytos = yymsp;
  yymsp->stateno = (YYACTIONTYPE)yyact;
  yymsp->major = (YYCODETYPE)yygoto;
  yyTraceShift(yypParser, yyact, "... then shift");
  return yyact;
}

/*
** The following code executes when the parse fails
*/
#ifndef YYNOERRORRECOVERY
static void yy_parse_failed(yyParser* yypParser /* The parser */
) {
  SynqSqliteParseARG_FETCH SynqSqliteParseCTX_FETCH
#ifndef NDEBUG
      if (yyTraceFILE) {
    fprintf(yyTraceFILE, "%sFail!\n", yyTracePrompt);
  }
#endif
  while (yypParser->yytos > yypParser->yystack)
    yy_pop_parser_stack(yypParser);
  /* Here code is inserted which will be executed whenever the
  ** parser fails */
  /************ Begin %parse_failure code
   * ***************************************/

  if (pCtx) {
    pCtx->error = 1;
  }
  /************ End %parse_failure code
   * *****************************************/
  SynqSqliteParseARG_STORE /* Suppress warning about unused %extra_argument
                              variable */
      SynqSqliteParseCTX_STORE
}
#endif /* YYNOERRORRECOVERY */

/*
** The following code executes when a syntax error first occurs.
*/
static void yy_syntax_error(
    yyParser* yypParser,             /* The parser */
    int yymajor,                     /* The major type of the error token */
    SynqSqliteParseTOKENTYPE yyminor /* The minor type of the error token */
) {
  SynqSqliteParseARG_FETCH SynqSqliteParseCTX_FETCH
#define TOKEN yyminor
      /************ Begin %syntax_error code
         ****************************************/

      (void) yymajor;
  (void)TOKEN;
  if (pCtx) {
    pCtx->error = 1;
  }
  /************ End %syntax_error code
   * ******************************************/
  SynqSqliteParseARG_STORE /* Suppress warning about unused %extra_argument
                              variable */
      SynqSqliteParseCTX_STORE
}

/*
** The following is executed when the parser accepts
*/
static void yy_accept(yyParser* yypParser /* The parser */
) {
  SynqSqliteParseARG_FETCH SynqSqliteParseCTX_FETCH
#ifndef NDEBUG
      if (yyTraceFILE) {
    fprintf(yyTraceFILE, "%sAccept!\n", yyTracePrompt);
  }
#endif
#ifndef YYNOERRORRECOVERY
  yypParser->yyerrcnt = -1;
#endif
  assert(yypParser->yytos == yypParser->yystack);
  /* Here code is inserted which will be executed whenever the
  ** parser accepts */
  /*********** Begin %parse_accept code
   * *****************************************/
  /*********** End %parse_accept code
   * *******************************************/
  SynqSqliteParseARG_STORE /* Suppress warning about unused %extra_argument
                              variable */
      SynqSqliteParseCTX_STORE
}

/* The main parser program.
** The first argument is a pointer to a structure obtained from
** "SynqSqliteParseAlloc" which describes the current state of the parser.
** The second argument is the major token number.  The third is
** the minor token.  The fourth optional argument is whatever the
** user wants (and specified in the grammar) and is available for
** use by the action routines.
**
** Inputs:
** <ul>
** <li> A pointer to the parser (an opaque structure.)
** <li> The major token number.
** <li> The minor token number.
** <li> An option argument of a grammar-specified type.
** </ul>
**
** Outputs:
** None.
*/
void SynqSqliteParse(
    void* yyp,                       /* The parser */
    int yymajor,                     /* The major token code number */
    SynqSqliteParseTOKENTYPE yyminor /* The value for the token */
        SynqSqliteParseARG_PDECL     /* Optional %extra_argument parameter */
) {
  YYMINORTYPE yyminorunion;
  YYACTIONTYPE yyact; /* The parser action. */
#if !defined(YYERRORSYMBOL) && !defined(YYNOERRORRECOVERY)
  int yyendofinput; /* True if we are at the end of input */
#endif
#ifdef YYERRORSYMBOL
  int yyerrorhit = 0; /* True if yymajor has invoked an error */
#endif
  yyParser* yypParser = (yyParser*)yyp; /* The parser */
  SynqSqliteParseCTX_FETCH SynqSqliteParseARG_STORE

      assert(yypParser->yytos != 0);
#if !defined(YYERRORSYMBOL) && !defined(YYNOERRORRECOVERY)
  yyendofinput = (yymajor == 0);
#endif

  yyact = yypParser->yytos->stateno;
#ifndef NDEBUG
  if (yyTraceFILE) {
    if (yyact < YY_MIN_REDUCE) {
      fprintf(yyTraceFILE, "%sInput '%s' in state %d\n", yyTracePrompt,
              yyTokenName[yymajor], yyact);
    } else {
      fprintf(yyTraceFILE, "%sInput '%s' with pending reduce %d\n",
              yyTracePrompt, yyTokenName[yymajor], yyact - YY_MIN_REDUCE);
    }
  }
#endif

  while (1) { /* Exit by "break" */
    assert(yypParser->yytos >= yypParser->yystack);
    assert(yyact == yypParser->yytos->stateno);
    yyact = yy_find_shift_action((YYCODETYPE)yymajor, yyact);
    if (yyact >= YY_MIN_REDUCE) {
      unsigned int yyruleno = yyact - YY_MIN_REDUCE; /* Reduce by this rule */
#ifndef NDEBUG
      assert(yyruleno < (int)(sizeof(yyRuleName) / sizeof(yyRuleName[0])));
      if (yyTraceFILE) {
        int yysize = yyRuleInfoNRhs[yyruleno];
        if (yysize) {
          fprintf(
              yyTraceFILE, "%sReduce %d [%s]%s, pop back to state %d.\n",
              yyTracePrompt, yyruleno, yyRuleName[yyruleno],
              yyruleno < YYNRULE_WITH_ACTION ? "" : " without external action",
              yypParser->yytos[yysize].stateno);
        } else {
          fprintf(
              yyTraceFILE, "%sReduce %d [%s]%s.\n", yyTracePrompt, yyruleno,
              yyRuleName[yyruleno],
              yyruleno < YYNRULE_WITH_ACTION ? "" : " without external action");
        }
      }
#endif /* NDEBUG */

      /* Check that the stack is large enough to grow by a single entry
      ** if the RHS of the rule is empty.  This ensures that there is room
      ** enough on the stack to push the LHS value */
      if (yyRuleInfoNRhs[yyruleno] == 0) {
#ifdef YYTRACKMAXSTACKDEPTH
        if ((int)(yypParser->yytos - yypParser->yystack) > yypParser->yyhwm) {
          yypParser->yyhwm++;
          assert(yypParser->yyhwm ==
                 (int)(yypParser->yytos - yypParser->yystack));
        }
#endif
        if (yypParser->yytos >= yypParser->yystackEnd) {
          if (yyGrowStack(yypParser)) {
            yyStackOverflow(yypParser);
            break;
          }
        }
      }
      yyact = yy_reduce(yypParser, yyruleno, yymajor,
                        yyminor SynqSqliteParseCTX_PARAM);
    } else if (yyact <= YY_MAX_SHIFTREDUCE) {
      yy_shift(yypParser, yyact, (YYCODETYPE)yymajor, yyminor);
#ifndef YYNOERRORRECOVERY
      yypParser->yyerrcnt--;
#endif
      break;
    } else if (yyact == YY_ACCEPT_ACTION) {
      yypParser->yytos--;
      yy_accept(yypParser);
      return;
    } else {
      assert(yyact == YY_ERROR_ACTION);
      yyminorunion.yy0 = yyminor;
#ifdef YYERRORSYMBOL
      int yymx;
#endif
#ifndef NDEBUG
      if (yyTraceFILE) {
        fprintf(yyTraceFILE, "%sSyntax Error!\n", yyTracePrompt);
      }
#endif
#ifdef YYERRORSYMBOL
      /* A syntax error has occurred.
      ** The response to an error depends upon whether or not the
      ** grammar defines an error token "ERROR".
      **
      ** This is what we do if the grammar does define ERROR:
      **
      **  * Call the %syntax_error function.
      **
      **  * Begin popping the stack until we enter a state where
      **    it is legal to shift the error symbol, then shift
      **    the error symbol.
      **
      **  * Set the error count to three.
      **
      **  * Begin accepting and shifting new tokens.  No new error
      **    processing will occur until three tokens have been
      **    shifted successfully.
      **
      */
      if (yypParser->yyerrcnt < 0) {
        yy_syntax_error(yypParser, yymajor, yyminor);
      }
      yymx = yypParser->yytos->major;
      if (yymx == YYERRORSYMBOL || yyerrorhit) {
#ifndef NDEBUG
        if (yyTraceFILE) {
          fprintf(yyTraceFILE, "%sDiscard input token %s\n", yyTracePrompt,
                  yyTokenName[yymajor]);
        }
#endif
        yy_destructor(yypParser, (YYCODETYPE)yymajor, &yyminorunion);
        yymajor = YYNOCODE;
      } else {
        while (yypParser->yytos > yypParser->yystack) {
          yyact =
              yy_find_reduce_action(yypParser->yytos->stateno, YYERRORSYMBOL);
          if (yyact <= YY_MAX_SHIFTREDUCE)
            break;
          yy_pop_parser_stack(yypParser);
        }
        if (yypParser->yytos <= yypParser->yystack || yymajor == 0) {
          yy_destructor(yypParser, (YYCODETYPE)yymajor, &yyminorunion);
          yy_parse_failed(yypParser);
#ifndef YYNOERRORRECOVERY
          yypParser->yyerrcnt = -1;
#endif
          yymajor = YYNOCODE;
        } else if (yymx != YYERRORSYMBOL) {
          yy_shift(yypParser, yyact, YYERRORSYMBOL, yyminor);
        }
      }
      yypParser->yyerrcnt = 3;
      yyerrorhit = 1;
      if (yymajor == YYNOCODE)
        break;
      yyact = yypParser->yytos->stateno;
#elif defined(YYNOERRORRECOVERY)
      /* If the YYNOERRORRECOVERY macro is defined, then do not attempt to
      ** do any kind of error recovery.  Instead, simply invoke the syntax
      ** error routine and continue going as if nothing had happened.
      **
      ** Applications can set this macro (for example inside %include) if
      ** they intend to abandon the parse upon the first syntax error seen.
      */
      yy_syntax_error(yypParser, yymajor, yyminor);
      yy_destructor(yypParser, (YYCODETYPE)yymajor, &yyminorunion);
      break;
#else /* YYERRORSYMBOL is not defined */
      /* This is what we do if the grammar does not define ERROR:
      **
      **  * Report an error message, and throw away the input token.
      **
      **  * If the input token is $, then fail the parse.
      **
      ** As before, subsequent error messages are suppressed until
      ** three input tokens have been successfully shifted.
      */
      if (yypParser->yyerrcnt <= 0) {
        yy_syntax_error(yypParser, yymajor, yyminor);
      }
      yypParser->yyerrcnt = 3;
      yy_destructor(yypParser, (YYCODETYPE)yymajor, &yyminorunion);
      if (yyendofinput) {
        yy_parse_failed(yypParser);
#ifndef YYNOERRORRECOVERY
        yypParser->yyerrcnt = -1;
#endif
      }
      break;
#endif
    }
  }
#ifndef NDEBUG
  if (yyTraceFILE) {
    yyStackEntry* i;
    char cDiv = '[';
    fprintf(yyTraceFILE, "%sReturn. Stack=", yyTracePrompt);
    for (i = &yypParser->yystack[1]; i <= yypParser->yytos; i++) {
      fprintf(yyTraceFILE, "%c%s", cDiv, yyTokenName[i->major]);
      cDiv = ' ';
    }
    fprintf(yyTraceFILE, "]\n");
  }
#endif
  return;
}

/*
** Return the fallback token corresponding to canonical token iToken, or
** 0 if iToken has no fallback.
*/
int SynqSqliteParseFallback(int iToken) {
#ifdef YYFALLBACK
  assert(iToken < (int)(sizeof(yyFallback) / sizeof(yyFallback[0])));
  return yyFallback[iToken];
#else
  (void)iToken;
  return 0;
#endif
}

/* syntaqlite extension: enumerate terminals that can be shifted/reduced from
** the parser's current state. Returns the total number of expected tokens,
** even when out_tokens/out_cap only request a prefix. */
static YYACTIONTYPE synq_find_reduce_action_safe(YYACTIONTYPE stateno,
                                                 YYCODETYPE iLookAhead) {
  int i;
  if (stateno > YY_REDUCE_COUNT)
    return yy_default[stateno];
  i = yy_reduce_ofst[stateno] + iLookAhead;
  if (i < 0 || i >= YY_ACTTAB_COUNT || yy_lookahead[i] != iLookAhead) {
    return yy_default[stateno];
  }
  return yy_action[i];
}

/* Like yy_find_shift_action but skips YYWILDCARD and YYFALLBACK paths.
** Wildcard matches are for error recovery (ANY token) and fallback matches
** accept keywords as identifiers — neither should appear as keyword
** autocompletion suggestions. */
static YYACTIONTYPE synq_find_shift_action_strict(YYCODETYPE iLookAhead,
                                                  YYACTIONTYPE stateno) {
  int i;
  if (stateno > YY_MAX_SHIFT)
    return stateno;
  i = yy_shift_ofst[stateno];
  assert(i >= 0);
  assert(i + YYNTOKEN <= (int)YY_NLOOKAHEAD);
  i += iLookAhead;
  if (yy_lookahead[i] != iLookAhead) {
    /* No specific entry — skip fallback and wildcard, use default. */
    return yy_default[stateno];
  }
  return yy_action[i];
}

static int synq_can_lookahead(yyParser* p, int token) {
  YYACTIONTYPE stack_states[YYSTACKDEPTH + 1];
  int top = 0;
  int i = 0;
  int steps = 0;

  if (p == 0 || p->yytos == 0)
    return 0;

  top = (int)(p->yytos - p->yystack);
  if (top < 0 || top > YYSTACKDEPTH)
    return 0;
  for (i = 0; i <= top; i++) {
    stack_states[i] = p->yystack[i].stateno;
  }

  while (steps++ < 10000) {
    YYACTIONTYPE action =
        synq_find_shift_action_strict((YYCODETYPE)token, stack_states[top]);

    if (action == YY_ERROR_ACTION || action == YY_NO_ACTION)
      return 0;
    if (action == YY_ACCEPT_ACTION)
      return token == 0;
    if (action <= YY_MAX_SHIFT)
      return 1;

    /* Shift-reduce: the token is consumed (shifted) then a reduce follows.
    ** This means the token IS accepted, same as a pure shift. */
    if (action >= YY_MIN_SHIFTREDUCE && action <= YY_MAX_SHIFTREDUCE)
      return 1;

    if (action >= YY_MIN_REDUCE && action <= YY_MAX_REDUCE) {
      int rule = (int)(action - YY_MIN_REDUCE);
      int yysize = yyRuleInfoNRhs[rule];
      YYACTIONTYPE goto_state;

      top += yysize; /* yyRuleInfoNRhs is negative rhs-size */
      if (top < 0)
        return 0;

      goto_state =
          synq_find_reduce_action_safe(stack_states[top], yyRuleInfoLhs[rule]);
      if (goto_state == YY_ERROR_ACTION || goto_state == YY_NO_ACTION)
        return 0;

      if (top >= YYSTACKDEPTH)
        return 0;
      top++;
      stack_states[top] = goto_state;
      continue;
    }

    return 0;
  }

  return 0;
}

int SynqSqliteParseExpectedTokens(void* parser, int* out_tokens, int out_cap) {
  int n = 0;
  int token = 0;
  yyParser* p = (yyParser*)parser;

  if (p == 0 || p->yytos == 0)
    return 0;

  for (token = 1; token < YYNTOKEN; token++) {
    if (!synq_can_lookahead(p, token))
      continue;
    if (out_tokens && n < out_cap)
      out_tokens[n] = token;
    n++;
  }

  return n;
}

/* syntaqlite extension: non-terminal IDs for completion context. */
#define SYNQ_NT_INPUT 188
#define SYNQ_NT_CMDLIST 189
#define SYNQ_NT_ECMD 190
#define SYNQ_NT_CMDX 191
#define SYNQ_NT_ERROR 192
#define SYNQ_NT_CMD 193
#define SYNQ_NT_EXPR 194
#define SYNQ_NT_DISTINCT 195
#define SYNQ_NT_EXPRLIST 196
#define SYNQ_NT_SORTLIST 197
#define SYNQ_NT_FILTER_OVER 198
#define SYNQ_NT_TYPETOKEN 199
#define SYNQ_NT_TYPENAME 200
#define SYNQ_NT_SIGNED 201
#define SYNQ_NT_SELCOLLIST 202
#define SYNQ_NT_SCLP 203
#define SYNQ_NT_SCANPT 204
#define SYNQ_NT_NM 205
#define SYNQ_NT_MULTISELECT_OP 206
#define SYNQ_NT_IN_OP 207
#define SYNQ_NT_DBNM 208
#define SYNQ_NT_SELECTNOWITH 209
#define SYNQ_NT_ONESELECT 210
#define SYNQ_NT_SELECT 211
#define SYNQ_NT_PAREN_EXPRLIST 212
#define SYNQ_NT_LIKEOP 213
#define SYNQ_NT_BETWEEN_OP 214
#define SYNQ_NT_CASE_OPERAND 215
#define SYNQ_NT_CASE_EXPRLIST 216
#define SYNQ_NT_CASE_ELSE 217
#define SYNQ_NT_SCANTOK 218
#define SYNQ_NT_AUTOINC 219
#define SYNQ_NT_REFARGS 220
#define SYNQ_NT_REFARG 221
#define SYNQ_NT_REFACT 222
#define SYNQ_NT_DEFER_SUBCLAUSE 223
#define SYNQ_NT_INIT_DEFERRED_PRED_OPT 224
#define SYNQ_NT_DEFER_SUBCLAUSE_OPT 225
#define SYNQ_NT_TABLE_OPTION_SET 226
#define SYNQ_NT_TABLE_OPTION 227
#define SYNQ_NT_TCONSCOMMA 228
#define SYNQ_NT_ONCONF 229
#define SYNQ_NT_CCONS 230
#define SYNQ_NT_CARGLIST 231
#define SYNQ_NT_TCONS 232
#define SYNQ_NT_CONSLIST 233
#define SYNQ_NT_GENERATED 234
#define SYNQ_NT_CREATE_TABLE 235
#define SYNQ_NT_CREATE_TABLE_ARGS 236
#define SYNQ_NT_CREATEKW 237
#define SYNQ_NT_TEMP 238
#define SYNQ_NT_IFNOTEXISTS 239
#define SYNQ_NT_COLUMNLIST 240
#define SYNQ_NT_CONSLIST_OPT 241
#define SYNQ_NT_COLUMNNAME 242
#define SYNQ_NT_TERM 243
#define SYNQ_NT_SORTORDER 244
#define SYNQ_NT_EIDLIST_OPT 245
#define SYNQ_NT_EIDLIST 246
#define SYNQ_NT_RESOLVETYPE 247
#define SYNQ_NT_WITHNM 248
#define SYNQ_NT_WQAS 249
#define SYNQ_NT_COLLATE 250
#define SYNQ_NT_WQLIST 251
#define SYNQ_NT_WQITEM 252
#define SYNQ_NT_WITH 253
#define SYNQ_NT_INSERT_CMD 254
#define SYNQ_NT_ORCONF 255
#define SYNQ_NT_INDEXED_OPT 256
#define SYNQ_NT_XFULLNAME 257
#define SYNQ_NT_WHERE_OPT_RET 258
#define SYNQ_NT_ORDERBY_OPT 259
#define SYNQ_NT_LIMIT_OPT 260
#define SYNQ_NT_SETLIST 261
#define SYNQ_NT_FROM 262
#define SYNQ_NT_IDLIST_OPT 263
#define SYNQ_NT_UPSERT 264
#define SYNQ_NT_RETURNING 265
#define SYNQ_NT_RAISETYPE 266
#define SYNQ_NT_INDEXED_BY 267
#define SYNQ_NT_IDLIST 268
#define SYNQ_NT_WHERE_OPT 269
#define SYNQ_NT_NEXPRLIST 270
#define SYNQ_NT_NULLS 271
#define SYNQ_NT_IFEXISTS 272
#define SYNQ_NT_TRANSTYPE 273
#define SYNQ_NT_TRANS_OPT 274
#define SYNQ_NT_SAVEPOINT_OPT 275
#define SYNQ_NT_KWCOLUMN_OPT 276
#define SYNQ_NT_FULLNAME 277
#define SYNQ_NT_ADD_COLUMN_FULLNAME 278
#define SYNQ_NT_AS 279
#define SYNQ_NT_GROUPBY_OPT 280
#define SYNQ_NT_HAVING_OPT 281
#define SYNQ_NT_WINDOW_CLAUSE 282
#define SYNQ_NT_SELTABLIST 283
#define SYNQ_NT_ON_USING 284
#define SYNQ_NT_JOINOP 285
#define SYNQ_NT_STL_PREFIX 286
#define SYNQ_NT_TRIGGER_TIME 287
#define SYNQ_NT_TRNM 288
#define SYNQ_NT_TRIGGER_DECL 289
#define SYNQ_NT_TRIGGER_CMD_LIST 290
#define SYNQ_NT_TRIGGER_EVENT 291
#define SYNQ_NT_FOREACH_CLAUSE 292
#define SYNQ_NT_WHEN_CLAUSE 293
#define SYNQ_NT_TRIGGER_CMD 294
#define SYNQ_NT_TRIDXBY 295
#define SYNQ_NT_PLUS_NUM 296
#define SYNQ_NT_MINUS_NUM 297
#define SYNQ_NT_NMNUM 298
#define SYNQ_NT_UNIQUEFLAG 299
#define SYNQ_NT_EXPLAIN 300
#define SYNQ_NT_DATABASE_KW_OPT 301
#define SYNQ_NT_KEY_OPT 302
#define SYNQ_NT_VINTO 303
#define SYNQ_NT_VALUES 304
#define SYNQ_NT_MVALUES 305
#define SYNQ_NT_CREATE_VTAB 306
#define SYNQ_NT_VTABARGLIST 307
#define SYNQ_NT_VTABARG 308
#define SYNQ_NT_VTABARGTOKEN 309
#define SYNQ_NT_LP 310
#define SYNQ_NT_ANYLIST 311
#define SYNQ_NT_RANGE_OR_ROWS 312
#define SYNQ_NT_FRAME_EXCLUDE_OPT 313
#define SYNQ_NT_FRAME_EXCLUDE 314
#define SYNQ_NT_WINDOWDEFN_LIST 315
#define SYNQ_NT_WINDOWDEFN 316
#define SYNQ_NT_WINDOW 317
#define SYNQ_NT_FRAME_OPT 318
#define SYNQ_NT_FRAME_BOUND_S 319
#define SYNQ_NT_FRAME_BOUND_E 320
#define SYNQ_NT_FRAME_BOUND 321
#define SYNQ_NT_FILTER_CLAUSE 322
#define SYNQ_NT_OVER_CLAUSE 323

/* syntaqlite extension: probe the goto table to check if a state has
** an explicit goto entry for non-terminal `nt`. */
static int synq_has_goto(YYACTIONTYPE state, YYCODETYPE nt) {
  int i;
  if (state > YY_REDUCE_COUNT)
    return 0;
  i = yy_reduce_ofst[state] + nt;
  if (i < 0 || i >= YY_ACTTAB_COUNT)
    return 0;
  return yy_lookahead[i] == nt;
}

/* syntaqlite extension: determine the semantic completion context
** (Expression vs TableRef) by walking the parser stack. Returns:
**   0 = Unknown, 1 = Expression, 2 = TableRef. */
uint32_t SynqSqliteParseCompletionContext(void* parser) {
  yyParser* p = (yyParser*)parser;
  if (p == 0 || p->yytos == 0)
    return 0;

  for (yyStackEntry* e = p->yytos; e >= p->yystack; e--) {
    YYACTIONTYPE s = e->stateno;

    /* Check if this state has gotos for table-ref non-terminals. */
    if (synq_has_goto(s, SYNQ_NT_SELTABLIST) ||
        synq_has_goto(s, SYNQ_NT_FULLNAME) ||
        synq_has_goto(s, SYNQ_NT_XFULLNAME)) {
      return 2; /* TableRef */
    }

    /* Check if this state has gotos for expression non-terminals. */
    if (synq_has_goto(s, SYNQ_NT_EXPR)) {
      return 1; /* Expression */
    }
  }
  return 0; /* Unknown */
}
