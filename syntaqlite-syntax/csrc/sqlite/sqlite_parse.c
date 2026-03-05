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

#include "syntaqlite_dialect/ast_builder.h"
#include "syntaqlite_dialect/dialect_macros.h"
#include "syntaqlite/types.h"
#include "csrc/sqlite/dialect_builder.h"

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
#define SYNTAQLITE_TK_ABORT                           1
#define SYNTAQLITE_TK_ACTION                          2
#define SYNTAQLITE_TK_AFTER                           3
#define SYNTAQLITE_TK_ANALYZE                         4
#define SYNTAQLITE_TK_ASC                             5
#define SYNTAQLITE_TK_ATTACH                          6
#define SYNTAQLITE_TK_BEFORE                          7
#define SYNTAQLITE_TK_BEGIN                           8
#define SYNTAQLITE_TK_BY                              9
#define SYNTAQLITE_TK_CASCADE                        10
#define SYNTAQLITE_TK_CAST                           11
#define SYNTAQLITE_TK_CONFLICT                       12
#define SYNTAQLITE_TK_DATABASE                       13
#define SYNTAQLITE_TK_DEFERRED                       14
#define SYNTAQLITE_TK_DESC                           15
#define SYNTAQLITE_TK_DETACH                         16
#define SYNTAQLITE_TK_EACH                           17
#define SYNTAQLITE_TK_END                            18
#define SYNTAQLITE_TK_EXCLUSIVE                      19
#define SYNTAQLITE_TK_EXPLAIN                        20
#define SYNTAQLITE_TK_FAIL                           21
#define SYNTAQLITE_TK_OR                             22
#define SYNTAQLITE_TK_AND                            23
#define SYNTAQLITE_TK_NOT                            24
#define SYNTAQLITE_TK_IS                             25
#define SYNTAQLITE_TK_ISNOT                          26
#define SYNTAQLITE_TK_MATCH                          27
#define SYNTAQLITE_TK_LIKE_KW                        28
#define SYNTAQLITE_TK_BETWEEN                        29
#define SYNTAQLITE_TK_IN                             30
#define SYNTAQLITE_TK_ISNULL                         31
#define SYNTAQLITE_TK_NOTNULL                        32
#define SYNTAQLITE_TK_NE                             33
#define SYNTAQLITE_TK_EQ                             34
#define SYNTAQLITE_TK_GT                             35
#define SYNTAQLITE_TK_LE                             36
#define SYNTAQLITE_TK_LT                             37
#define SYNTAQLITE_TK_GE                             38
#define SYNTAQLITE_TK_ESCAPE                         39
#define SYNTAQLITE_TK_ID                             40
#define SYNTAQLITE_TK_COLUMNKW                       41
#define SYNTAQLITE_TK_DO                             42
#define SYNTAQLITE_TK_FOR                            43
#define SYNTAQLITE_TK_IGNORE                         44
#define SYNTAQLITE_TK_IMMEDIATE                      45
#define SYNTAQLITE_TK_INITIALLY                      46
#define SYNTAQLITE_TK_INSTEAD                        47
#define SYNTAQLITE_TK_NO                             48
#define SYNTAQLITE_TK_PLAN                           49
#define SYNTAQLITE_TK_QUERY                          50
#define SYNTAQLITE_TK_KEY                            51
#define SYNTAQLITE_TK_OF                             52
#define SYNTAQLITE_TK_OFFSET                         53
#define SYNTAQLITE_TK_PRAGMA                         54
#define SYNTAQLITE_TK_RAISE                          55
#define SYNTAQLITE_TK_RECURSIVE                      56
#define SYNTAQLITE_TK_RELEASE                        57
#define SYNTAQLITE_TK_REPLACE                        58
#define SYNTAQLITE_TK_RESTRICT                       59
#define SYNTAQLITE_TK_ROW                            60
#define SYNTAQLITE_TK_ROWS                           61
#define SYNTAQLITE_TK_ROLLBACK                       62
#define SYNTAQLITE_TK_SAVEPOINT                      63
#define SYNTAQLITE_TK_TEMP                           64
#define SYNTAQLITE_TK_TRIGGER                        65
#define SYNTAQLITE_TK_VACUUM                         66
#define SYNTAQLITE_TK_VIEW                           67
#define SYNTAQLITE_TK_VIRTUAL                        68
#define SYNTAQLITE_TK_WITH                           69
#define SYNTAQLITE_TK_WITHOUT                        70
#define SYNTAQLITE_TK_NULLS                          71
#define SYNTAQLITE_TK_FIRST                          72
#define SYNTAQLITE_TK_LAST                           73
#define SYNTAQLITE_TK_CURRENT                        74
#define SYNTAQLITE_TK_FOLLOWING                      75
#define SYNTAQLITE_TK_PARTITION                      76
#define SYNTAQLITE_TK_PRECEDING                      77
#define SYNTAQLITE_TK_RANGE                          78
#define SYNTAQLITE_TK_UNBOUNDED                      79
#define SYNTAQLITE_TK_EXCLUDE                        80
#define SYNTAQLITE_TK_GROUPS                         81
#define SYNTAQLITE_TK_OTHERS                         82
#define SYNTAQLITE_TK_TIES                           83
#define SYNTAQLITE_TK_GENERATED                      84
#define SYNTAQLITE_TK_ALWAYS                         85
#define SYNTAQLITE_TK_WITHIN                         86
#define SYNTAQLITE_TK_MATERIALIZED                   87
#define SYNTAQLITE_TK_REINDEX                        88
#define SYNTAQLITE_TK_RENAME                         89
#define SYNTAQLITE_TK_CTIME_KW                       90
#define SYNTAQLITE_TK_IF                             91
#define SYNTAQLITE_TK_ANY                            92
#define SYNTAQLITE_TK_BITAND                         93
#define SYNTAQLITE_TK_BITOR                          94
#define SYNTAQLITE_TK_LSHIFT                         95
#define SYNTAQLITE_TK_RSHIFT                         96
#define SYNTAQLITE_TK_PLUS                           97
#define SYNTAQLITE_TK_MINUS                          98
#define SYNTAQLITE_TK_STAR                           99
#define SYNTAQLITE_TK_SLASH                          100
#define SYNTAQLITE_TK_REM                            101
#define SYNTAQLITE_TK_CONCAT                         102
#define SYNTAQLITE_TK_PTR                            103
#define SYNTAQLITE_TK_COLLATE                        104
#define SYNTAQLITE_TK_BITNOT                         105
#define SYNTAQLITE_TK_ON                             106
#define SYNTAQLITE_TK_INDEXED                        107
#define SYNTAQLITE_TK_STRING                         108
#define SYNTAQLITE_TK_JOIN_KW                        109
#define SYNTAQLITE_TK_INTEGER                        110
#define SYNTAQLITE_TK_FLOAT                          111
#define SYNTAQLITE_TK_SEMI                           112
#define SYNTAQLITE_TK_LP                             113
#define SYNTAQLITE_TK_ORDER                          114
#define SYNTAQLITE_TK_RP                             115
#define SYNTAQLITE_TK_GROUP                          116
#define SYNTAQLITE_TK_AS                             117
#define SYNTAQLITE_TK_COMMA                          118
#define SYNTAQLITE_TK_DOT                            119
#define SYNTAQLITE_TK_UNION                          120
#define SYNTAQLITE_TK_ALL                            121
#define SYNTAQLITE_TK_EXCEPT                         122
#define SYNTAQLITE_TK_INTERSECT                      123
#define SYNTAQLITE_TK_EXISTS                         124
#define SYNTAQLITE_TK_NULL                           125
#define SYNTAQLITE_TK_DISTINCT                       126
#define SYNTAQLITE_TK_FROM                           127
#define SYNTAQLITE_TK_CASE                           128
#define SYNTAQLITE_TK_WHEN                           129
#define SYNTAQLITE_TK_THEN                           130
#define SYNTAQLITE_TK_ELSE                           131
#define SYNTAQLITE_TK_TABLE                          132
#define SYNTAQLITE_TK_CONSTRAINT                     133
#define SYNTAQLITE_TK_DEFAULT                        134
#define SYNTAQLITE_TK_PRIMARY                        135
#define SYNTAQLITE_TK_UNIQUE                         136
#define SYNTAQLITE_TK_CHECK                          137
#define SYNTAQLITE_TK_REFERENCES                     138
#define SYNTAQLITE_TK_AUTOINCR                       139
#define SYNTAQLITE_TK_INSERT                         140
#define SYNTAQLITE_TK_DELETE                         141
#define SYNTAQLITE_TK_UPDATE                         142
#define SYNTAQLITE_TK_SET                            143
#define SYNTAQLITE_TK_DEFERRABLE                     144
#define SYNTAQLITE_TK_FOREIGN                        145
#define SYNTAQLITE_TK_INTO                           146
#define SYNTAQLITE_TK_VALUES                         147
#define SYNTAQLITE_TK_WHERE                          148
#define SYNTAQLITE_TK_RETURNING                      149
#define SYNTAQLITE_TK_NOTHING                        150
#define SYNTAQLITE_TK_BLOB                           151
#define SYNTAQLITE_TK_QNUMBER                        152
#define SYNTAQLITE_TK_VARIABLE                       153
#define SYNTAQLITE_TK_DROP                           154
#define SYNTAQLITE_TK_INDEX                          155
#define SYNTAQLITE_TK_ALTER                          156
#define SYNTAQLITE_TK_TO                             157
#define SYNTAQLITE_TK_ADD                            158
#define SYNTAQLITE_TK_COMMIT                         159
#define SYNTAQLITE_TK_TRANSACTION                    160
#define SYNTAQLITE_TK_SELECT                         161
#define SYNTAQLITE_TK_HAVING                         162
#define SYNTAQLITE_TK_LIMIT                          163
#define SYNTAQLITE_TK_JOIN                           164
#define SYNTAQLITE_TK_USING                          165
#define SYNTAQLITE_TK_CREATE                         166
#define SYNTAQLITE_TK_WINDOW                         167
#define SYNTAQLITE_TK_OVER                           168
#define SYNTAQLITE_TK_FILTER                         169
#define SYNTAQLITE_TK_COLUMN                         170
#define SYNTAQLITE_TK_AGG_FUNCTION                   171
#define SYNTAQLITE_TK_AGG_COLUMN                     172
#define SYNTAQLITE_TK_TRUEFALSE                      173
#define SYNTAQLITE_TK_FUNCTION                       174
#define SYNTAQLITE_TK_UPLUS                          175
#define SYNTAQLITE_TK_UMINUS                         176
#define SYNTAQLITE_TK_TRUTH                          177
#define SYNTAQLITE_TK_REGISTER                       178
#define SYNTAQLITE_TK_VECTOR                         179
#define SYNTAQLITE_TK_SELECT_COLUMN                  180
#define SYNTAQLITE_TK_IF_NULL_ROW                    181
#define SYNTAQLITE_TK_ASTERISK                       182
#define SYNTAQLITE_TK_SPAN                           183
#define SYNTAQLITE_TK_ERROR                          184
#define SYNTAQLITE_TK_SPACE                          185
#define SYNTAQLITE_TK_COMMENT                        186
#define SYNTAQLITE_TK_ILLEGAL                        187
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
**    SynqSqliteParseTOKENTYPE     is the data type used for minor type for terminal
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
**                       which is SynqSqliteParseTOKENTYPE.  The entry in the union
**                       for terminal symbols is called "yy0".
**    YYSTACKDEPTH       is the maximum depth of the parser's stack.  If
**                       zero the stack is dynamically sized using realloc()
**    SynqSqliteParseARG_SDECL     A static variable declaration for the %extra_argument
**    SynqSqliteParseARG_PDECL     A parameter declaration for the %extra_argument
**    SynqSqliteParseARG_PARAM     Code to pass %extra_argument as a subroutine parameter
**    SynqSqliteParseARG_STORE     Code to store %extra_argument into yypParser
**    SynqSqliteParseARG_FETCH     Code to extract %extra_argument from yypParser
**    SynqSqliteParseCTX_*         As SynqSqliteParseARG_ except for %extra_context
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
# define INTERFACE 1
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
#define SynqSqliteParseARG_PDECL ,SynqParseCtx* pCtx
#define SynqSqliteParseARG_PARAM ,pCtx
#define SynqSqliteParseARG_FETCH SynqParseCtx* pCtx=yypParser->pCtx;
#define SynqSqliteParseARG_STORE yypParser->pCtx=pCtx;
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
#define YYNSTATE             596
#define YYNRULE              414
#define YYNRULE_WITH_ACTION  414
#define YYNTOKEN             188
#define YY_MAX_SHIFT         595
#define YY_MIN_SHIFTREDUCE   861
#define YY_MAX_SHIFTREDUCE   1274
#define YY_ERROR_ACTION      1275
#define YY_ACCEPT_ACTION     1276
#define YY_NO_ACTION         1277
#define YY_MIN_REDUCE        1278
#define YY_MAX_REDUCE        1691
#define YY_MIN_DSTRCTR       0
#define YY_MAX_DSTRCTR       0
/************* End control #defines *******************************************/
#define YY_NLOOKAHEAD ((int)(sizeof(yy_lookahead)/sizeof(yy_lookahead[0])))

/* Define the yytestcase() macro to be a no-op if is not already defined
** otherwise.
**
** Applications can choose to define yytestcase() in the %include section
** to a macro that can assist in verifying code coverage.  For production
** code the yytestcase() macro should be turned off.  But it is useful
** for testing.
*/
#ifndef yytestcase
# define yytestcase(X)
#endif

/* Macro to determine if stack space has the ability to grow using
** heap memory.
*/
#if YYSTACKDEPTH<=0 || YYDYNSTACK
# define YYGROWABLESTACK 1
#else
# define YYGROWABLESTACK 0
#endif

/* Guarantee a minimum number of initial stack slots.
*/
#if YYSTACKDEPTH<=0
# undef YYSTACKDEPTH
# define YYSTACKDEPTH 2  /* Need a minimum stack size */
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
#define YY_ACTTAB_COUNT (2383)
static const YYACTIONTYPE yy_action[] = {
 /*     0 */     6,  317,  212, 1060, 1060, 1072, 1669,  212,   97,   99,
 /*    10 */   295, 1533,  530,   97,   99, 1073, 1608,  430, 1563,  295,
 /*    20 */  1533,  297,   90,   91,  420,   48, 1060,  907,  907,  904,
 /*    30 */   889,  898,  898,   92,   92,   93,   93,   93,   93, 1405,
 /*    40 */  1654,  406, 1360,  472,  395,   90,   91,  420,   48, 1346,
 /*    50 */   907,  907,  904,  889,  898,  898,   92,   92,   93,   93,
 /*    60 */    93,   93,  578, 1349,  111,  295, 1533,  410, 1060,  343,
 /*    70 */    93,   93,   93,   93,   96,  588,   69,    6, 1352, 1060,
 /*    80 */  1351,  540,  403, 1666,  279,  107, 1403,  207,  908,  908,
 /*    90 */   905,  890,  325,   89,   89,   89,   89,   95,   95,   94,
 /*   100 */    94,   94,   88,   87,  453,  571,  569, 1607,  464,  465,
 /*   110 */   491,  532,  453, 1454,  571,  569,   89,   89,   89,   89,
 /*   120 */    95,   95,   94,   94,   94,   88,   87,  453,   89,   89,
 /*   130 */    89,   89,   95,   95,   94,   94,   94,   88,   87,  453,
 /*   140 */  1386,   68,   90,   91,  420,   48, 1060,  907,  907,  904,
 /*   150 */   889,  898,  898,   92,   92,   93,   93,   93,   93,  430,
 /*   160 */   571,  569,  484,  361,   90,   91,  420,   48, 1060,  907,
 /*   170 */   907,  904,  889,  898,  898,   92,   92,   93,   93,   93,
 /*   180 */    93,  540,   86,  578,   80,   44,  899,  389,  431,  394,
 /*   190 */  1627, 1674, 1662,   52,  533, 1261,  588, 1261,   89,   89,
 /*   200 */    89,   89,   95,   95,   94,   94,   94,   88,   87,  453,
 /*   210 */   388,  259, 1475,   89,   89,   89,   89,   95,   95,   94,
 /*   220 */    94,   94,   88,   87,  453,   95,   95,   94,   94,   94,
 /*   230 */    88,   87,  453, 1006, 1454,   89,   89,   89,   89,   95,
 /*   240 */    95,   94,   94,   94,   88,   87,  453,   90,   91,  420,
 /*   250 */    48,  532,  907,  907,  904,  889,  898,  898,   92,   92,
 /*   260 */    93,   93,   93,   93,   94,   94,   94,   88,   87,  453,
 /*   270 */  1081, 1061,   90,   91,  420,   48,  497,  907,  907,  904,
 /*   280 */   889,  898,  898,   92,   92,   93,   93,   93,   93, 1061,
 /*   290 */  1082,  541, 1661, 1061,  589,   90,   91,  420,   48, 1373,
 /*   300 */   907,  907,  904,  889,  898,  898,   92,   92,   93,   93,
 /*   310 */    93,   93, 1061,  575,   73,  359,  477,  346,   89,   89,
 /*   320 */    89,   89,   95,   95,   94,   94,   94,   88,   87,  453,
 /*   330 */   879, 1080,  209,   65,  533,  950,  404, 1687, 1061, 1062,
 /*   340 */  1061, 1151,   53,   89,   89,   89,   89,   95,   95,   94,
 /*   350 */    94,   94,   88,   87,  453,   12, 1061, 1062, 1061,  498,
 /*   360 */  1061, 1062, 1061,  295, 1533,  306,   89,   89,   89,   89,
 /*   370 */    95,   95,   94,   94,   94,   88,   87,  453, 1060, 1061,
 /*   380 */  1062, 1061,  241, 1372,  520,  517,  516, 1151,  372, 1615,
 /*   390 */  1616, 1608,  534,  105,  515,   90,   91,  420,   48,  266,
 /*   400 */   907,  907,  904,  889,  898,  898,   92,   92,   93,   93,
 /*   410 */    93,   93,  295, 1533, 1639, 1300, 1371, 1153,   90,   91,
 /*   420 */   420,   48,  580,  907,  907,  904,  889,  898,  898,   92,
 /*   430 */    92,   93,   93,   93,   93, 1061,  324,  295, 1533, 1338,
 /*   440 */  1561,   90,   91,  420,   48,  500,  907,  907,  904,  889,
 /*   450 */   898,  898,   92,   92,   93,   93,   93,   93,  571,  569,
 /*   460 */   292, 1533, 1061,  213,  549,  245,   89,   89,   89,   89,
 /*   470 */    95,   95,   94,   94,   94,   88,   87,  453,   36,   88,
 /*   480 */    87,  453, 1607, 1605, 1603, 1061, 1425, 1398,  521,   89,
 /*   490 */    89,   89,   89,   95,   95,   94,   94,   94,   88,   87,
 /*   500 */   453,   37, 1061, 1062, 1061, 1426, 1426,  571,  569,  571,
 /*   510 */   569,  941,   89,   89,   89,   89,   95,   95,   94,   94,
 /*   520 */    94,   88,   87,  453, 1369,  580, 1396,   34, 1394, 1061,
 /*   530 */  1062, 1061,  571,  569,  933,   90,   91,  420,   48,  324,
 /*   540 */   907,  907,  904,  889,  898,  898,   92,   92,   93,   93,
 /*   550 */    93,   93, 1061, 1062, 1061,  571,  569, 1097,   90,   91,
 /*   560 */   420,   48,  547,  907,  907,  904,  889,  898,  898,   92,
 /*   570 */    92,   93,   93,   93,   93,  295, 1533,  581, 1167, 1167,
 /*   580 */   505,   90,   91,  420,   48,  248,  907,  907,  904,  889,
 /*   590 */   898,  898,   92,   92,   93,   93,   93,   93,  295, 1533,
 /*   600 */   584,  456,  463,  296, 1284,  580,   89,   89,   89,   89,
 /*   610 */    95,   95,   94,   94,   94,   88,   87,  453,  424,  324,
 /*   620 */   295, 1533, 1531, 1164,  293, 1533, 1681, 1164,  194,   89,
 /*   630 */    89,   89,   89,   95,   95,   94,   94,   94,   88,   87,
 /*   640 */   453,    6, 1476,  212,  213,  549,  313, 1667,  182,   97,
 /*   650 */    99, 1274,   89,   89,   89,   89,   95,   95,   94,   94,
 /*   660 */    94,   88,   87,  453,  275, 1288,  335,  503,  337,  501,
 /*   670 */   571,  569, 1061,  226, 1055,   90,   91,  420,   48,  482,
 /*   680 */   907,  907,  904,  889,  898,  898,   92,   92,   93,   93,
 /*   690 */    93,   93, 1061,  571,  569, 1061, 1615, 1616,   90,   91,
 /*   700 */   420,   48,  271,  907,  907,  904,  889,  898,  898,   92,
 /*   710 */    92,   93,   93,   93,   93,  571,  569,  462, 1061,  571,
 /*   720 */   569,   90,   91,  420,   48,   65,  907,  907,  904,  889,
 /*   730 */   898,  898,   92,   92,   93,   93,   93,   93, 1621, 1061,
 /*   740 */  1062, 1061,  338,  574,  340,  177,   89,   89,   89,   89,
 /*   750 */    95,   95,   94,   94,   94,   88,   87,  453, 1286, 1061,
 /*   760 */  1062, 1061, 1061, 1062, 1061,  178,  404, 1687, 1038,   89,
 /*   770 */    89,   89,   89,   95,   95,   94,   94,   94,   88,   87,
 /*   780 */   453, 1162, 1515,  426,  534, 1061, 1062, 1061, 1061,  404,
 /*   790 */  1687,    5,   89,   89,   89,   89,   95,   95,   94,   94,
 /*   800 */    94,   88,   87,  453,  212,  583,   78,  886,  886,  341,
 /*   810 */    97,   99, 1061,   12, 1079,   90,   91,  420,   48, 1152,
 /*   820 */   907,  907,  904,  889,  898,  898,   92,   92,   93,   93,
 /*   830 */    93,   93,  339,  368,  878,  364,  509,   90,   91,  420,
 /*   840 */    48,  396,  907,  907,  904,  889,  898,  898,   92,   92,
 /*   850 */    93,   93,   93,   93,  204, 1061, 1062, 1061,  278,  277,
 /*   860 */   276,    2,   90,   91,  420,   48,  242,  907,  907,  904,
 /*   870 */   889,  898,  898,   92,   92,   93,   93,   93,   93, 1061,
 /*   880 */  1062, 1061,  404, 1687,   66,   32,   89,   89,   89,   89,
 /*   890 */    95,   95,   94,   94,   94,   88,   87,  453,  543,  256,
 /*   900 */   370,  284,  878,  483,  484,  361, 1243,  189,   89,   89,
 /*   910 */    89,   89,   95,   95,   94,   94,   94,   88,   87,  453,
 /*   920 */  1128,  291, 1224,   85,  212, 1127,  479,  405,  314, 1242,
 /*   930 */    97,   99,  263,   89,   89,   89,   89,   95,   95,   94,
 /*   940 */    94,   94,   88,   87,  453,  425,   90,   98,  420,   48,
 /*   950 */   315,  907,  907,  904,  889,  898,  898,   92,   92,   93,
 /*   960 */    93,   93,   93,   91,  420,   48,  316,  907,  907,  904,
 /*   970 */   889,  898,  898,   92,   92,   93,   93,   93,   93,  420,
 /*   980 */    48,  383,  907,  907,  904,  889,  898,  898,   92,   92,
 /*   990 */    93,   93,   93,   93, 1218,    6,  578, 1060,  111, 1060,
 /*  1000 */   382, 1665,  591, 1006,   93,   93,   93,   93, 1061,  588,
 /*  1010 */  1343, 1536,  499, 1636,   34,   82,  432,   89,   89,   89,
 /*  1020 */    89,   95,   95,   94,   94,   94,   88,   87,  453,   34,
 /*  1030 */   196,  459, 1341,   89,   89,   89,   89,   95,   95,   94,
 /*  1040 */    94,   94,   88,   87,  453, 1125,  576, 1454,   89,   89,
 /*  1050 */    89,   89,   95,   95,   94,   94,   94,   88,   87,  453,
 /*  1060 */    34,  591,   89,   89,   89,   89,   95,   95,   94,   94,
 /*  1070 */    94,   88,   87,  453,   82, 1061, 1062, 1061, 1060,    8,
 /*  1080 */   578, 1067,  111,  936,  578,  359,  145,   14,   84,   84,
 /*  1090 */   459,  495,  580,  588,  471,  301,   83,  588,  459,  577,
 /*  1100 */   459, 1063, 1065,    6,    4,  576,  324,  580,  241, 1664,
 /*  1110 */   520,  517,  516, 1125,   50,  585, 1065, 1256,    9,   24,
 /*  1120 */   515,  324,  249,  557,  555, 1674, 1060,   11,  578,  554,
 /*  1130 */   145, 1454,  531, 1067, 1256, 1454,  591, 1256,  580,  261,
 /*  1140 */  1067,  588, 1065, 1066, 1068, 1061,  225,   84,   84,   82,
 /*  1150 */   936, 1064,  324, 1063, 1065,   83, 1060,  459,  577,  459,
 /*  1160 */  1063, 1065,  536,    4, 1060,  459, 1256,  561, 1065,  508,
 /*  1170 */  1238,  455,  454, 1420,  585, 1065,  408,  386,   24, 1454,
 /*  1180 */   576,  592,  444, 1256, 1196, 1196, 1256, 1060,  590,  426,
 /*  1190 */  1256, 1240,  298, 1651, 1065, 1066, 1651,  427,  214,  555,
 /*  1200 */  1578, 1065, 1066, 1068,  556,  553,  563, 1256,  393, 1676,
 /*  1210 */  1256,  591, 1061, 1062, 1061, 1067,  578,  877,   44,  551,
 /*  1220 */   104,  434,   84,   84,   82, 1279,  595,  594, 1284,  588,
 /*  1230 */    83,  485,  459,  577,  459, 1063, 1065,  473,    4,  356,
 /*  1240 */   459, 1507,  523,  552,  295, 1533, 1531, 1060, 1434,  585,
 /*  1250 */  1065, 1060,  578,   24,   44,  576, 1060,  440, 1060,  204,
 /*  1260 */  1343, 1128,  441,  436,  261,  588, 1127, 1454,  269, 1405,
 /*  1270 */   313,  493,  182, 1060,  555,  376, 1065, 1066, 1068,  554,
 /*  1280 */   561,  528, 1340,  384,   75,  877, 1405,  578,  275,  152,
 /*  1290 */  1067,  244,  286,  526,  379,  525,  243,   84,   84,  385,
 /*  1300 */   588,  413,  375, 1454,  578,   83,  156,  459,  577,  459,
 /*  1310 */  1063, 1065,  403,    4, 1278,  109, 1403,  588,  217,  345,
 /*  1320 */   327, 1072,  283, 1060,  585, 1065, 1067,  299,   24,  403,
 /*  1330 */   326, 1073,  334, 1404,  461,    3,  261,  591, 1454,  571,
 /*  1340 */   569,  462, 1081, 1103, 1064,  348, 1063, 1065, 1105,  300,
 /*  1350 */    82, 1065, 1066, 1068,  578, 1454,  145,  354,  562, 1060,
 /*  1360 */  1434, 1065, 1082, 1387,  310, 1060,  459,  588,  218, 1238,
 /*  1370 */  1243,  331, 1405,  418, 1104,  957,  333,  220,  430,  290,
 /*  1380 */   160,  576,  207,   33,  958, 1010, 1385, 1065, 1066,  971,
 /*  1390 */  1240,  405, 1652, 1239,  578, 1652,  145, 1060,  565, 1011,
 /*  1400 */   475,  419,  216, 1080,  578, 1454,  145,  588,  458, 1060,
 /*  1410 */  1405,  295, 1533,  564,  591,  403, 1067,  588,  106, 1403,
 /*  1420 */   476,  419,  499,   84,   84, 1060,  864,   47,  417,  416,
 /*  1430 */   260,   83,  563,  459,  577,  459, 1063, 1065,  535,    4,
 /*  1440 */   486,  419, 1448,  459,   64, 1454,  578,  591,  121, 1060,
 /*  1450 */   585, 1065,  573,  403,   24, 1454,  108, 1403,  576,  588,
 /*  1460 */    82,  580,  399,  295, 1533,  586,  490,  419,  421,   29,
 /*  1470 */   469, 1060,  563,  334, 1125,  324,  459, 1065, 1066, 1068,
 /*  1480 */  1223,  578,  563,   44,  208,  578,  309,   44,  318,  419,
 /*  1490 */   250,  576,  965, 1067,  588,  103, 1060, 1454,  588,   40,
 /*  1500 */    84,   84,  578,  439,  145,  358,  571,  569,   83,  211,
 /*  1510 */   459,  577,  459, 1063, 1065,  588,    4,  578,  512,   44,
 /*  1520 */   567,  242,  251,  966,  566, 1060, 1067,  585, 1065,  572,
 /*  1530 */   588,   24, 1454,   84,   84, 1120, 1454, 1060,  493, 1060,
 /*  1540 */   559,   83, 1125,  459,  577,  459, 1063, 1065,   28,    4,
 /*  1550 */   493,  249,  499, 1454, 1065, 1066, 1068,  591,  571,  569,
 /*  1560 */   585, 1065, 1447, 1060,   24,  217,  558,  327, 1454,  283,
 /*  1570 */    82, 1060,   55,  265,  489, 1267,  587,  326,  944,  334,
 /*  1580 */   319,  461, 1232,  990, 1578,  407,  459, 1065, 1066, 1068,
 /*  1590 */  1060,  578, 1060,   44,  302,  496,  560,  578, 1060,   44,
 /*  1600 */   425,  576,  457, 1578,  588,  493,  305,  578, 1267,  145,
 /*  1610 */   588,  211,  578,  294,   46,  218,    6,  268,  331, 1478,
 /*  1620 */   588,  538, 1668,  333,  220,  542,  355,  160, 1075, 1076,
 /*  1630 */    33,  209,  578,  304,  145,  308, 1067,  288,  375,  578,
 /*  1640 */  1151,  143, 1454,   84,   84,  588,  944,  437, 1454,  216,
 /*  1650 */   411,   83,  588,  459,  577,  459, 1063, 1065, 1454,    4,
 /*  1660 */   518,  307,  401, 1454,  360,  227,  438,  229, 1060, 1060,
 /*  1670 */   585, 1065,  274,  864,   24,  217, 1369,  327,  578,  283,
 /*  1680 */   145,  398, 1690, 1454, 1061,  450, 1151,  326, 1335,  334,
 /*  1690 */  1454,  588,  381, 1060, 1060, 1121, 1060, 1065, 1066, 1068,
 /*  1700 */  1060, 1060,  351, 1060,  269,   73, 1578,  508,  580,  328,
 /*  1710 */   451, 1197, 1197, 1578,  329,  421,  508,  469,    6,  578,
 /*  1720 */   334,  145,  324,  550, 1668,  218, 1060, 1223,  331, 1454,
 /*  1730 */  1566,  997,  588,  333,  220, 1567,  578,  160,  126,  508,
 /*  1740 */    33,  455,  454, 1276,    1, 1280,  595,  594, 1284,  588,
 /*  1750 */  1192, 1061, 1062, 1061, 1196, 1196,  452, 1565,  578,  216,
 /*  1760 */   127,  578,  257,  128,  295, 1533, 1531,  593,  190,  504,
 /*  1770 */  1454,  588,  959,  578,  588,   45,  445, 1564, 1194, 1506,
 /*  1780 */   578, 1256,  112,  392,  258, 1193,  588, 1454, 1505,  578,
 /*  1790 */   313,  113,  182,  588,  578, 1411,  129,  320, 1256,  997,
 /*  1800 */   579, 1256,  588,  578, 1060,  130, 1412,  588,  275, 1454,
 /*  1810 */   524, 1504, 1454,  578,  270,  131,  588,  330,  580,   50,
 /*  1820 */   578,  960,  132, 1628, 1454,  421,  588,  469, 1060, 1060,
 /*  1830 */   334, 1454,  324,  588,  578, 1060,  133, 1223,   73, 1620,
 /*  1840 */  1454, 1529,  508, 1206,  578, 1454,  114,  588, 1528,  578,
 /*  1850 */   578,  115,  116,   51, 1454,    3,  578,  588,  117,  571,
 /*  1860 */   569,  462,  588,  588, 1454,   54,  578,  252,  134,  588,
 /*  1870 */   578, 1454,  135,  578, 1060,  136,  578, 1618,  137,  588,
 /*  1880 */   578, 1109,  138,  588, 1060, 1454,  588, 1530,  428,  588,
 /*  1890 */   578, 1060,  110,  588,  487, 1454, 1060, 1525, 1195, 1195,
 /*  1900 */  1454, 1454,  303,  588,  399,  442, 1060, 1454,  357, 1510,
 /*  1910 */   578,   73,  118,  578,  366,  119,  578, 1454,   43, 1509,
 /*  1920 */   578, 1454,  120,  588, 1454, 1027,  588, 1454,  247,  588,
 /*  1930 */   578, 1454,  139,  588,  578,  578,  150,  151,  578,  222,
 /*  1940 */   140, 1454,  578,  588,  122,  448,  466,  588,  588, 1060,
 /*  1950 */   492,  588,  578,  247,  141,  588,  578,  578,  123,  147,
 /*  1960 */   219, 1454,  508, 1060, 1454,  588,  578, 1454,  199,  588,
 /*  1970 */   588, 1454,  221, 1069, 1060,  578,  468,  200,  578,  588,
 /*  1980 */   142, 1454,  578, 1517,  124, 1454, 1454, 1508,  588, 1454,
 /*  1990 */  1516,  588, 1097, 1454,  470,  588,  578, 1060,  197,  166,
 /*  2000 */   578, 1060,  198, 1454,  167,  223,  168, 1454, 1454,  588,
 /*  2010 */  1432, 1060,  169,  588, 1431, 1060,  170, 1454,  578,  578,
 /*  2020 */   158,  146,  267,  578,  474,  148, 1454,  494, 1650, 1454,
 /*  2030 */   247,  588,  588, 1454,  429,  467,  588,  578, 1421,  153,
 /*  2040 */   578, 1069,  157,  578,  578,  191,  159, 1454,  175,   70,
 /*  2050 */   588, 1454,  578,  588,  154,   35,  588,  588, 1419,  578,
 /*  2060 */   561,  149,  578, 1060,  155,  588,  578, 1060,  144, 1454,
 /*  2070 */  1454,  578,  588,  125, 1454,  588,  349, 1060,  513,  588,
 /*  2080 */   480,  253,  205, 1060,  588, 1060, 1060,  344, 1454, 1060,
 /*  2090 */   352, 1454,  172, 1060, 1454, 1454, 1596,  373, 1594,  363,
 /*  2100 */    73, 1156, 1501, 1454,  247, 1060,  367, 1060, 1060, 1418,
 /*  2110 */  1454, 1060, 1060, 1454, 1060, 1060, 1060, 1454,  369, 1060,
 /*  2120 */   371, 1390, 1454, 1060, 1370,  378, 1060, 1348, 1342, 1575,
 /*  2130 */  1060, 1060, 1577,  478,  993, 1228, 1541,  253,   75, 1312,
 /*  2140 */   347,  511, 1227,  400, 1299,   75, 1226,  875,  195,   75,
 /*  2150 */   188,   73,   66,  231,  187,  481, 1634,  233,  409,  488,
 /*  2160 */  1450, 1449,   38, 1422,  179,  362,  502,  412,  236,   60,
 /*  2170 */  1580,  507,  365,  285, 1336,  238,  239,  414,  527, 1393,
 /*  2180 */   443, 1392, 1391, 1380,   62,  415,  950, 1357, 1363, 1362,
 /*  2190 */  1356,  380, 1379, 1355,   30,  280, 1354,  529,   67,  537,
 /*  2200 */   281, 1535,  387,  446, 1534, 1672,   10,  262,  390, 1310,
 /*  2210 */   397,  391,  447, 1671,   79,  323, 1487,  449,  582, 1488,
 /*  2220 */   321,  402,  322,  213,  201, 1686, 1600, 1601,  422, 1599,
 /*  2230 */  1598,  202,  183,  311,  272,  273,  203, 1216,   49,  460,
 /*  2240 */   264,  423, 1214,  332, 1189,  336, 1187,  224,  102, 1085,
 /*  2250 */   342,  171,  161,  228,  230, 1121,  350,  215,   13,  232,
 /*  2260 */   173,  353, 1175,  433,  162,  174,  163,  435,  176,   56,
 /*  2270 */    57,   58,   59, 1180,  234,  235, 1174,  192,  164,   39,
 /*  2280 */  1165,  247,  180, 1171,  506,  181,  237,  510,  382, 1221,
 /*  2290 */   240,  184,  514,   61,   15,  519,   16,  374,  948,  522,
 /*  2300 */   961,  377,  312,   63,  206,  165,  101,  287,  289, 1159,
 /*  2310 */   246,  185, 1154,   75,   17, 1246,   31,  186,  539,   71,
 /*  2320 */   193,  544,  545,  210,  546,   72,  548, 1272,   18,   19,
 /*  2330 */    20, 1258, 1262, 1266, 1260,    7,   73,   21, 1265,  897,
 /*  2340 */   892,  891,   22,   76,  991,   74,   23,   77,  911,  568,
 /*  2350 */   570,   26,   81,  282, 1078,   27, 1277, 1277,   25,   41,
 /*  2360 */  1277, 1479, 1477,  985,  888,   42, 1277, 1277,  885,  254,
 /*  2370 */  1277, 1277,  255, 1277, 1277,  887,  876,  872, 1277,  100,
 /*  2380 */   866, 1277,  865,
};
static const YYCODETYPE yy_lookahead[] = {
 /*     0 */   312,  267,  207,  192,  192,    5,  318,  207,  213,  214,
 /*    10 */   209,  210,  211,  213,  214,   15,  205,  205,  284,  209,
 /*    20 */   210,  211,   22,   23,   24,   25,  192,   27,   28,   29,
 /*    30 */    30,   31,   32,   33,   34,   35,   36,   37,   38,  205,
 /*    40 */   309,  310,  223,  254,  244,   22,   23,   24,   25,  230,
 /*    50 */    27,   28,   29,   30,   31,   32,   33,   34,   35,   36,
 /*    60 */    37,   38,  192,  243,  194,  209,  210,  211,  192,  257,
 /*    70 */    35,   36,   37,   38,   39,  205,   53,  312,  243,  192,
 /*    80 */   243,  205,  248,  318,  283,  251,  252,  286,   27,   28,
 /*    90 */    29,   30,  205,   93,   94,   95,   96,   97,   98,   99,
 /*   100 */   100,  101,  102,  103,  104,  304,  305,  296,  297,  298,
 /*   110 */   254,   24,  104,  243,  304,  305,   93,   94,   95,   96,
 /*   120 */    97,   98,   99,  100,  101,  102,  103,  104,   93,   94,
 /*   130 */    95,   96,   97,   98,   99,  100,  101,  102,  103,  104,
 /*   140 */   232,  118,   22,   23,   24,   25,  192,   27,   28,   29,
 /*   150 */    30,   31,   32,   33,   34,   35,   36,   37,   38,  205,
 /*   160 */   304,  305,  141,  142,   22,   23,   24,   25,  192,   27,
 /*   170 */    28,   29,   30,   31,   32,   33,   34,   35,   36,   37,
 /*   180 */    38,  205,  129,  192,  131,  194,  125,  259,  197,  319,
 /*   190 */   303,  321,  316,   51,  107,   75,  205,   77,   93,   94,
 /*   200 */    95,   96,   97,   98,   99,  100,  101,  102,  103,  104,
 /*   210 */   282,  257,  198,   93,   94,   95,   96,   97,   98,   99,
 /*   220 */   100,  101,  102,  103,  104,   97,   98,   99,  100,  101,
 /*   230 */   102,  103,  104,   58,  243,   93,   94,   95,   96,   97,
 /*   240 */    98,   99,  100,  101,  102,  103,  104,   22,   23,   24,
 /*   250 */    25,   24,   27,   28,   29,   30,   31,   32,   33,   34,
 /*   260 */    35,   36,   37,   38,   99,  100,  101,  102,  103,  104,
 /*   270 */     1,   40,   22,   23,   24,   25,   24,   27,   28,   29,
 /*   280 */    30,   31,   32,   33,   34,   35,   36,   37,   38,   40,
 /*   290 */    21,  315,  316,   40,  201,   22,   23,   24,   25,  222,
 /*   300 */    27,   28,   29,   30,   31,   32,   33,   34,   35,   36,
 /*   310 */    37,   38,   40,   44,  118,  140,  141,  142,   93,   94,
 /*   320 */    95,   96,   97,   98,   99,  100,  101,  102,  103,  104,
 /*   330 */    99,   62,  109,  106,  107,  139,  322,  323,  107,  108,
 /*   340 */   109,  118,  117,   93,   94,   95,   96,   97,   98,   99,
 /*   350 */   100,  101,  102,  103,  104,  204,  107,  108,  109,  107,
 /*   360 */   107,  108,  109,  209,  210,  211,   93,   94,   95,   96,
 /*   370 */    97,   98,   99,  100,  101,  102,  103,  104,  192,  107,
 /*   380 */   108,  109,  133,  222,  135,  136,  137,  164,  115,  296,
 /*   390 */   297,  205,  165,  118,  145,   22,   23,   24,   25,  149,
 /*   400 */    27,   28,   29,   30,   31,   32,   33,   34,   35,   36,
 /*   410 */    37,   38,  209,  210,  211,  210,  222,  164,   22,   23,
 /*   420 */    24,   25,  147,   27,   28,   29,   30,   31,   32,   33,
 /*   430 */    34,   35,   36,   37,   38,   40,  161,  209,  210,  211,
 /*   440 */   285,   22,   23,   24,   25,  294,   27,   28,   29,   30,
 /*   450 */    31,   32,   33,   34,   35,   36,   37,   38,  304,  305,
 /*   460 */   209,  210,   40,  168,  169,   70,   93,   94,   95,   96,
 /*   470 */    97,   98,   99,  100,  101,  102,  103,  104,   56,  102,
 /*   480 */   103,  104,  296,  297,  298,   40,  247,  247,  115,   93,
 /*   490 */    94,   95,   96,   97,   98,   99,  100,  101,  102,  103,
 /*   500 */   104,   56,  107,  108,  109,  266,  266,  304,  305,  304,
 /*   510 */   305,  115,   93,   94,   95,   96,   97,   98,   99,  100,
 /*   520 */   101,  102,  103,  104,  221,  147,  223,   69,  225,  107,
 /*   530 */   108,  109,  304,  305,  115,   22,   23,   24,   25,  161,
 /*   540 */    27,   28,   29,   30,   31,   32,   33,   34,   35,   36,
 /*   550 */    37,   38,  107,  108,  109,  304,  305,   41,   22,   23,
 /*   560 */    24,   25,   86,   27,   28,   29,   30,   31,   32,   33,
 /*   570 */    34,   35,   36,   37,   38,  209,  210,  211,  140,  141,
 /*   580 */   142,   22,   23,   24,   25,  279,   27,   28,   29,   30,
 /*   590 */    31,   32,   33,   34,   35,   36,   37,   38,  209,  210,
 /*   600 */   211,  201,  191,   89,  193,  147,   93,   94,   95,   96,
 /*   610 */    97,   98,   99,  100,  101,  102,  103,  104,  238,  161,
 /*   620 */   209,  210,  211,    3,  209,  210,  314,    7,  115,   93,
 /*   630 */    94,   95,   96,   97,   98,   99,  100,  101,  102,  103,
 /*   640 */   104,  312,  198,  207,  168,  169,  235,  318,  237,  213,
 /*   650 */   214,  115,   93,   94,   95,   96,   97,   98,   99,  100,
 /*   660 */   101,  102,  103,  104,  253,  198,  273,   47,  154,  289,
 /*   670 */   304,  305,   40,  157,  115,   22,   23,   24,   25,  299,
 /*   680 */    27,   28,   29,   30,   31,   32,   33,   34,   35,   36,
 /*   690 */    37,   38,   40,  304,  305,   40,  296,  297,   22,   23,
 /*   700 */    24,   25,  218,   27,   28,   29,   30,   31,   32,   33,
 /*   710 */    34,   35,   36,   37,   38,  304,  305,  306,   40,  304,
 /*   720 */   305,   22,   23,   24,   25,  106,   27,   28,   29,   30,
 /*   730 */    31,   32,   33,   34,   35,   36,   37,   38,  302,  107,
 /*   740 */   108,  109,   65,  266,   67,  113,   93,   94,   95,   96,
 /*   750 */    97,   98,   99,  100,  101,  102,  103,  104,  198,  107,
 /*   760 */   108,  109,  107,  108,  109,  113,  322,  323,  115,   93,
 /*   770 */    94,   95,   96,   97,   98,   99,  100,  101,  102,  103,
 /*   780 */   104,   18,  199,  200,  165,  107,  108,  109,   40,  322,
 /*   790 */   323,  113,   93,   94,   95,   96,   97,   98,   99,  100,
 /*   800 */   101,  102,  103,  104,  207,  120,  130,  122,  123,  132,
 /*   810 */   213,  214,   40,  204,  115,   22,   23,   24,   25,  164,
 /*   820 */    27,   28,   29,   30,   31,   32,   33,   34,   35,   36,
 /*   830 */    37,   38,  155,   65,   40,   67,  291,   22,   23,   24,
 /*   840 */    25,  244,   27,   28,   29,   30,   31,   32,   33,   34,
 /*   850 */    35,   36,   37,   38,  118,  107,  108,  109,  140,  141,
 /*   860 */   142,  113,   22,   23,   24,   25,   27,   27,   28,   29,
 /*   870 */    30,   31,   32,   33,   34,   35,   36,   37,   38,  107,
 /*   880 */   108,  109,  322,  323,  148,  113,   93,   94,   95,   96,
 /*   890 */    97,   98,   99,  100,  101,  102,  103,  104,   99,  290,
 /*   900 */   132,  287,  108,  294,  141,  142,   92,  113,   93,   94,
 /*   910 */    95,   96,   97,   98,   99,  100,  101,  102,  103,  104,
 /*   920 */   121,  204,   64,  130,  207,  126,   68,  113,  279,  115,
 /*   930 */   213,  214,  117,   93,   94,   95,   96,   97,   98,   99,
 /*   940 */   100,  101,  102,  103,  104,  106,   22,   23,   24,   25,
 /*   950 */   279,   27,   28,   29,   30,   31,   32,   33,   34,   35,
 /*   960 */    36,   37,   38,   23,   24,   25,  279,   27,   28,   29,
 /*   970 */    30,   31,   32,   33,   34,   35,   36,   37,   38,   24,
 /*   980 */    25,  125,   27,   28,   29,   30,   31,   32,   33,   34,
 /*   990 */    35,   36,   37,   38,  136,  312,  192,  192,  194,  192,
 /*  1000 */   144,  318,   11,   58,   35,   36,   37,   38,   40,  205,
 /*  1010 */   205,  279,  205,  155,   69,   24,   42,   93,   94,   95,
 /*  1020 */    96,   97,   98,   99,  100,  101,  102,  103,  104,   69,
 /*  1030 */   206,   40,  227,   93,   94,   95,   96,   97,   98,   99,
 /*  1040 */   100,  101,  102,  103,  104,   40,   55,  243,   93,   94,
 /*  1050 */    95,   96,   97,   98,   99,  100,  101,  102,  103,  104,
 /*  1060 */    69,   11,   93,   94,   95,   96,   97,   98,   99,  100,
 /*  1070 */   101,  102,  103,  104,   24,  107,  108,  109,  192,   29,
 /*  1080 */   192,   90,  194,   40,  192,  140,  194,  113,   97,   98,
 /*  1090 */    40,  205,  147,  205,  134,  288,  105,  205,  107,  108,
 /*  1100 */   109,  110,  111,  312,  113,   55,  161,  147,  133,  318,
 /*  1110 */   135,  136,  137,  108,  146,  124,  125,   61,  113,  128,
 /*  1120 */   145,  161,  117,  319,   74,  321,  192,  195,  192,   79,
 /*  1130 */   194,  243,  196,   90,   78,  243,   11,   81,  147,  205,
 /*  1140 */    90,  205,  151,  152,  153,   40,  276,   97,   98,   24,
 /*  1150 */   107,  108,  161,  110,  111,  105,  192,  107,  108,  109,
 /*  1160 */   110,  111,  270,  113,  192,   40,   61,  114,  125,  205,
 /*  1170 */    92,   97,   98,  264,  124,  125,  242,  205,  128,  243,
 /*  1180 */    55,   76,   24,   78,  110,  111,   81,  192,  199,  200,
 /*  1190 */    61,  113,  262,  115,  151,  152,  118,  307,  308,   74,
 /*  1200 */   205,  151,  152,  153,   79,   76,  270,   78,  320,  321,
 /*  1210 */    81,   11,  107,  108,  109,   90,  192,   40,  194,  114,
 /*  1220 */   167,  197,   97,   98,   24,  190,  191,  192,  193,  205,
 /*  1230 */   105,  142,  107,  108,  109,  110,  111,  256,  113,  150,
 /*  1240 */    40,  277,   84,  114,  209,  210,  211,  192,  267,  124,
 /*  1250 */   125,  192,  192,  128,  194,   55,  192,  197,  192,  118,
 /*  1260 */   205,  121,  104,  268,  205,  205,  126,  243,  127,  205,
 /*  1270 */   235,  205,  237,  192,   74,  117,  151,  152,  153,   79,
 /*  1280 */   114,  226,  227,  125,  118,  108,  205,  192,  253,  194,
 /*  1290 */    90,  133,  134,  135,  136,  137,  138,   97,   98,  240,
 /*  1300 */   205,  242,  144,  243,  192,  105,  194,  107,  108,  109,
 /*  1310 */   110,  111,  248,  113,    0,  251,  252,  205,    4,  258,
 /*  1320 */     6,    5,    8,  192,  124,  125,   90,  261,  128,  248,
 /*  1330 */    16,   15,   18,  252,   20,  300,  205,   11,  243,  304,
 /*  1340 */   305,  306,    1,   14,  108,  258,  110,  111,   19,  256,
 /*  1350 */    24,  151,  152,  153,  192,  243,  194,  295,  196,  192,
 /*  1360 */   267,  125,   21,  232,  233,  192,   40,  205,   54,   92,
 /*  1370 */    92,   57,  205,  242,   45,  125,   62,   63,  205,  283,
 /*  1380 */    66,   55,  286,   69,  134,   44,  115,  151,  152,  118,
 /*  1390 */   113,  113,  115,  115,  192,  118,  194,  192,  196,   58,
 /*  1400 */   202,  203,   88,   62,  192,  243,  194,  205,  196,  192,
 /*  1410 */   205,  209,  210,  211,   11,  248,   90,  205,  251,  252,
 /*  1420 */   202,  203,  205,   97,   98,  192,  112,   24,   97,   98,
 /*  1430 */   257,  105,  270,  107,  108,  109,  110,  111,  205,  113,
 /*  1440 */   202,  203,  264,   40,  113,  243,  192,   11,  194,  192,
 /*  1450 */   124,  125,  126,  248,  128,  243,  251,  252,   55,  205,
 /*  1460 */    24,  147,  205,  209,  210,  211,  202,  203,  154,   34,
 /*  1470 */   156,  192,  270,  159,   40,  161,   40,  151,  152,  153,
 /*  1480 */   166,  192,  270,  194,  205,  192,  197,  194,  202,  203,
 /*  1490 */   197,   55,   14,   90,  205,  113,  192,  243,  205,  117,
 /*  1500 */    97,   98,  192,  246,  194,  288,  304,  305,  105,  205,
 /*  1510 */   107,  108,  109,  110,  111,  205,  113,  192,   24,  194,
 /*  1520 */    24,   27,  197,   45,  270,  192,   90,  124,  125,  126,
 /*  1530 */   205,  128,  243,   97,   98,   99,  243,  192,  205,  192,
 /*  1540 */    48,  105,  108,  107,  108,  109,  110,  111,  113,  113,
 /*  1550 */   205,  117,  205,  243,  151,  152,  153,   11,  304,  305,
 /*  1560 */   124,  125,  264,  192,  128,    4,   74,    6,  243,    8,
 /*  1570 */    24,  192,  148,  149,  106,   83,  205,   16,   40,   18,
 /*  1580 */   270,   20,  115,   87,  205,  118,   40,  151,  152,  153,
 /*  1590 */   192,  192,  192,  194,  261,  295,  197,  192,  192,  194,
 /*  1600 */   106,   55,  197,  205,  205,  205,  261,  192,  116,  194,
 /*  1610 */   205,  205,  192,  113,  194,   54,  312,  149,   57,  119,
 /*  1620 */   205,  317,  318,   62,   63,  205,  269,   66,   72,   73,
 /*  1630 */    69,  109,  192,  269,  194,  288,   90,  115,  144,  192,
 /*  1640 */   118,  194,  243,   97,   98,  205,  108,  268,  243,   88,
 /*  1650 */   264,  105,  205,  107,  108,  109,  110,  111,  243,  113,
 /*  1660 */   219,  261,  215,  243,  262,  117,  268,  119,  192,  192,
 /*  1670 */   124,  125,  228,  112,  128,    4,  221,    6,  192,    8,
 /*  1680 */   194,  205,  205,  243,   40,  270,  164,   16,  236,   18,
 /*  1690 */   243,  205,  244,  192,  192,  118,  192,  151,  152,  153,
 /*  1700 */   192,  192,  115,  192,  127,  118,  205,  205,  147,  205,
 /*  1710 */   270,  110,  111,  205,  205,  154,  205,  156,  312,  192,
 /*  1720 */   159,  194,  161,  317,  318,   54,  192,  166,   57,  243,
 /*  1730 */   284,   40,  205,   62,   63,  284,  192,   66,  194,  205,
 /*  1740 */    69,   97,   98,  188,  189,  190,  191,  192,  193,  205,
 /*  1750 */   106,  107,  108,  109,  110,  111,  270,  284,  192,   88,
 /*  1760 */   194,  192,  259,  194,  209,  210,  211,  114,  115,  268,
 /*  1770 */   243,  205,   10,  192,  205,  194,  268,  284,  134,  277,
 /*  1780 */   192,   61,  194,  262,  259,  141,  205,  243,  277,  192,
 /*  1790 */   235,  194,  237,  205,  192,  244,  194,  270,   78,  108,
 /*  1800 */   217,   81,  205,  192,  192,  194,  244,  205,  253,  243,
 /*  1810 */    48,  277,  243,  192,  195,  194,  205,  205,  147,  146,
 /*  1820 */   192,   59,  194,  303,  243,  154,  205,  156,  192,  192,
 /*  1830 */   159,  243,  161,  205,  192,  192,  194,  166,  118,  208,
 /*  1840 */   243,  205,  205,   13,  192,  243,  194,  205,  205,  192,
 /*  1850 */   192,  194,  194,  301,  243,  300,  192,  205,  194,  304,
 /*  1860 */   305,  306,  205,  205,  243,  301,  192,  119,  194,  205,
 /*  1870 */   192,  243,  194,  192,  192,  194,  192,  208,  194,  205,
 /*  1880 */   192,   63,  194,  205,  192,  243,  205,  205,  208,  205,
 /*  1890 */   192,  192,  194,  205,  142,  243,  192,  205,  110,  111,
 /*  1900 */   243,  243,  150,  205,  205,  143,  192,  243,  115,  205,
 /*  1910 */   192,  118,  194,  192,  277,  194,  192,  243,  194,  205,
 /*  1920 */   192,  243,  194,  205,  243,  115,  205,  243,  118,  205,
 /*  1930 */   192,  243,  194,  205,  192,  192,  194,  194,  192,  160,
 /*  1940 */   194,  243,  192,  205,  194,  246,  274,  205,  205,  192,
 /*  1950 */   115,  205,  192,  118,  194,  205,  192,  192,  194,  194,
 /*  1960 */   275,  243,  205,  192,  243,  205,  192,  243,  194,  205,
 /*  1970 */   205,  243,  275,   40,  192,  192,  205,  194,  192,  205,
 /*  1980 */   194,  243,  192,  274,  194,  243,  243,  205,  205,  243,
 /*  1990 */   274,  205,   41,  243,   91,  205,  192,  192,  194,  276,
 /*  2000 */   192,  192,  194,  243,  272,  276,  272,  243,  243,  205,
 /*  2010 */   205,  192,  272,  205,  205,  192,  272,  243,  192,  192,
 /*  2020 */   194,  194,  149,  192,  205,  194,  243,  115,  205,  243,
 /*  2030 */   118,  205,  205,  243,  277,  278,  205,  192,  265,  194,
 /*  2040 */   192,  108,  194,  192,  192,  194,  194,  243,  113,  163,
 /*  2050 */   205,  243,  192,  205,  194,  263,  205,  205,  260,  192,
 /*  2060 */   114,  194,  192,  192,  194,  205,  192,  192,  194,  243,
 /*  2070 */   243,  192,  205,  194,  243,  205,  205,  192,  115,  205,
 /*  2080 */   205,  118,   22,  192,  205,  192,  192,  259,  243,  192,
 /*  2090 */   205,  243,  255,  192,  243,  243,  205,  115,  205,  205,
 /*  2100 */   118,  115,  205,  243,  118,  192,  205,  192,  192,  260,
 /*  2110 */   243,  192,  192,  243,  192,  192,  192,  243,  205,  192,
 /*  2120 */   205,  205,  243,  192,  205,  205,  192,  205,  205,  205,
 /*  2130 */   192,  192,  205,  208,  115,  115,  205,  118,  118,  205,
 /*  2140 */   259,   91,  115,  205,  205,  118,  115,  115,  115,  118,
 /*  2150 */   118,  118,  148,  239,  113,  208,  269,  239,  269,  269,
 /*  2160 */   265,  265,  263,  255,  255,  208,  245,  269,  239,  129,
 /*  2170 */   293,   43,  292,  208,  208,  239,  239,  245,  106,  229,
 /*  2180 */    46,  229,  229,  224,  113,  245,  139,  229,  234,  234,
 /*  2190 */   219,  229,  224,  229,  118,  208,  229,  241,  162,  116,
 /*  2200 */    80,  260,  259,   71,  260,  313,  113,  281,  280,  212,
 /*  2210 */   208,  269,  104,  313,  129,  216,  271,  117,  249,  271,
 /*  2220 */   250,  245,  250,  168,  231,  323,  204,  204,  311,  204,
 /*  2230 */   204,  231,  220,  220,  218,  218,  231,   49,  204,   50,
 /*  2240 */   113,  311,  112,  157,  115,  158,  115,  157,  132,  124,
 /*  2250 */   147,  146,  143,  117,  165,  118,  132,  308,  113,  106,
 /*  2260 */   127,  155,  112,   42,  143,  127,  143,   12,  146,   34,
 /*  2270 */    34,   34,   34,  107,    9,  119,  112,    8,  143,  117,
 /*  2280 */    52,  118,   52,   60,   17,  106,  119,   24,  144,  124,
 /*  2290 */   138,  113,   51,  113,  113,   51,  113,  115,   40,   85,
 /*  2300 */     2,  117,   51,  113,   12,  118,  115,  115,  115,  107,
 /*  2310 */     9,  113,  164,  118,    9,  115,  113,  118,  117,    9,
 /*  2320 */   115,  114,  113,  119,  116,  148,  113,  115,    9,    9,
 /*  2330 */     9,   77,   60,   60,   75,   23,  118,    9,   82,  115,
 /*  2340 */   115,  115,  113,  127,   87,  118,  113,  127,   18,  118,
 /*  2350 */   118,    9,  118,  113,  115,    9,  324,  324,  113,  113,
 /*  2360 */   324,  119,  119,  115,  115,  113,  324,  324,  121,  119,
 /*  2370 */   324,  324,  119,  324,  324,  115,  115,  115,  324,  113,
 /*  2380 */   112,  324,  112,  324,  324,  324,  324,  324,  324,  324,
 /*  2390 */   324,  324,  324,  324,  324,  324,  324,  324,  324,  324,
 /*  2400 */   324,  324,  324,  324,  324,  324,  324,  324,  324,  324,
 /*  2410 */   324,  324,  324,  324,  324,  324,  324,  324,  324,  324,
 /*  2420 */   324,  324,  324,  324,  324,  324,  324,  324,  324,  324,
 /*  2430 */   324,  324,  324,  324,  324,  324,  324,  324,  324,  324,
 /*  2440 */   324,  324,  324,  324,  324,  324,  324,  324,  324,  324,
 /*  2450 */   324,  324,  324,  324,  324,  324,  324,  324,  324,  324,
 /*  2460 */   324,  324,  324,  324,  324,  324,  324,  324,  324,  324,
 /*  2470 */   324,  324,  324,  324,  324,  324,  324,  324,  324,  324,
 /*  2480 */   324,  324,  324,  324,  324,  324,  324,  324,  324,  324,
 /*  2490 */   324,  324,  324,  324,  324,  324,  324,  324,  324,  324,
 /*  2500 */   324,  324,  324,  324,  324,  324,  324,  324,  324,  324,
 /*  2510 */   324,  324,  324,  324,  324,  324,  324,  324,  324,  324,
 /*  2520 */   324,  324,  324,  324,  324,  324,  324,  324,  324,  324,
 /*  2530 */   324,  324,  324,  324,  324,  324,  324,  324,  324,  324,
 /*  2540 */   324,  324,  324,  324,  324,  324,  324,  324,  324,  324,
 /*  2550 */   324,  324,  324,  324,  324,  324,  324,  324,  324,  324,
 /*  2560 */   324,  324,  324,  324,  324,  324,  324,  324,  324,  324,
 /*  2570 */   324,
};
#define YY_SHIFT_COUNT    (595)
#define YY_SHIFT_MIN      (0)
#define YY_SHIFT_MAX      (2346)
static const unsigned short int yy_shift_ofst[] = {
 /*     0 */  1561, 1314,  991, 1671,  991,  458, 1050, 1125, 1200, 1546,
 /*    10 */  1546, 1546,  945, 1546, 1546, 1546, 1546, 1546, 1546, 1546,
 /*    20 */  1546, 1546, 1546, 1546, 1546, 1546, 1546, 1546, 1644, 1644,
 /*    30 */   249, 1105, 1105,  422,  445,  960,  272,  272,  458,  458,
 /*    40 */   458,  458,  458,    0,    0,  142,  840, 1326, 1403, 1436,
 /*    50 */  1546, 1546, 1546, 1546, 1546, 1546, 1546, 1546, 1546, 1546,
 /*    60 */  1546, 1546, 1546, 1546, 1546, 1546, 1546, 1546, 1546, 1546,
 /*    70 */  1546, 1546, 1546, 1546, 1546, 1546, 1546, 1546, 1546, 1546,
 /*    80 */  1546, 1546, 1546, 1546, 1546, 1546, 1546, 1546, 1546, 1546,
 /*    90 */  1546, 1546, 1546, 1546, 1546, 1546, 1546, 1546, 1546, 1546,
 /*   100 */  1546,  395,  272,  272,  272,  272,  275,  275,  275,  275,
 /*   110 */    23,  120,  225,  250,  273,  373,  396,  419,  513,  536,
 /*   120 */   559,  653,  676,  699,  793,  815,  840,  840,  840,  840,
 /*   130 */   840,  840,  840,  840,  840,  840,  840,  840,  840,  840,
 /*   140 */   840,  840,  840,  840,  924,  840,  940,  955,  955,   35,
 /*   150 */   969,  969,  969,  969,  969,  969,  969,  105,  128,  165,
 /*   160 */   968,  632,  632,  632,  632,  395,  272,  272,  272,  272,
 /*   170 */   272,  272,  272,  272,  272,  272,  272,  272,  272,  272,
 /*   180 */   272,  272,  858, 1494,  272,  272,  272,  272, 1074, 1074,
 /*   190 */   476,  377,   21,  295,  295,  295,  378,    8,    8, 2383,
 /*   200 */  2383, 1158, 1158, 1158,  652, 1341, 1341,  678,  253,  655,
 /*   210 */   231, 1129,  748,  772, 1078, 1277,  272,  272,  272,  272,
 /*   220 */   272,  272,  272,  272,  272,  272,  272,  272,  272,  272,
 /*   230 */   272,  272,  272,  272,  272,  272,  272,  272,  272,  272,
 /*   240 */   272,  272,  272,  272,  272,  272,  272,  272,  227,  272,
 /*   250 */  1720, 1720,  272,  272,  272,  272,  763, 1056, 1056,   87,
 /*   260 */    87, 1177, 1053, 1177, 2383, 2383, 2383, 2383, 2383, 2383,
 /*   270 */  2383, 1043, 1236, 1236,  975,  175, 1762, 1762, 1762, 1522,
 /*   280 */  1005, 1492,  269, 1329,  438,  620, 1331, 1434, 1434, 1434,
 /*   290 */   223, 1434,  685,  685,  799,  685,  516, 1468, 1424, 1141,
 /*   300 */  1424,  252,  736, 1468, 1468,  736, 1468, 1141,  252,  196,
 /*   310 */  1271,  839, 1316, 1382,  619,  619,  619,  619, 1577, 1166,
 /*   320 */  1166, 1316, 1316,   53, 1140, 1673, 1830, 1830, 1748, 1748,
 /*   330 */  1748, 1818, 1818, 1779, 1779, 1779, 1951, 1951, 1903, 1903,
 /*   340 */  1903, 1903, 1873, 1935, 1886, 1946, 2060, 1886, 1946, 1748,
 /*   350 */  2050, 2004, 1748, 2050, 2004, 1873, 1873, 2004, 1935, 2060,
 /*   360 */  2004, 2060, 2041, 1748, 2050, 2040, 2128, 1748, 2050, 1748,
 /*   370 */  2050, 2041, 2072, 2072, 2072, 2134, 2071, 2071, 2041, 2072,
 /*   380 */  2047, 2072, 2134, 2072, 2072, 2076, 1748, 1886, 1946, 1886,
 /*   390 */  2036, 2083, 2004, 2120, 2120, 2132, 2132, 2093, 1748, 2108,
 /*   400 */  2108, 2085, 2100, 2041, 2055, 2383, 2383, 2383, 2383, 2383,
 /*   410 */  2383, 2383, 2383, 2383, 2383, 2383, 2383, 2383, 2383, 2383,
 /*   420 */    61,  677,  814, 1278,  768,  718,  794, 1467, 1435,  514,
 /*   430 */  1548, 1587, 1089, 1752, 1793,  974, 1810, 1835, 1912, 1963,
 /*   440 */  1982, 1538, 1250, 1478,  856, 1986, 1556, 1691, 2019, 1496,
 /*   450 */  2020, 2027, 2031, 1933, 1601, 1788, 2032, 2033, 1653, 1500,
 /*   460 */  2188, 2189, 2127, 2130, 2129, 2131, 2086, 2087, 2090, 2116,
 /*   470 */  2125, 2103, 2105, 2109, 2136, 2137, 2137, 2133, 2089, 2124,
 /*   480 */  2145, 2153, 2106, 2150, 2138, 2121, 2137, 2123, 2221, 2255,
 /*   490 */  2137, 2122, 2235, 2236, 2237, 2238, 2135, 2166, 2265, 2156,
 /*   500 */  2164, 2269, 2162, 2228, 2163, 2230, 2223, 2267, 2167, 2179,
 /*   510 */  2165, 2263, 2144, 2152, 2178, 2241, 2180, 2181, 2182, 2183,
 /*   520 */  2244, 2258, 2184, 2214, 2298, 2190, 2251, 2292, 2187, 2191,
 /*   530 */  2192, 2193, 2202, 2301, 2198, 2148, 2195, 2305, 2200, 2203,
 /*   540 */  2201, 2199, 2204, 2205, 2310, 2207, 2209, 2208, 2177, 2213,
 /*   550 */  2212, 2319, 2320, 2321, 2254, 2272, 2259, 2312, 2273, 2256,
 /*   560 */  2218, 2328, 2224, 2195, 2225, 2226, 2227, 2257, 2229, 2231,
 /*   570 */  2233, 2232, 2216, 2220, 2234, 2239, 2240, 2242, 2243, 2330,
 /*   580 */  2245, 2248, 2246, 2247, 2249, 2252, 2260, 2250, 2253, 2261,
 /*   590 */  2262, 2266, 2342, 2346, 2268, 2270,
};
#define YY_REDUCE_COUNT (419)
#define YY_REDUCE_MIN   (-312)
#define YY_REDUCE_MAX   (2034)
static const short yy_reduce_ofst[] = {
 /*     0 */  1555, 1035, 1202,  411, 1254, -199, -130,  888,  804,  936,
 /*    10 */  1162, 1212, -144,   -9, 1024, 1060, 1289,  892, 1293, 1325,
 /*    20 */  1310, 1399, 1415, 1440, 1447, 1486, 1527, 1405, -189,  186,
 /*    30 */  1131, 1304, 1406, -166, 1064, -190, 1167, 1205,  154,  203,
 /*    40 */   228,  366,  389, -200,  597,  436,  717, 1095, 1112, 1420,
 /*    50 */  1544, 1566, 1569, 1581, 1588, 1597, 1602, 1611, 1621, 1628,
 /*    60 */  1642, 1652, 1657, 1658, 1664, 1674, 1678, 1681, 1684, 1688,
 /*    70 */  1698, 1718, 1721, 1724, 1728, 1738, 1742, 1743, 1746, 1750,
 /*    80 */  1760, 1764, 1765, 1774, 1783, 1786, 1790, 1804, 1808, 1826,
 /*    90 */  1827, 1831, 1845, 1848, 1851, 1852, 1860, 1867, 1870, 1874,
 /*   100 */  1879, 1055, 1757, 1059,  -24, 1081,  251,  415,  251,  415,
 /*   110 */  -205, -205, -205, -205, -205, -205, -205, -205, -205, -205,
 /*   120 */  -205, -205, -205, -205, -205, -205, -205, -205, -205, -205,
 /*   130 */  -205, -205, -205, -205, -205, -205, -205, -205, -205, -205,
 /*   140 */  -205, -205, -205, -205, -205, -205, -205, -205, -205, -205,
 /*   150 */  -205, -205, -205, -205, -205, -205, -205, -205, -205, -205,
 /*   160 */  -113, 1066, 1333, 1345, 1400,  805,  934,  964, 1502, 1511,
 /*   170 */  1534, -188,  -46, 1173,  807,  995, 1217, 1379, 1398, 1347,
 /*   180 */  1501, 1637,  380,  303, 1257, 1508, -124, 1699,   93,  400,
 /*   190 */    14, -205,  609,  444,  467,  560,  205, -205, -205, -205,
 /*   200 */  -205, -181, -181, -181,  886,  239,  240,  972, 1233, 1279,
 /*   210 */  1371, -312, 1476, 1477, -269, -269, 1504, 1509, 1612, 1636,
 /*   220 */  1643, 1682, 1692, 1704, 1714, 1771, 1782, 1805, 1809, 1819,
 /*   230 */  1823, 1871, 1875, 1885, 1891, 1893, 1894, 1897, 1901, 1913,
 /*   240 */  1915, 1916, 1919, 1920, 1922, 1923, 1924, 1927, -266, 1931,
 /*   250 */  -235,  329, 1934, 1938, 1939, 1371,  151,  683,  791,  981,
 /*   260 */  1093,  583,  -72,  989,  890, 1198, 1218, 1238, 1264, 1096,
 /*   270 */  1286, -180, -165, -163,  -92, -211,   77,  161,  194,  155,
 /*   280 */   306,  312,  477,  393,  545,  614,  484,  649,  671,  687,
 /*   290 */   155,  732,  824,  824,  932,  824,  870,  909, 1061,  930,
 /*   300 */  1087, 1062, 1357, 1178, 1298, 1364, 1386, 1402, 1300, 1441,
 /*   310 */  1444, 1455, 1448, 1452, 1446, 1451, 1473, 1493, 1521, 1503,
 /*   320 */  1525, 1551, 1562, 1583, 1619, 1520, 1552, 1564, 1631, 1669,
 /*   330 */  1680, 1685, 1697, 1672, 1709, 1716, 1723, 1729, 1732, 1734,
 /*   340 */  1740, 1744, 1773, 1792, 1798, 1828, 1837, 1849, 1881, 1925,
 /*   350 */  1914, 1887, 1947, 1918, 1889, 1895, 1896, 1890, 1899, 1908,
 /*   360 */  1898, 1909, 1921, 1957, 1929, 1877, 1880, 1965, 1936, 1966,
 /*   370 */  1937, 1932, 1950, 1952, 1953, 1959, 1954, 1955, 1940, 1958,
 /*   380 */  1971, 1962, 1968, 1964, 1967, 1956, 1987, 1941, 1943, 1944,
 /*   390 */  1926, 1928, 1942, 1892, 1900, 1945, 1948, 1997, 2002, 1970,
 /*   400 */  1972, 1999, 1969, 1976, 1902, 1917, 1930, 1949, 1993, 2022,
 /*   410 */  2023, 2025, 2026, 2000, 2012, 2013, 2016, 2017, 2005, 2034,
};
static const YYACTIONTYPE yy_default[] = {
 /*     0 */  1415, 1415, 1469, 1415, 1275, 1562, 1275, 1275, 1275, 1469,
 /*    10 */  1469, 1469, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275,
 /*    20 */  1275, 1275, 1275, 1275, 1334, 1275, 1275, 1275, 1275, 1275,
 /*    30 */  1275, 1670, 1670, 1275, 1275, 1275, 1275, 1275, 1275, 1275,
 /*    40 */  1275, 1275, 1275, 1491, 1491, 1625, 1540, 1275, 1275, 1275,
 /*    50 */  1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275,
 /*    60 */  1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275,
 /*    70 */  1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275,
 /*    80 */  1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275,
 /*    90 */  1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275,
 /*   100 */  1275, 1339, 1275, 1275, 1275, 1275, 1417, 1416, 1275, 1275,
 /*   110 */  1558, 1275, 1275, 1436, 1275, 1275, 1275, 1275, 1275, 1275,
 /*   120 */  1470, 1471, 1275, 1275, 1275, 1275, 1629, 1622, 1626, 1442,
 /*   130 */  1441, 1440, 1439, 1590, 1572, 1550, 1554, 1560, 1559, 1470,
 /*   140 */  1330, 1331, 1329, 1333, 1275, 1471, 1461, 1467, 1460, 1326,
 /*   150 */  1320, 1319, 1318, 1459, 1327, 1323, 1317, 1458, 1462, 1456,
 /*   160 */  1630, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275,
 /*   170 */  1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275,
 /*   180 */  1275, 1275, 1642, 1395, 1275, 1275, 1275, 1275, 1275, 1275,
 /*   190 */  1473, 1457, 1540, 1474, 1287, 1285, 1275, 1464, 1463, 1466,
 /*   200 */  1465, 1511, 1345, 1344, 1275, 1275, 1275, 1275, 1275, 1275,
 /*   210 */  1275, 1670, 1275, 1275, 1275, 1275, 1619, 1617, 1275, 1275,
 /*   220 */  1275, 1275, 1524, 1275, 1275, 1275, 1275, 1275, 1275, 1275,
 /*   230 */  1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275,
 /*   240 */  1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1574, 1275,
 /*   250 */  1670, 1670, 1275, 1275, 1275, 1275, 1540, 1670, 1670, 1433,
 /*   260 */  1433, 1290, 1555, 1290, 1653, 1539, 1539, 1539, 1539, 1562,
 /*   270 */  1539, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275,
 /*   280 */  1543, 1275, 1275, 1519, 1275, 1583, 1399, 1543, 1543, 1543,
 /*   290 */  1548, 1543, 1401, 1400, 1546, 1532, 1513, 1445, 1435, 1547,
 /*   300 */  1435, 1595, 1549, 1445, 1445, 1549, 1445, 1547, 1595, 1366,
 /*   310 */  1389, 1359, 1491, 1275, 1574, 1574, 1574, 1574, 1547, 1555,
 /*   320 */  1555, 1491, 1491, 1332, 1546, 1630, 1624, 1624, 1311, 1311,
 /*   330 */  1311, 1527, 1527, 1523, 1523, 1523, 1513, 1513, 1503, 1503,
 /*   340 */  1503, 1503, 1452, 1443, 1557, 1555, 1424, 1557, 1555, 1311,
 /*   350 */  1637, 1549, 1311, 1637, 1549, 1452, 1452, 1549, 1443, 1424,
 /*   360 */  1549, 1424, 1409, 1311, 1637, 1589, 1587, 1311, 1637, 1311,
 /*   370 */  1637, 1409, 1397, 1397, 1397, 1381, 1275, 1275, 1409, 1397,
 /*   380 */  1366, 1397, 1381, 1397, 1397, 1384, 1311, 1557, 1555, 1557,
 /*   390 */  1553, 1551, 1549, 1680, 1680, 1494, 1494, 1313, 1311, 1413,
 /*   400 */  1413, 1275, 1275, 1409, 1688, 1658, 1658, 1653, 1347, 1540,
 /*   410 */  1540, 1540, 1540, 1347, 1368, 1368, 1399, 1399, 1347, 1540,
 /*   420 */  1275, 1275, 1275, 1275, 1275, 1275, 1291, 1275, 1602, 1512,
 /*   430 */  1429, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275,
 /*   440 */  1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1406,
 /*   450 */  1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1297,
 /*   460 */  1275, 1632, 1648, 1275, 1275, 1275, 1518, 1275, 1275, 1275,
 /*   470 */  1275, 1275, 1275, 1275, 1430, 1437, 1438, 1275, 1275, 1275,
 /*   480 */  1275, 1275, 1275, 1275, 1275, 1275, 1451, 1275, 1275, 1275,
 /*   490 */  1446, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1593,
 /*   500 */  1275, 1275, 1275, 1275, 1586, 1585, 1275, 1275, 1500, 1275,
 /*   510 */  1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275,
 /*   520 */  1275, 1364, 1275, 1275, 1275, 1275, 1275, 1275, 1337, 1275,
 /*   530 */  1275, 1275, 1275, 1275, 1275, 1275, 1552, 1275, 1275, 1275,
 /*   540 */  1275, 1685, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275,
 /*   550 */  1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275,
 /*   560 */  1556, 1275, 1275, 1468, 1275, 1275, 1275, 1275, 1275, 1647,
 /*   570 */  1275, 1646, 1275, 1275, 1275, 1275, 1275, 1481, 1453, 1275,
 /*   580 */  1275, 1275, 1275, 1301, 1275, 1275, 1275, 1298, 1275, 1275,
 /*   590 */  1275, 1275, 1275, 1275, 1275, 1275,
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
   40,  /*      ABORT => ID */
   40,  /*     ACTION => ID */
   40,  /*      AFTER => ID */
   40,  /*    ANALYZE => ID */
   40,  /*        ASC => ID */
   40,  /*     ATTACH => ID */
   40,  /*     BEFORE => ID */
   40,  /*      BEGIN => ID */
   40,  /*         BY => ID */
   40,  /*    CASCADE => ID */
   40,  /*       CAST => ID */
   40,  /*   CONFLICT => ID */
   40,  /*   DATABASE => ID */
   40,  /*   DEFERRED => ID */
   40,  /*       DESC => ID */
   40,  /*     DETACH => ID */
   40,  /*       EACH => ID */
   40,  /*        END => ID */
   40,  /*  EXCLUSIVE => ID */
   40,  /*    EXPLAIN => ID */
   40,  /*       FAIL => ID */
    0,  /*         OR => nothing */
    0,  /*        AND => nothing */
    0,  /*        NOT => nothing */
    0,  /*         IS => nothing */
    0,  /*      ISNOT => nothing */
   40,  /*      MATCH => ID */
   40,  /*    LIKE_KW => ID */
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
   40,  /*   COLUMNKW => ID */
   40,  /*         DO => ID */
   40,  /*        FOR => ID */
   40,  /*     IGNORE => ID */
   40,  /*  IMMEDIATE => ID */
   40,  /*  INITIALLY => ID */
   40,  /*    INSTEAD => ID */
   40,  /*         NO => ID */
   40,  /*       PLAN => ID */
   40,  /*      QUERY => ID */
   40,  /*        KEY => ID */
   40,  /*         OF => ID */
   40,  /*     OFFSET => ID */
   40,  /*     PRAGMA => ID */
   40,  /*      RAISE => ID */
   40,  /*  RECURSIVE => ID */
   40,  /*    RELEASE => ID */
   40,  /*    REPLACE => ID */
   40,  /*   RESTRICT => ID */
   40,  /*        ROW => ID */
   40,  /*       ROWS => ID */
   40,  /*   ROLLBACK => ID */
   40,  /*  SAVEPOINT => ID */
   40,  /*       TEMP => ID */
   40,  /*    TRIGGER => ID */
   40,  /*     VACUUM => ID */
   40,  /*       VIEW => ID */
   40,  /*    VIRTUAL => ID */
   40,  /*       WITH => ID */
   40,  /*    WITHOUT => ID */
   40,  /*      NULLS => ID */
   40,  /*      FIRST => ID */
   40,  /*       LAST => ID */
   40,  /*    CURRENT => ID */
   40,  /*  FOLLOWING => ID */
   40,  /*  PARTITION => ID */
   40,  /*  PRECEDING => ID */
   40,  /*      RANGE => ID */
   40,  /*  UNBOUNDED => ID */
   40,  /*    EXCLUDE => ID */
   40,  /*     GROUPS => ID */
   40,  /*     OTHERS => ID */
   40,  /*       TIES => ID */
   40,  /*  GENERATED => ID */
   40,  /*     ALWAYS => ID */
   40,  /*     WITHIN => ID */
   40,  /* MATERIALIZED => ID */
   40,  /*    REINDEX => ID */
   40,  /*     RENAME => ID */
   40,  /*   CTIME_KW => ID */
   40,  /*         IF => ID */
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
  YYACTIONTYPE stateno;  /* The state-number, or reduce action in SHIFTREDUCE */
  YYCODETYPE major;      /* The major token value.  This is the code
                         ** number for the token at this stack level */
  YYMINORTYPE minor;     /* The user-supplied minor token value.  This
                         ** is the value of the token  */
};
typedef struct yyStackEntry yyStackEntry;

/* The state of the parser is completely contained in an instance of
** the following structure */
struct yyParser {
  yyStackEntry *yytos;          /* Pointer to top element of the stack */
#ifdef YYTRACKMAXSTACKDEPTH
  int yyhwm;                    /* High-water mark of the stack */
#endif
#ifndef YYNOERRORRECOVERY
  int yyerrcnt;                 /* Shifts left before out of the error */
#endif
  SynqSqliteParseARG_SDECL                /* A place to hold %extra_argument */
  SynqSqliteParseCTX_SDECL                /* A place to hold %extra_context */
  yyStackEntry *yystackEnd;           /* Last entry in the stack */
  yyStackEntry *yystack;              /* The parser stack */
  yyStackEntry yystk0[YYSTACKDEPTH];  /* Initial stack space */
};
typedef struct yyParser yyParser;

#include <assert.h>
#ifndef NDEBUG
#include <stdio.h>

#include "syntaqlite_sqlite/sqlite_tokens.h"

#include "syntaqlite_dialect/dialect_macros.h"
static FILE *yyTraceFILE = 0;
static char *yyTracePrompt = 0;
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
void SynqSqliteParseTrace(FILE *TraceFILE, char *zTracePrompt){
  yyTraceFILE = TraceFILE;
  yyTracePrompt = zTracePrompt;
  if( yyTraceFILE==0 ) yyTracePrompt = 0;
  else if( yyTracePrompt==0 ) yyTraceFILE = 0;
}
#endif /* NDEBUG */

#if defined(YYCOVERAGE) || !defined(NDEBUG)
/* For tracing shifts, the names of all terminals and nonterminals
** are required.  The following table supplies these names */
static const char *const yyTokenName[] = { 
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
static const char *const yyRuleName[] = {
 /*   0 */ "input ::= cmdlist",
 /*   1 */ "cmdlist ::= cmdlist ecmd",
 /*   2 */ "cmdlist ::= ecmd",
 /*   3 */ "ecmd ::= SEMI",
 /*   4 */ "ecmd ::= cmdx SEMI",
 /*   5 */ "ecmd ::= error SEMI",
 /*   6 */ "cmdx ::= cmd",
 /*   7 */ "expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist ORDER BY sortlist RP",
 /*   8 */ "expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist ORDER BY sortlist RP filter_over",
 /*   9 */ "expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist RP WITHIN GROUP LP ORDER BY expr RP",
 /*  10 */ "expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist RP WITHIN GROUP LP ORDER BY expr RP filter_over",
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
 /*  59 */ "create_table_args ::= LP columnlist conslist_opt RP table_option_set",
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
 /* 116 */ "tcons ::= FOREIGN KEY LP eidlist RP REFERENCES nm eidlist_opt refargs defer_subclause_opt",
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
 /* 140 */ "cmd ::= with DELETE FROM xfullname indexed_opt where_opt_ret orderby_opt limit_opt",
 /* 141 */ "cmd ::= with UPDATE orconf xfullname indexed_opt SET setlist from where_opt_ret orderby_opt limit_opt",
 /* 142 */ "cmd ::= with insert_cmd INTO xfullname idlist_opt select upsert",
 /* 143 */ "cmd ::= with insert_cmd INTO xfullname idlist_opt DEFAULT VALUES returning",
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
 /* 169 */ "upsert ::= ON CONFLICT LP sortlist RP where_opt DO UPDATE SET setlist where_opt upsert",
 /* 170 */ "upsert ::= ON CONFLICT LP sortlist RP where_opt DO NOTHING upsert",
 /* 171 */ "upsert ::= ON CONFLICT DO NOTHING returning",
 /* 172 */ "upsert ::= ON CONFLICT DO UPDATE SET setlist where_opt returning",
 /* 173 */ "returning ::= RETURNING selcollist",
 /* 174 */ "returning ::=",
 /* 175 */ "expr ::= error",
 /* 176 */ "expr ::= term",
 /* 177 */ "expr ::= LP expr RP",
 /* 178 */ "expr ::= expr PLUS|MINUS expr",
 /* 179 */ "expr ::= expr STAR|SLASH|REM expr",
 /* 180 */ "expr ::= expr LT|GT|GE|LE expr",
 /* 181 */ "expr ::= expr EQ|NE expr",
 /* 182 */ "expr ::= expr AND expr",
 /* 183 */ "expr ::= expr OR expr",
 /* 184 */ "expr ::= expr BITAND|BITOR|LSHIFT|RSHIFT expr",
 /* 185 */ "expr ::= expr CONCAT expr",
 /* 186 */ "expr ::= expr PTR expr",
 /* 187 */ "expr ::= PLUS|MINUS expr",
 /* 188 */ "expr ::= BITNOT expr",
 /* 189 */ "expr ::= NOT expr",
 /* 190 */ "exprlist ::= nexprlist",
 /* 191 */ "exprlist ::=",
 /* 192 */ "nexprlist ::= nexprlist COMMA expr",
 /* 193 */ "nexprlist ::= expr",
 /* 194 */ "expr ::= LP nexprlist COMMA expr RP",
 /* 195 */ "expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist RP",
 /* 196 */ "expr ::= ID|INDEXED|JOIN_KW LP STAR RP",
 /* 197 */ "expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist RP filter_over",
 /* 198 */ "expr ::= ID|INDEXED|JOIN_KW LP STAR RP filter_over",
 /* 199 */ "nm ::= error",
 /* 200 */ "nm ::= ID|INDEXED|JOIN_KW",
 /* 201 */ "nm ::= STRING",
 /* 202 */ "term ::= INTEGER",
 /* 203 */ "term ::= STRING",
 /* 204 */ "term ::= NULL|FLOAT|BLOB",
 /* 205 */ "term ::= QNUMBER",
 /* 206 */ "term ::= CTIME_KW",
 /* 207 */ "expr ::= VARIABLE",
 /* 208 */ "expr ::= expr COLLATE ID|STRING",
 /* 209 */ "sortlist ::= sortlist COMMA expr sortorder nulls",
 /* 210 */ "sortlist ::= expr sortorder nulls",
 /* 211 */ "sortorder ::= ASC",
 /* 212 */ "sortorder ::= DESC",
 /* 213 */ "sortorder ::=",
 /* 214 */ "nulls ::= NULLS FIRST",
 /* 215 */ "nulls ::= NULLS LAST",
 /* 216 */ "nulls ::=",
 /* 217 */ "expr ::= RAISE LP IGNORE RP",
 /* 218 */ "expr ::= RAISE LP raisetype COMMA expr RP",
 /* 219 */ "raisetype ::= ROLLBACK",
 /* 220 */ "raisetype ::= ABORT",
 /* 221 */ "raisetype ::= FAIL",
 /* 222 */ "fullname ::= nm",
 /* 223 */ "fullname ::= nm DOT nm",
 /* 224 */ "ifexists ::= IF EXISTS",
 /* 225 */ "ifexists ::=",
 /* 226 */ "cmd ::= DROP TABLE ifexists fullname",
 /* 227 */ "cmd ::= DROP VIEW ifexists fullname",
 /* 228 */ "cmd ::= DROP INDEX ifexists fullname",
 /* 229 */ "cmd ::= DROP TRIGGER ifexists fullname",
 /* 230 */ "cmd ::= ALTER TABLE fullname RENAME TO nm",
 /* 231 */ "cmd ::= ALTER TABLE fullname RENAME kwcolumn_opt nm TO nm",
 /* 232 */ "cmd ::= ALTER TABLE fullname DROP kwcolumn_opt nm",
 /* 233 */ "cmd ::= ALTER TABLE add_column_fullname ADD kwcolumn_opt columnname carglist",
 /* 234 */ "add_column_fullname ::= fullname",
 /* 235 */ "kwcolumn_opt ::=",
 /* 236 */ "kwcolumn_opt ::= COLUMNKW",
 /* 237 */ "columnname ::= nm typetoken",
 /* 238 */ "cmd ::= BEGIN transtype trans_opt",
 /* 239 */ "cmd ::= COMMIT|END trans_opt",
 /* 240 */ "cmd ::= ROLLBACK trans_opt",
 /* 241 */ "transtype ::=",
 /* 242 */ "transtype ::= DEFERRED",
 /* 243 */ "transtype ::= IMMEDIATE",
 /* 244 */ "transtype ::= EXCLUSIVE",
 /* 245 */ "trans_opt ::=",
 /* 246 */ "trans_opt ::= TRANSACTION",
 /* 247 */ "trans_opt ::= TRANSACTION nm",
 /* 248 */ "savepoint_opt ::= SAVEPOINT",
 /* 249 */ "savepoint_opt ::=",
 /* 250 */ "cmd ::= SAVEPOINT nm",
 /* 251 */ "cmd ::= RELEASE savepoint_opt nm",
 /* 252 */ "cmd ::= ROLLBACK trans_opt TO savepoint_opt nm",
 /* 253 */ "cmd ::= select",
 /* 254 */ "select ::= selectnowith",
 /* 255 */ "selectnowith ::= oneselect",
 /* 256 */ "oneselect ::= SELECT distinct selcollist from where_opt groupby_opt having_opt orderby_opt limit_opt",
 /* 257 */ "oneselect ::= SELECT distinct selcollist from where_opt groupby_opt having_opt window_clause orderby_opt limit_opt",
 /* 258 */ "selcollist ::= sclp scanpt expr scanpt as",
 /* 259 */ "selcollist ::= sclp scanpt STAR",
 /* 260 */ "sclp ::= selcollist COMMA",
 /* 261 */ "sclp ::=",
 /* 262 */ "scanpt ::=",
 /* 263 */ "as ::= AS nm",
 /* 264 */ "as ::= ID|STRING",
 /* 265 */ "as ::=",
 /* 266 */ "distinct ::= DISTINCT",
 /* 267 */ "distinct ::= ALL",
 /* 268 */ "distinct ::=",
 /* 269 */ "from ::=",
 /* 270 */ "from ::= FROM seltablist",
 /* 271 */ "where_opt ::=",
 /* 272 */ "where_opt ::= WHERE expr",
 /* 273 */ "groupby_opt ::=",
 /* 274 */ "groupby_opt ::= GROUP BY nexprlist",
 /* 275 */ "having_opt ::=",
 /* 276 */ "having_opt ::= HAVING expr",
 /* 277 */ "orderby_opt ::=",
 /* 278 */ "orderby_opt ::= ORDER BY sortlist",
 /* 279 */ "limit_opt ::=",
 /* 280 */ "limit_opt ::= LIMIT expr",
 /* 281 */ "limit_opt ::= LIMIT expr OFFSET expr",
 /* 282 */ "limit_opt ::= LIMIT expr COMMA expr",
 /* 283 */ "stl_prefix ::= seltablist joinop",
 /* 284 */ "stl_prefix ::=",
 /* 285 */ "seltablist ::= stl_prefix nm dbnm as on_using",
 /* 286 */ "seltablist ::= stl_prefix nm dbnm as indexed_by on_using",
 /* 287 */ "seltablist ::= stl_prefix nm dbnm LP exprlist RP as on_using",
 /* 288 */ "seltablist ::= stl_prefix LP select RP as on_using",
 /* 289 */ "seltablist ::= stl_prefix LP seltablist RP as on_using",
 /* 290 */ "joinop ::= COMMA|JOIN",
 /* 291 */ "joinop ::= JOIN_KW JOIN",
 /* 292 */ "joinop ::= JOIN_KW nm JOIN",
 /* 293 */ "joinop ::= JOIN_KW nm nm JOIN",
 /* 294 */ "on_using ::= ON expr",
 /* 295 */ "on_using ::= USING LP idlist RP",
 /* 296 */ "on_using ::=",
 /* 297 */ "indexed_by ::= INDEXED BY nm",
 /* 298 */ "indexed_by ::= NOT INDEXED",
 /* 299 */ "idlist ::= idlist COMMA nm",
 /* 300 */ "idlist ::= nm",
 /* 301 */ "cmd ::= createkw trigger_decl BEGIN trigger_cmd_list END",
 /* 302 */ "trigger_decl ::= temp TRIGGER ifnotexists nm dbnm trigger_time trigger_event ON fullname foreach_clause when_clause",
 /* 303 */ "trigger_time ::= BEFORE|AFTER",
 /* 304 */ "trigger_time ::= INSTEAD OF",
 /* 305 */ "trigger_time ::=",
 /* 306 */ "trigger_event ::= DELETE|INSERT",
 /* 307 */ "trigger_event ::= UPDATE",
 /* 308 */ "trigger_event ::= UPDATE OF idlist",
 /* 309 */ "foreach_clause ::=",
 /* 310 */ "foreach_clause ::= FOR EACH ROW",
 /* 311 */ "when_clause ::=",
 /* 312 */ "when_clause ::= WHEN expr",
 /* 313 */ "trigger_cmd_list ::= trigger_cmd_list trigger_cmd SEMI",
 /* 314 */ "trigger_cmd_list ::= trigger_cmd SEMI",
 /* 315 */ "trnm ::= nm",
 /* 316 */ "trnm ::= nm DOT nm",
 /* 317 */ "tridxby ::=",
 /* 318 */ "tridxby ::= INDEXED BY nm",
 /* 319 */ "tridxby ::= NOT INDEXED",
 /* 320 */ "trigger_cmd ::= UPDATE orconf trnm tridxby SET setlist from where_opt scanpt",
 /* 321 */ "trigger_cmd ::= scanpt insert_cmd INTO trnm idlist_opt select upsert scanpt",
 /* 322 */ "trigger_cmd ::= DELETE FROM trnm tridxby where_opt scanpt",
 /* 323 */ "trigger_cmd ::= scanpt select scanpt",
 /* 324 */ "cmd ::= PRAGMA nm dbnm",
 /* 325 */ "cmd ::= PRAGMA nm dbnm EQ nmnum",
 /* 326 */ "cmd ::= PRAGMA nm dbnm LP nmnum RP",
 /* 327 */ "cmd ::= PRAGMA nm dbnm EQ minus_num",
 /* 328 */ "cmd ::= PRAGMA nm dbnm LP minus_num RP",
 /* 329 */ "nmnum ::= plus_num",
 /* 330 */ "nmnum ::= nm",
 /* 331 */ "nmnum ::= ON",
 /* 332 */ "nmnum ::= DELETE",
 /* 333 */ "nmnum ::= DEFAULT",
 /* 334 */ "plus_num ::= PLUS INTEGER|FLOAT",
 /* 335 */ "plus_num ::= INTEGER|FLOAT",
 /* 336 */ "minus_num ::= MINUS INTEGER|FLOAT",
 /* 337 */ "signed ::= plus_num",
 /* 338 */ "signed ::= minus_num",
 /* 339 */ "cmd ::= ANALYZE",
 /* 340 */ "cmd ::= ANALYZE nm dbnm",
 /* 341 */ "cmd ::= REINDEX",
 /* 342 */ "cmd ::= REINDEX nm dbnm",
 /* 343 */ "cmd ::= ATTACH database_kw_opt expr AS expr key_opt",
 /* 344 */ "cmd ::= DETACH database_kw_opt expr",
 /* 345 */ "database_kw_opt ::= DATABASE",
 /* 346 */ "database_kw_opt ::=",
 /* 347 */ "key_opt ::=",
 /* 348 */ "key_opt ::= KEY expr",
 /* 349 */ "cmd ::= VACUUM vinto",
 /* 350 */ "cmd ::= VACUUM nm vinto",
 /* 351 */ "vinto ::= INTO expr",
 /* 352 */ "vinto ::=",
 /* 353 */ "ecmd ::= explain cmdx SEMI",
 /* 354 */ "explain ::= EXPLAIN",
 /* 355 */ "explain ::= EXPLAIN QUERY PLAN",
 /* 356 */ "cmd ::= createkw uniqueflag INDEX ifnotexists nm dbnm ON nm LP sortlist RP where_opt",
 /* 357 */ "uniqueflag ::= UNIQUE",
 /* 358 */ "uniqueflag ::=",
 /* 359 */ "ifnotexists ::=",
 /* 360 */ "ifnotexists ::= IF NOT EXISTS",
 /* 361 */ "cmd ::= createkw temp VIEW ifnotexists nm dbnm eidlist_opt AS select",
 /* 362 */ "createkw ::= CREATE",
 /* 363 */ "temp ::= TEMP",
 /* 364 */ "temp ::=",
 /* 365 */ "values ::= VALUES LP nexprlist RP",
 /* 366 */ "mvalues ::= values COMMA LP nexprlist RP",
 /* 367 */ "mvalues ::= mvalues COMMA LP nexprlist RP",
 /* 368 */ "oneselect ::= values",
 /* 369 */ "oneselect ::= mvalues",
 /* 370 */ "cmd ::= create_vtab",
 /* 371 */ "cmd ::= create_vtab LP vtabarglist RP",
 /* 372 */ "create_vtab ::= createkw VIRTUAL TABLE ifnotexists nm dbnm USING nm",
 /* 373 */ "vtabarglist ::= vtabarg",
 /* 374 */ "vtabarglist ::= vtabarglist COMMA vtabarg",
 /* 375 */ "vtabarg ::=",
 /* 376 */ "vtabarg ::= vtabarg vtabargtoken",
 /* 377 */ "vtabargtoken ::= ANY",
 /* 378 */ "vtabargtoken ::= lp anylist RP",
 /* 379 */ "lp ::= LP",
 /* 380 */ "anylist ::=",
 /* 381 */ "anylist ::= anylist LP anylist RP",
 /* 382 */ "anylist ::= anylist ANY",
 /* 383 */ "windowdefn_list ::= windowdefn",
 /* 384 */ "windowdefn_list ::= windowdefn_list COMMA windowdefn",
 /* 385 */ "windowdefn ::= nm AS LP window RP",
 /* 386 */ "window ::= PARTITION BY nexprlist orderby_opt frame_opt",
 /* 387 */ "window ::= nm PARTITION BY nexprlist orderby_opt frame_opt",
 /* 388 */ "window ::= ORDER BY sortlist frame_opt",
 /* 389 */ "window ::= nm ORDER BY sortlist frame_opt",
 /* 390 */ "window ::= frame_opt",
 /* 391 */ "window ::= nm frame_opt",
 /* 392 */ "frame_opt ::=",
 /* 393 */ "frame_opt ::= range_or_rows frame_bound_s frame_exclude_opt",
 /* 394 */ "frame_opt ::= range_or_rows BETWEEN frame_bound_s AND frame_bound_e frame_exclude_opt",
 /* 395 */ "range_or_rows ::= RANGE|ROWS|GROUPS",
 /* 396 */ "frame_bound_s ::= frame_bound",
 /* 397 */ "frame_bound_s ::= UNBOUNDED PRECEDING",
 /* 398 */ "frame_bound_e ::= frame_bound",
 /* 399 */ "frame_bound_e ::= UNBOUNDED FOLLOWING",
 /* 400 */ "frame_bound ::= expr PRECEDING|FOLLOWING",
 /* 401 */ "frame_bound ::= CURRENT ROW",
 /* 402 */ "frame_exclude_opt ::=",
 /* 403 */ "frame_exclude_opt ::= EXCLUDE frame_exclude",
 /* 404 */ "frame_exclude ::= NO OTHERS",
 /* 405 */ "frame_exclude ::= CURRENT ROW",
 /* 406 */ "frame_exclude ::= GROUP|TIES",
 /* 407 */ "window_clause ::= WINDOW windowdefn_list",
 /* 408 */ "filter_over ::= filter_clause over_clause",
 /* 409 */ "filter_over ::= over_clause",
 /* 410 */ "filter_over ::= filter_clause",
 /* 411 */ "over_clause ::= OVER LP window RP",
 /* 412 */ "over_clause ::= OVER nm",
 /* 413 */ "filter_clause ::= FILTER LP WHERE expr RP",
};
#endif /* NDEBUG */


#if YYGROWABLESTACK
/*
** Try to increase the size of the parser stack.  Return the number
** of errors.  Return 0 on success.
*/
static int yyGrowStack(yyParser *p){
  int oldSize = 1 + (int)(p->yystackEnd - p->yystack);
  int newSize;
  int idx;
  yyStackEntry *pNew;

  newSize = oldSize*2 + 100;
  idx = (int)(p->yytos - p->yystack);
  if( p->yystack==p->yystk0 ){
    pNew = YYREALLOC(0, newSize*sizeof(pNew[0]));
    if( pNew==0 ) return 1;
    memcpy(pNew, p->yystack, oldSize*sizeof(pNew[0]));
  }else{
    pNew = YYREALLOC(p->yystack, newSize*sizeof(pNew[0]));
    if( pNew==0 ) return 1;
  }
  p->yystack = pNew;
  p->yytos = &p->yystack[idx];
#ifndef NDEBUG
  if( yyTraceFILE ){
    fprintf(yyTraceFILE,"%sStack grows from %d to %d entries.\n",
            yyTracePrompt, oldSize, newSize);
  }
#endif
  p->yystackEnd = &p->yystack[newSize-1];
  return 0;
}
#endif /* YYGROWABLESTACK */

#if !YYGROWABLESTACK
/* For builds that do no have a growable stack, yyGrowStack always
** returns an error.
*/
# define yyGrowStack(X) 1
#endif

/* Datatype of the argument to the memory allocated passed as the
** second argument to SynqSqliteParseAlloc() below.  This can be changed by
** putting an appropriate #define in the %include section of the input
** grammar.
*/
#ifndef YYMALLOCARGTYPE
# define YYMALLOCARGTYPE size_t
#endif

/* Initialize a new parser that has already been allocated.
*/
void SynqSqliteParseInit(void *yypRawParser SynqSqliteParseCTX_PDECL){
  yyParser *yypParser = (yyParser*)yypRawParser;
  SynqSqliteParseCTX_STORE
#ifdef YYTRACKMAXSTACKDEPTH
  yypParser->yyhwm = 0;
#endif
  yypParser->yystack = yypParser->yystk0;
  yypParser->yystackEnd = &yypParser->yystack[YYSTACKDEPTH-1];
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
void *SynqSqliteParseAlloc(void *(*mallocProc)(YYMALLOCARGTYPE) SynqSqliteParseCTX_PDECL){
  yyParser *yypParser;
  yypParser = (yyParser*)(*mallocProc)( (YYMALLOCARGTYPE)sizeof(yyParser) );
  if( yypParser ){
    SynqSqliteParseCTX_STORE
    SynqSqliteParseInit(yypParser SynqSqliteParseCTX_PARAM);
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
  yyParser *yypParser,    /* The parser */
  YYCODETYPE yymajor,     /* Type code for object to destroy */
  YYMINORTYPE *yypminor   /* The object to be destroyed */
){
  SynqSqliteParseARG_FETCH
  SynqSqliteParseCTX_FETCH
  switch( yymajor ){
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
/********* Begin destructor definitions ***************************************/
/********* End destructor definitions *****************************************/
    default:  break;   /* If no destructor action specified: do nothing */
  }
}

/*
** Pop the parser's stack once.
**
** If there is a destructor routine associated with the token which
** is popped from the stack, then call it.
*/
static void yy_pop_parser_stack(yyParser *pParser){
  yyStackEntry *yytos;
  assert( pParser->yytos!=0 );
  assert( pParser->yytos > pParser->yystack );
  yytos = pParser->yytos--;
#ifndef NDEBUG
  if( yyTraceFILE ){
    fprintf(yyTraceFILE,"%sPopping %s\n",
      yyTracePrompt,
      yyTokenName[yytos->major]);
  }
#endif
  yy_destructor(pParser, yytos->major, &yytos->minor);
}

/*
** Clear all secondary memory allocations from the parser
*/
void SynqSqliteParseFinalize(void *p){
  yyParser *pParser = (yyParser*)p;

  /* In-lined version of calling yy_pop_parser_stack() for each
  ** element left in the stack */
  yyStackEntry *yytos = pParser->yytos;
  while( yytos>pParser->yystack ){
#ifndef NDEBUG
    if( yyTraceFILE ){
      fprintf(yyTraceFILE,"%sPopping %s\n",
        yyTracePrompt,
        yyTokenName[yytos->major]);
    }
#endif
    if( yytos->major>=YY_MIN_DSTRCTR ){
      yy_destructor(pParser, yytos->major, &yytos->minor);
    }
    yytos--;
  }

#if YYGROWABLESTACK
  if( pParser->yystack!=pParser->yystk0 ) YYFREE(pParser->yystack);
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
  void *p,                    /* The parser to be deleted */
  void (*freeProc)(void*)     /* Function used to reclaim memory */
){
#ifndef YYPARSEFREENEVERNULL
  if( p==0 ) return;
#endif
  SynqSqliteParseFinalize(p);
  (*freeProc)(p);
}
#endif /* SynqSqliteParse_ENGINEALWAYSONSTACK */

/*
** Return the peak depth of the stack for a parser.
*/
#ifdef YYTRACKMAXSTACKDEPTH
int SynqSqliteParseStackPeak(void *p){
  yyParser *pParser = (yyParser*)p;
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
int SynqSqliteParseCoverage(FILE *out){
  int stateno, iLookAhead, i;
  int nMissed = 0;
  for(stateno=0; stateno<YYNSTATE; stateno++){
    i = yy_shift_ofst[stateno];
    for(iLookAhead=0; iLookAhead<YYNTOKEN; iLookAhead++){
      if( yy_lookahead[i+iLookAhead]!=iLookAhead ) continue;
      if( yycoverage[stateno][iLookAhead]==0 ) nMissed++;
      if( out ){
        fprintf(out,"State %d lookahead %s %s\n", stateno,
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
  YYCODETYPE iLookAhead,    /* The look-ahead token */
  YYACTIONTYPE stateno      /* Current state number */
){
  int i;

  if( stateno>YY_MAX_SHIFT ) return stateno;
  assert( stateno <= YY_SHIFT_COUNT );
#if defined(YYCOVERAGE)
  yycoverage[stateno][iLookAhead] = 1;
#endif
  do{
    i = yy_shift_ofst[stateno];
    assert( i>=0 );
    assert( i<=YY_ACTTAB_COUNT );
    assert( i+YYNTOKEN<=(int)YY_NLOOKAHEAD );
    assert( iLookAhead!=YYNOCODE );
    assert( iLookAhead < YYNTOKEN );
    i += iLookAhead;
    assert( i<(int)YY_NLOOKAHEAD );
    if( yy_lookahead[i]!=iLookAhead ){
#ifdef YYFALLBACK
      YYCODETYPE iFallback;            /* Fallback token */
      assert( iLookAhead<sizeof(yyFallback)/sizeof(yyFallback[0]) );
      iFallback = yyFallback[iLookAhead];
      if( iFallback!=0 ){
#ifndef NDEBUG
        if( yyTraceFILE ){
          fprintf(yyTraceFILE, "%sFALLBACK %s => %s\n",
             yyTracePrompt, yyTokenName[iLookAhead], yyTokenName[iFallback]);
        }
#endif
        assert( yyFallback[iFallback]==0 ); /* Fallback loop must terminate */
        iLookAhead = iFallback;
        continue;
      }
#endif
#ifdef YYWILDCARD
      {
        int j = i - iLookAhead + YYWILDCARD;
        assert( j<(int)(sizeof(yy_lookahead)/sizeof(yy_lookahead[0])) );
        if( yy_lookahead[j]==YYWILDCARD && iLookAhead>0 ){
#ifndef NDEBUG
          if( yyTraceFILE ){
            fprintf(yyTraceFILE, "%sWILDCARD %s => %s\n",
               yyTracePrompt, yyTokenName[iLookAhead],
               yyTokenName[YYWILDCARD]);
          }
#endif /* NDEBUG */
          return yy_action[j];
        }
      }
#endif /* YYWILDCARD */
      return yy_default[stateno];
    }else{
      assert( i>=0 && i<(int)(sizeof(yy_action)/sizeof(yy_action[0])) );
      return yy_action[i];
    }
  }while(1);
}

/*
** Find the appropriate action for a parser given the non-terminal
** look-ahead token iLookAhead.
*/
static YYACTIONTYPE yy_find_reduce_action(
  YYACTIONTYPE stateno,     /* Current state number */
  YYCODETYPE iLookAhead     /* The look-ahead token */
){
  int i;
#ifdef YYERRORSYMBOL
  if( stateno>YY_REDUCE_COUNT ){
    return yy_default[stateno];
  }
#else
  assert( stateno<=YY_REDUCE_COUNT );
#endif
  i = yy_reduce_ofst[stateno];
  assert( iLookAhead!=YYNOCODE );
  i += iLookAhead;
#ifdef YYERRORSYMBOL
  if( i<0 || i>=YY_ACTTAB_COUNT || yy_lookahead[i]!=iLookAhead ){
    return yy_default[stateno];
  }
#else
  assert( i>=0 && i<YY_ACTTAB_COUNT );
  assert( yy_lookahead[i]==iLookAhead );
#endif
  return yy_action[i];
}

/*
** The following routine is called if the stack overflows.
*/
static void yyStackOverflow(yyParser *yypParser){
   SynqSqliteParseARG_FETCH
   SynqSqliteParseCTX_FETCH
#ifndef NDEBUG
   if( yyTraceFILE ){
     fprintf(yyTraceFILE,"%sStack Overflow!\n",yyTracePrompt);
   }
#endif
   while( yypParser->yytos>yypParser->yystack ) yy_pop_parser_stack(yypParser);
   /* Here code is inserted which will execute if the parser
   ** stack every overflows */
/******** Begin %stack_overflow code ******************************************/

  if (pCtx) {
    pCtx->error = 1;
  }
/******** End %stack_overflow code ********************************************/
   SynqSqliteParseARG_STORE /* Suppress warning about unused %extra_argument var */
   SynqSqliteParseCTX_STORE
}

/*
** Print tracing information for a SHIFT action
*/
#ifndef NDEBUG
static void yyTraceShift(yyParser *yypParser, int yyNewState, const char *zTag){
  if( yyTraceFILE ){
    if( yyNewState<YYNSTATE ){
      fprintf(yyTraceFILE,"%s%s '%s', go to state %d\n",
         yyTracePrompt, zTag, yyTokenName[yypParser->yytos->major],
         yyNewState);
    }else{
      fprintf(yyTraceFILE,"%s%s '%s', pending reduce %d\n",
         yyTracePrompt, zTag, yyTokenName[yypParser->yytos->major],
         yyNewState - YY_MIN_REDUCE);
    }
  }
}
#else
# define yyTraceShift(X,Y,Z)
#endif

/*
** Perform a shift action.
*/
static void yy_shift(
  yyParser *yypParser,          /* The parser to be shifted */
  YYACTIONTYPE yyNewState,      /* The new state to shift in */
  YYCODETYPE yyMajor,           /* The major token to shift in */
  SynqSqliteParseTOKENTYPE yyMinor        /* The minor token to shift in */
){
  yyStackEntry *yytos;
  yypParser->yytos++;
#ifdef YYTRACKMAXSTACKDEPTH
  if( (int)(yypParser->yytos - yypParser->yystack)>yypParser->yyhwm ){
    yypParser->yyhwm++;
    assert( yypParser->yyhwm == (int)(yypParser->yytos - yypParser->yystack) );
  }
#endif
  yytos = yypParser->yytos;
  if( yytos>yypParser->yystackEnd ){
    if( yyGrowStack(yypParser) ){
      yypParser->yytos--;
      yyStackOverflow(yypParser);
      return;
    }
    yytos = yypParser->yytos;
    assert( yytos <= yypParser->yystackEnd );
  }
  if( yyNewState > YY_MAX_SHIFT ){
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
   188,  /* (0) input ::= cmdlist */
   189,  /* (1) cmdlist ::= cmdlist ecmd */
   189,  /* (2) cmdlist ::= ecmd */
   190,  /* (3) ecmd ::= SEMI */
   190,  /* (4) ecmd ::= cmdx SEMI */
   190,  /* (5) ecmd ::= error SEMI */
   191,  /* (6) cmdx ::= cmd */
   194,  /* (7) expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist ORDER BY sortlist RP */
   194,  /* (8) expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist ORDER BY sortlist RP filter_over */
   194,  /* (9) expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist RP WITHIN GROUP LP ORDER BY expr RP */
   194,  /* (10) expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist RP WITHIN GROUP LP ORDER BY expr RP filter_over */
   194,  /* (11) expr ::= CAST LP expr AS typetoken RP */
   199,  /* (12) typetoken ::= */
   199,  /* (13) typetoken ::= typename */
   199,  /* (14) typetoken ::= typename LP signed RP */
   199,  /* (15) typetoken ::= typename LP signed COMMA signed RP */
   200,  /* (16) typename ::= ID|STRING */
   200,  /* (17) typename ::= typename ID|STRING */
   202,  /* (18) selcollist ::= sclp scanpt nm DOT STAR */
   194,  /* (19) expr ::= ID|INDEXED|JOIN_KW */
   194,  /* (20) expr ::= nm DOT nm */
   194,  /* (21) expr ::= nm DOT nm DOT nm */
   209,  /* (22) selectnowith ::= selectnowith multiselect_op oneselect */
   206,  /* (23) multiselect_op ::= UNION */
   206,  /* (24) multiselect_op ::= UNION ALL */
   206,  /* (25) multiselect_op ::= EXCEPT|INTERSECT */
   194,  /* (26) expr ::= LP select RP */
   194,  /* (27) expr ::= EXISTS LP select RP */
   207,  /* (28) in_op ::= IN */
   207,  /* (29) in_op ::= NOT IN */
   194,  /* (30) expr ::= expr in_op LP exprlist RP */
   194,  /* (31) expr ::= expr in_op LP select RP */
   194,  /* (32) expr ::= expr in_op nm dbnm paren_exprlist */
   208,  /* (33) dbnm ::= */
   208,  /* (34) dbnm ::= DOT nm */
   212,  /* (35) paren_exprlist ::= */
   212,  /* (36) paren_exprlist ::= LP exprlist RP */
   194,  /* (37) expr ::= expr ISNULL|NOTNULL */
   194,  /* (38) expr ::= expr NOT NULL */
   194,  /* (39) expr ::= expr IS expr */
   194,  /* (40) expr ::= expr IS NOT expr */
   194,  /* (41) expr ::= expr IS NOT DISTINCT FROM expr */
   194,  /* (42) expr ::= expr IS DISTINCT FROM expr */
   214,  /* (43) between_op ::= BETWEEN */
   214,  /* (44) between_op ::= NOT BETWEEN */
   194,  /* (45) expr ::= expr between_op expr AND expr */
   213,  /* (46) likeop ::= LIKE_KW|MATCH */
   213,  /* (47) likeop ::= NOT LIKE_KW|MATCH */
   194,  /* (48) expr ::= expr likeop expr */
   194,  /* (49) expr ::= expr likeop expr ESCAPE expr */
   194,  /* (50) expr ::= CASE case_operand case_exprlist case_else END */
   216,  /* (51) case_exprlist ::= case_exprlist WHEN expr THEN expr */
   216,  /* (52) case_exprlist ::= WHEN expr THEN expr */
   217,  /* (53) case_else ::= ELSE expr */
   217,  /* (54) case_else ::= */
   215,  /* (55) case_operand ::= expr */
   215,  /* (56) case_operand ::= */
   193,  /* (57) cmd ::= create_table create_table_args */
   235,  /* (58) create_table ::= createkw temp TABLE ifnotexists nm dbnm */
   236,  /* (59) create_table_args ::= LP columnlist conslist_opt RP table_option_set */
   236,  /* (60) create_table_args ::= AS select */
   226,  /* (61) table_option_set ::= */
   226,  /* (62) table_option_set ::= table_option */
   226,  /* (63) table_option_set ::= table_option_set COMMA table_option */
   227,  /* (64) table_option ::= WITHOUT nm */
   227,  /* (65) table_option ::= nm */
   240,  /* (66) columnlist ::= columnlist COMMA columnname carglist */
   240,  /* (67) columnlist ::= columnname carglist */
   231,  /* (68) carglist ::= carglist ccons */
   231,  /* (69) carglist ::= */
   230,  /* (70) ccons ::= CONSTRAINT nm */
   230,  /* (71) ccons ::= DEFAULT scantok term */
   230,  /* (72) ccons ::= DEFAULT LP expr RP */
   230,  /* (73) ccons ::= DEFAULT PLUS scantok term */
   230,  /* (74) ccons ::= DEFAULT MINUS scantok term */
   230,  /* (75) ccons ::= DEFAULT scantok ID|INDEXED */
   230,  /* (76) ccons ::= NULL onconf */
   230,  /* (77) ccons ::= NOT NULL onconf */
   230,  /* (78) ccons ::= PRIMARY KEY sortorder onconf autoinc */
   230,  /* (79) ccons ::= UNIQUE onconf */
   230,  /* (80) ccons ::= CHECK LP expr RP */
   230,  /* (81) ccons ::= REFERENCES nm eidlist_opt refargs */
   230,  /* (82) ccons ::= defer_subclause */
   230,  /* (83) ccons ::= COLLATE ID|STRING */
   230,  /* (84) ccons ::= GENERATED ALWAYS AS generated */
   230,  /* (85) ccons ::= AS generated */
   234,  /* (86) generated ::= LP expr RP */
   234,  /* (87) generated ::= LP expr RP ID */
   219,  /* (88) autoinc ::= */
   219,  /* (89) autoinc ::= AUTOINCR */
   220,  /* (90) refargs ::= */
   220,  /* (91) refargs ::= refargs refarg */
   221,  /* (92) refarg ::= MATCH nm */
   221,  /* (93) refarg ::= ON INSERT refact */
   221,  /* (94) refarg ::= ON DELETE refact */
   221,  /* (95) refarg ::= ON UPDATE refact */
   222,  /* (96) refact ::= SET NULL */
   222,  /* (97) refact ::= SET DEFAULT */
   222,  /* (98) refact ::= CASCADE */
   222,  /* (99) refact ::= RESTRICT */
   222,  /* (100) refact ::= NO ACTION */
   223,  /* (101) defer_subclause ::= NOT DEFERRABLE init_deferred_pred_opt */
   223,  /* (102) defer_subclause ::= DEFERRABLE init_deferred_pred_opt */
   224,  /* (103) init_deferred_pred_opt ::= */
   224,  /* (104) init_deferred_pred_opt ::= INITIALLY DEFERRED */
   224,  /* (105) init_deferred_pred_opt ::= INITIALLY IMMEDIATE */
   241,  /* (106) conslist_opt ::= */
   241,  /* (107) conslist_opt ::= COMMA conslist */
   233,  /* (108) conslist ::= conslist tconscomma tcons */
   233,  /* (109) conslist ::= tcons */
   228,  /* (110) tconscomma ::= COMMA */
   228,  /* (111) tconscomma ::= */
   232,  /* (112) tcons ::= CONSTRAINT nm */
   232,  /* (113) tcons ::= PRIMARY KEY LP sortlist autoinc RP onconf */
   232,  /* (114) tcons ::= UNIQUE LP sortlist RP onconf */
   232,  /* (115) tcons ::= CHECK LP expr RP onconf */
   232,  /* (116) tcons ::= FOREIGN KEY LP eidlist RP REFERENCES nm eidlist_opt refargs defer_subclause_opt */
   225,  /* (117) defer_subclause_opt ::= */
   225,  /* (118) defer_subclause_opt ::= defer_subclause */
   229,  /* (119) onconf ::= */
   229,  /* (120) onconf ::= ON CONFLICT resolvetype */
   218,  /* (121) scantok ::= */
   211,  /* (122) select ::= WITH wqlist selectnowith */
   211,  /* (123) select ::= WITH RECURSIVE wqlist selectnowith */
   252,  /* (124) wqitem ::= withnm eidlist_opt wqas LP select RP */
   251,  /* (125) wqlist ::= wqitem */
   251,  /* (126) wqlist ::= wqlist COMMA wqitem */
   248,  /* (127) withnm ::= nm */
   249,  /* (128) wqas ::= AS */
   249,  /* (129) wqas ::= AS MATERIALIZED */
   249,  /* (130) wqas ::= AS NOT MATERIALIZED */
   245,  /* (131) eidlist_opt ::= */
   245,  /* (132) eidlist_opt ::= LP eidlist RP */
   246,  /* (133) eidlist ::= nm collate sortorder */
   246,  /* (134) eidlist ::= eidlist COMMA nm collate sortorder */
   250,  /* (135) collate ::= */
   250,  /* (136) collate ::= COLLATE ID|STRING */
   253,  /* (137) with ::= */
   253,  /* (138) with ::= WITH wqlist */
   253,  /* (139) with ::= WITH RECURSIVE wqlist */
   193,  /* (140) cmd ::= with DELETE FROM xfullname indexed_opt where_opt_ret orderby_opt limit_opt */
   193,  /* (141) cmd ::= with UPDATE orconf xfullname indexed_opt SET setlist from where_opt_ret orderby_opt limit_opt */
   193,  /* (142) cmd ::= with insert_cmd INTO xfullname idlist_opt select upsert */
   193,  /* (143) cmd ::= with insert_cmd INTO xfullname idlist_opt DEFAULT VALUES returning */
   254,  /* (144) insert_cmd ::= INSERT orconf */
   254,  /* (145) insert_cmd ::= REPLACE */
   255,  /* (146) orconf ::= */
   255,  /* (147) orconf ::= OR resolvetype */
   247,  /* (148) resolvetype ::= raisetype */
   247,  /* (149) resolvetype ::= IGNORE */
   247,  /* (150) resolvetype ::= REPLACE */
   257,  /* (151) xfullname ::= nm */
   257,  /* (152) xfullname ::= nm DOT nm */
   257,  /* (153) xfullname ::= nm DOT nm AS nm */
   257,  /* (154) xfullname ::= nm AS nm */
   256,  /* (155) indexed_opt ::= */
   256,  /* (156) indexed_opt ::= indexed_by */
   258,  /* (157) where_opt_ret ::= */
   258,  /* (158) where_opt_ret ::= WHERE expr */
   258,  /* (159) where_opt_ret ::= RETURNING selcollist */
   258,  /* (160) where_opt_ret ::= WHERE expr RETURNING selcollist */
   261,  /* (161) setlist ::= setlist COMMA nm EQ expr */
   261,  /* (162) setlist ::= setlist COMMA LP idlist RP EQ expr */
   261,  /* (163) setlist ::= nm EQ expr */
   261,  /* (164) setlist ::= LP idlist RP EQ expr */
   263,  /* (165) idlist_opt ::= */
   263,  /* (166) idlist_opt ::= LP idlist RP */
   264,  /* (167) upsert ::= */
   264,  /* (168) upsert ::= RETURNING selcollist */
   264,  /* (169) upsert ::= ON CONFLICT LP sortlist RP where_opt DO UPDATE SET setlist where_opt upsert */
   264,  /* (170) upsert ::= ON CONFLICT LP sortlist RP where_opt DO NOTHING upsert */
   264,  /* (171) upsert ::= ON CONFLICT DO NOTHING returning */
   264,  /* (172) upsert ::= ON CONFLICT DO UPDATE SET setlist where_opt returning */
   265,  /* (173) returning ::= RETURNING selcollist */
   265,  /* (174) returning ::= */
   194,  /* (175) expr ::= error */
   194,  /* (176) expr ::= term */
   194,  /* (177) expr ::= LP expr RP */
   194,  /* (178) expr ::= expr PLUS|MINUS expr */
   194,  /* (179) expr ::= expr STAR|SLASH|REM expr */
   194,  /* (180) expr ::= expr LT|GT|GE|LE expr */
   194,  /* (181) expr ::= expr EQ|NE expr */
   194,  /* (182) expr ::= expr AND expr */
   194,  /* (183) expr ::= expr OR expr */
   194,  /* (184) expr ::= expr BITAND|BITOR|LSHIFT|RSHIFT expr */
   194,  /* (185) expr ::= expr CONCAT expr */
   194,  /* (186) expr ::= expr PTR expr */
   194,  /* (187) expr ::= PLUS|MINUS expr */
   194,  /* (188) expr ::= BITNOT expr */
   194,  /* (189) expr ::= NOT expr */
   196,  /* (190) exprlist ::= nexprlist */
   196,  /* (191) exprlist ::= */
   270,  /* (192) nexprlist ::= nexprlist COMMA expr */
   270,  /* (193) nexprlist ::= expr */
   194,  /* (194) expr ::= LP nexprlist COMMA expr RP */
   194,  /* (195) expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist RP */
   194,  /* (196) expr ::= ID|INDEXED|JOIN_KW LP STAR RP */
   194,  /* (197) expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist RP filter_over */
   194,  /* (198) expr ::= ID|INDEXED|JOIN_KW LP STAR RP filter_over */
   205,  /* (199) nm ::= error */
   205,  /* (200) nm ::= ID|INDEXED|JOIN_KW */
   205,  /* (201) nm ::= STRING */
   243,  /* (202) term ::= INTEGER */
   243,  /* (203) term ::= STRING */
   243,  /* (204) term ::= NULL|FLOAT|BLOB */
   243,  /* (205) term ::= QNUMBER */
   243,  /* (206) term ::= CTIME_KW */
   194,  /* (207) expr ::= VARIABLE */
   194,  /* (208) expr ::= expr COLLATE ID|STRING */
   197,  /* (209) sortlist ::= sortlist COMMA expr sortorder nulls */
   197,  /* (210) sortlist ::= expr sortorder nulls */
   244,  /* (211) sortorder ::= ASC */
   244,  /* (212) sortorder ::= DESC */
   244,  /* (213) sortorder ::= */
   271,  /* (214) nulls ::= NULLS FIRST */
   271,  /* (215) nulls ::= NULLS LAST */
   271,  /* (216) nulls ::= */
   194,  /* (217) expr ::= RAISE LP IGNORE RP */
   194,  /* (218) expr ::= RAISE LP raisetype COMMA expr RP */
   266,  /* (219) raisetype ::= ROLLBACK */
   266,  /* (220) raisetype ::= ABORT */
   266,  /* (221) raisetype ::= FAIL */
   277,  /* (222) fullname ::= nm */
   277,  /* (223) fullname ::= nm DOT nm */
   272,  /* (224) ifexists ::= IF EXISTS */
   272,  /* (225) ifexists ::= */
   193,  /* (226) cmd ::= DROP TABLE ifexists fullname */
   193,  /* (227) cmd ::= DROP VIEW ifexists fullname */
   193,  /* (228) cmd ::= DROP INDEX ifexists fullname */
   193,  /* (229) cmd ::= DROP TRIGGER ifexists fullname */
   193,  /* (230) cmd ::= ALTER TABLE fullname RENAME TO nm */
   193,  /* (231) cmd ::= ALTER TABLE fullname RENAME kwcolumn_opt nm TO nm */
   193,  /* (232) cmd ::= ALTER TABLE fullname DROP kwcolumn_opt nm */
   193,  /* (233) cmd ::= ALTER TABLE add_column_fullname ADD kwcolumn_opt columnname carglist */
   278,  /* (234) add_column_fullname ::= fullname */
   276,  /* (235) kwcolumn_opt ::= */
   276,  /* (236) kwcolumn_opt ::= COLUMNKW */
   242,  /* (237) columnname ::= nm typetoken */
   193,  /* (238) cmd ::= BEGIN transtype trans_opt */
   193,  /* (239) cmd ::= COMMIT|END trans_opt */
   193,  /* (240) cmd ::= ROLLBACK trans_opt */
   273,  /* (241) transtype ::= */
   273,  /* (242) transtype ::= DEFERRED */
   273,  /* (243) transtype ::= IMMEDIATE */
   273,  /* (244) transtype ::= EXCLUSIVE */
   274,  /* (245) trans_opt ::= */
   274,  /* (246) trans_opt ::= TRANSACTION */
   274,  /* (247) trans_opt ::= TRANSACTION nm */
   275,  /* (248) savepoint_opt ::= SAVEPOINT */
   275,  /* (249) savepoint_opt ::= */
   193,  /* (250) cmd ::= SAVEPOINT nm */
   193,  /* (251) cmd ::= RELEASE savepoint_opt nm */
   193,  /* (252) cmd ::= ROLLBACK trans_opt TO savepoint_opt nm */
   193,  /* (253) cmd ::= select */
   211,  /* (254) select ::= selectnowith */
   209,  /* (255) selectnowith ::= oneselect */
   210,  /* (256) oneselect ::= SELECT distinct selcollist from where_opt groupby_opt having_opt orderby_opt limit_opt */
   210,  /* (257) oneselect ::= SELECT distinct selcollist from where_opt groupby_opt having_opt window_clause orderby_opt limit_opt */
   202,  /* (258) selcollist ::= sclp scanpt expr scanpt as */
   202,  /* (259) selcollist ::= sclp scanpt STAR */
   203,  /* (260) sclp ::= selcollist COMMA */
   203,  /* (261) sclp ::= */
   204,  /* (262) scanpt ::= */
   279,  /* (263) as ::= AS nm */
   279,  /* (264) as ::= ID|STRING */
   279,  /* (265) as ::= */
   195,  /* (266) distinct ::= DISTINCT */
   195,  /* (267) distinct ::= ALL */
   195,  /* (268) distinct ::= */
   262,  /* (269) from ::= */
   262,  /* (270) from ::= FROM seltablist */
   269,  /* (271) where_opt ::= */
   269,  /* (272) where_opt ::= WHERE expr */
   280,  /* (273) groupby_opt ::= */
   280,  /* (274) groupby_opt ::= GROUP BY nexprlist */
   281,  /* (275) having_opt ::= */
   281,  /* (276) having_opt ::= HAVING expr */
   259,  /* (277) orderby_opt ::= */
   259,  /* (278) orderby_opt ::= ORDER BY sortlist */
   260,  /* (279) limit_opt ::= */
   260,  /* (280) limit_opt ::= LIMIT expr */
   260,  /* (281) limit_opt ::= LIMIT expr OFFSET expr */
   260,  /* (282) limit_opt ::= LIMIT expr COMMA expr */
   286,  /* (283) stl_prefix ::= seltablist joinop */
   286,  /* (284) stl_prefix ::= */
   283,  /* (285) seltablist ::= stl_prefix nm dbnm as on_using */
   283,  /* (286) seltablist ::= stl_prefix nm dbnm as indexed_by on_using */
   283,  /* (287) seltablist ::= stl_prefix nm dbnm LP exprlist RP as on_using */
   283,  /* (288) seltablist ::= stl_prefix LP select RP as on_using */
   283,  /* (289) seltablist ::= stl_prefix LP seltablist RP as on_using */
   285,  /* (290) joinop ::= COMMA|JOIN */
   285,  /* (291) joinop ::= JOIN_KW JOIN */
   285,  /* (292) joinop ::= JOIN_KW nm JOIN */
   285,  /* (293) joinop ::= JOIN_KW nm nm JOIN */
   284,  /* (294) on_using ::= ON expr */
   284,  /* (295) on_using ::= USING LP idlist RP */
   284,  /* (296) on_using ::= */
   267,  /* (297) indexed_by ::= INDEXED BY nm */
   267,  /* (298) indexed_by ::= NOT INDEXED */
   268,  /* (299) idlist ::= idlist COMMA nm */
   268,  /* (300) idlist ::= nm */
   193,  /* (301) cmd ::= createkw trigger_decl BEGIN trigger_cmd_list END */
   289,  /* (302) trigger_decl ::= temp TRIGGER ifnotexists nm dbnm trigger_time trigger_event ON fullname foreach_clause when_clause */
   287,  /* (303) trigger_time ::= BEFORE|AFTER */
   287,  /* (304) trigger_time ::= INSTEAD OF */
   287,  /* (305) trigger_time ::= */
   291,  /* (306) trigger_event ::= DELETE|INSERT */
   291,  /* (307) trigger_event ::= UPDATE */
   291,  /* (308) trigger_event ::= UPDATE OF idlist */
   292,  /* (309) foreach_clause ::= */
   292,  /* (310) foreach_clause ::= FOR EACH ROW */
   293,  /* (311) when_clause ::= */
   293,  /* (312) when_clause ::= WHEN expr */
   290,  /* (313) trigger_cmd_list ::= trigger_cmd_list trigger_cmd SEMI */
   290,  /* (314) trigger_cmd_list ::= trigger_cmd SEMI */
   288,  /* (315) trnm ::= nm */
   288,  /* (316) trnm ::= nm DOT nm */
   295,  /* (317) tridxby ::= */
   295,  /* (318) tridxby ::= INDEXED BY nm */
   295,  /* (319) tridxby ::= NOT INDEXED */
   294,  /* (320) trigger_cmd ::= UPDATE orconf trnm tridxby SET setlist from where_opt scanpt */
   294,  /* (321) trigger_cmd ::= scanpt insert_cmd INTO trnm idlist_opt select upsert scanpt */
   294,  /* (322) trigger_cmd ::= DELETE FROM trnm tridxby where_opt scanpt */
   294,  /* (323) trigger_cmd ::= scanpt select scanpt */
   193,  /* (324) cmd ::= PRAGMA nm dbnm */
   193,  /* (325) cmd ::= PRAGMA nm dbnm EQ nmnum */
   193,  /* (326) cmd ::= PRAGMA nm dbnm LP nmnum RP */
   193,  /* (327) cmd ::= PRAGMA nm dbnm EQ minus_num */
   193,  /* (328) cmd ::= PRAGMA nm dbnm LP minus_num RP */
   298,  /* (329) nmnum ::= plus_num */
   298,  /* (330) nmnum ::= nm */
   298,  /* (331) nmnum ::= ON */
   298,  /* (332) nmnum ::= DELETE */
   298,  /* (333) nmnum ::= DEFAULT */
   296,  /* (334) plus_num ::= PLUS INTEGER|FLOAT */
   296,  /* (335) plus_num ::= INTEGER|FLOAT */
   297,  /* (336) minus_num ::= MINUS INTEGER|FLOAT */
   201,  /* (337) signed ::= plus_num */
   201,  /* (338) signed ::= minus_num */
   193,  /* (339) cmd ::= ANALYZE */
   193,  /* (340) cmd ::= ANALYZE nm dbnm */
   193,  /* (341) cmd ::= REINDEX */
   193,  /* (342) cmd ::= REINDEX nm dbnm */
   193,  /* (343) cmd ::= ATTACH database_kw_opt expr AS expr key_opt */
   193,  /* (344) cmd ::= DETACH database_kw_opt expr */
   301,  /* (345) database_kw_opt ::= DATABASE */
   301,  /* (346) database_kw_opt ::= */
   302,  /* (347) key_opt ::= */
   302,  /* (348) key_opt ::= KEY expr */
   193,  /* (349) cmd ::= VACUUM vinto */
   193,  /* (350) cmd ::= VACUUM nm vinto */
   303,  /* (351) vinto ::= INTO expr */
   303,  /* (352) vinto ::= */
   190,  /* (353) ecmd ::= explain cmdx SEMI */
   300,  /* (354) explain ::= EXPLAIN */
   300,  /* (355) explain ::= EXPLAIN QUERY PLAN */
   193,  /* (356) cmd ::= createkw uniqueflag INDEX ifnotexists nm dbnm ON nm LP sortlist RP where_opt */
   299,  /* (357) uniqueflag ::= UNIQUE */
   299,  /* (358) uniqueflag ::= */
   239,  /* (359) ifnotexists ::= */
   239,  /* (360) ifnotexists ::= IF NOT EXISTS */
   193,  /* (361) cmd ::= createkw temp VIEW ifnotexists nm dbnm eidlist_opt AS select */
   237,  /* (362) createkw ::= CREATE */
   238,  /* (363) temp ::= TEMP */
   238,  /* (364) temp ::= */
   304,  /* (365) values ::= VALUES LP nexprlist RP */
   305,  /* (366) mvalues ::= values COMMA LP nexprlist RP */
   305,  /* (367) mvalues ::= mvalues COMMA LP nexprlist RP */
   210,  /* (368) oneselect ::= values */
   210,  /* (369) oneselect ::= mvalues */
   193,  /* (370) cmd ::= create_vtab */
   193,  /* (371) cmd ::= create_vtab LP vtabarglist RP */
   306,  /* (372) create_vtab ::= createkw VIRTUAL TABLE ifnotexists nm dbnm USING nm */
   307,  /* (373) vtabarglist ::= vtabarg */
   307,  /* (374) vtabarglist ::= vtabarglist COMMA vtabarg */
   308,  /* (375) vtabarg ::= */
   308,  /* (376) vtabarg ::= vtabarg vtabargtoken */
   309,  /* (377) vtabargtoken ::= ANY */
   309,  /* (378) vtabargtoken ::= lp anylist RP */
   310,  /* (379) lp ::= LP */
   311,  /* (380) anylist ::= */
   311,  /* (381) anylist ::= anylist LP anylist RP */
   311,  /* (382) anylist ::= anylist ANY */
   315,  /* (383) windowdefn_list ::= windowdefn */
   315,  /* (384) windowdefn_list ::= windowdefn_list COMMA windowdefn */
   316,  /* (385) windowdefn ::= nm AS LP window RP */
   317,  /* (386) window ::= PARTITION BY nexprlist orderby_opt frame_opt */
   317,  /* (387) window ::= nm PARTITION BY nexprlist orderby_opt frame_opt */
   317,  /* (388) window ::= ORDER BY sortlist frame_opt */
   317,  /* (389) window ::= nm ORDER BY sortlist frame_opt */
   317,  /* (390) window ::= frame_opt */
   317,  /* (391) window ::= nm frame_opt */
   318,  /* (392) frame_opt ::= */
   318,  /* (393) frame_opt ::= range_or_rows frame_bound_s frame_exclude_opt */
   318,  /* (394) frame_opt ::= range_or_rows BETWEEN frame_bound_s AND frame_bound_e frame_exclude_opt */
   312,  /* (395) range_or_rows ::= RANGE|ROWS|GROUPS */
   319,  /* (396) frame_bound_s ::= frame_bound */
   319,  /* (397) frame_bound_s ::= UNBOUNDED PRECEDING */
   320,  /* (398) frame_bound_e ::= frame_bound */
   320,  /* (399) frame_bound_e ::= UNBOUNDED FOLLOWING */
   321,  /* (400) frame_bound ::= expr PRECEDING|FOLLOWING */
   321,  /* (401) frame_bound ::= CURRENT ROW */
   313,  /* (402) frame_exclude_opt ::= */
   313,  /* (403) frame_exclude_opt ::= EXCLUDE frame_exclude */
   314,  /* (404) frame_exclude ::= NO OTHERS */
   314,  /* (405) frame_exclude ::= CURRENT ROW */
   314,  /* (406) frame_exclude ::= GROUP|TIES */
   282,  /* (407) window_clause ::= WINDOW windowdefn_list */
   198,  /* (408) filter_over ::= filter_clause over_clause */
   198,  /* (409) filter_over ::= over_clause */
   198,  /* (410) filter_over ::= filter_clause */
   323,  /* (411) over_clause ::= OVER LP window RP */
   323,  /* (412) over_clause ::= OVER nm */
   322,  /* (413) filter_clause ::= FILTER LP WHERE expr RP */
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
   -8,  /* (7) expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist ORDER BY sortlist RP */
   -9,  /* (8) expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist ORDER BY sortlist RP filter_over */
  -12,  /* (9) expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist RP WITHIN GROUP LP ORDER BY expr RP */
  -13,  /* (10) expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist RP WITHIN GROUP LP ORDER BY expr RP filter_over */
   -6,  /* (11) expr ::= CAST LP expr AS typetoken RP */
    0,  /* (12) typetoken ::= */
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
    0,  /* (33) dbnm ::= */
   -2,  /* (34) dbnm ::= DOT nm */
    0,  /* (35) paren_exprlist ::= */
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
    0,  /* (54) case_else ::= */
   -1,  /* (55) case_operand ::= expr */
    0,  /* (56) case_operand ::= */
   -2,  /* (57) cmd ::= create_table create_table_args */
   -6,  /* (58) create_table ::= createkw temp TABLE ifnotexists nm dbnm */
   -5,  /* (59) create_table_args ::= LP columnlist conslist_opt RP table_option_set */
   -2,  /* (60) create_table_args ::= AS select */
    0,  /* (61) table_option_set ::= */
   -1,  /* (62) table_option_set ::= table_option */
   -3,  /* (63) table_option_set ::= table_option_set COMMA table_option */
   -2,  /* (64) table_option ::= WITHOUT nm */
   -1,  /* (65) table_option ::= nm */
   -4,  /* (66) columnlist ::= columnlist COMMA columnname carglist */
   -2,  /* (67) columnlist ::= columnname carglist */
   -2,  /* (68) carglist ::= carglist ccons */
    0,  /* (69) carglist ::= */
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
    0,  /* (88) autoinc ::= */
   -1,  /* (89) autoinc ::= AUTOINCR */
    0,  /* (90) refargs ::= */
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
    0,  /* (103) init_deferred_pred_opt ::= */
   -2,  /* (104) init_deferred_pred_opt ::= INITIALLY DEFERRED */
   -2,  /* (105) init_deferred_pred_opt ::= INITIALLY IMMEDIATE */
    0,  /* (106) conslist_opt ::= */
   -2,  /* (107) conslist_opt ::= COMMA conslist */
   -3,  /* (108) conslist ::= conslist tconscomma tcons */
   -1,  /* (109) conslist ::= tcons */
   -1,  /* (110) tconscomma ::= COMMA */
    0,  /* (111) tconscomma ::= */
   -2,  /* (112) tcons ::= CONSTRAINT nm */
   -7,  /* (113) tcons ::= PRIMARY KEY LP sortlist autoinc RP onconf */
   -5,  /* (114) tcons ::= UNIQUE LP sortlist RP onconf */
   -5,  /* (115) tcons ::= CHECK LP expr RP onconf */
  -10,  /* (116) tcons ::= FOREIGN KEY LP eidlist RP REFERENCES nm eidlist_opt refargs defer_subclause_opt */
    0,  /* (117) defer_subclause_opt ::= */
   -1,  /* (118) defer_subclause_opt ::= defer_subclause */
    0,  /* (119) onconf ::= */
   -3,  /* (120) onconf ::= ON CONFLICT resolvetype */
    0,  /* (121) scantok ::= */
   -3,  /* (122) select ::= WITH wqlist selectnowith */
   -4,  /* (123) select ::= WITH RECURSIVE wqlist selectnowith */
   -6,  /* (124) wqitem ::= withnm eidlist_opt wqas LP select RP */
   -1,  /* (125) wqlist ::= wqitem */
   -3,  /* (126) wqlist ::= wqlist COMMA wqitem */
   -1,  /* (127) withnm ::= nm */
   -1,  /* (128) wqas ::= AS */
   -2,  /* (129) wqas ::= AS MATERIALIZED */
   -3,  /* (130) wqas ::= AS NOT MATERIALIZED */
    0,  /* (131) eidlist_opt ::= */
   -3,  /* (132) eidlist_opt ::= LP eidlist RP */
   -3,  /* (133) eidlist ::= nm collate sortorder */
   -5,  /* (134) eidlist ::= eidlist COMMA nm collate sortorder */
    0,  /* (135) collate ::= */
   -2,  /* (136) collate ::= COLLATE ID|STRING */
    0,  /* (137) with ::= */
   -2,  /* (138) with ::= WITH wqlist */
   -3,  /* (139) with ::= WITH RECURSIVE wqlist */
   -8,  /* (140) cmd ::= with DELETE FROM xfullname indexed_opt where_opt_ret orderby_opt limit_opt */
  -11,  /* (141) cmd ::= with UPDATE orconf xfullname indexed_opt SET setlist from where_opt_ret orderby_opt limit_opt */
   -7,  /* (142) cmd ::= with insert_cmd INTO xfullname idlist_opt select upsert */
   -8,  /* (143) cmd ::= with insert_cmd INTO xfullname idlist_opt DEFAULT VALUES returning */
   -2,  /* (144) insert_cmd ::= INSERT orconf */
   -1,  /* (145) insert_cmd ::= REPLACE */
    0,  /* (146) orconf ::= */
   -2,  /* (147) orconf ::= OR resolvetype */
   -1,  /* (148) resolvetype ::= raisetype */
   -1,  /* (149) resolvetype ::= IGNORE */
   -1,  /* (150) resolvetype ::= REPLACE */
   -1,  /* (151) xfullname ::= nm */
   -3,  /* (152) xfullname ::= nm DOT nm */
   -5,  /* (153) xfullname ::= nm DOT nm AS nm */
   -3,  /* (154) xfullname ::= nm AS nm */
    0,  /* (155) indexed_opt ::= */
   -1,  /* (156) indexed_opt ::= indexed_by */
    0,  /* (157) where_opt_ret ::= */
   -2,  /* (158) where_opt_ret ::= WHERE expr */
   -2,  /* (159) where_opt_ret ::= RETURNING selcollist */
   -4,  /* (160) where_opt_ret ::= WHERE expr RETURNING selcollist */
   -5,  /* (161) setlist ::= setlist COMMA nm EQ expr */
   -7,  /* (162) setlist ::= setlist COMMA LP idlist RP EQ expr */
   -3,  /* (163) setlist ::= nm EQ expr */
   -5,  /* (164) setlist ::= LP idlist RP EQ expr */
    0,  /* (165) idlist_opt ::= */
   -3,  /* (166) idlist_opt ::= LP idlist RP */
    0,  /* (167) upsert ::= */
   -2,  /* (168) upsert ::= RETURNING selcollist */
  -12,  /* (169) upsert ::= ON CONFLICT LP sortlist RP where_opt DO UPDATE SET setlist where_opt upsert */
   -9,  /* (170) upsert ::= ON CONFLICT LP sortlist RP where_opt DO NOTHING upsert */
   -5,  /* (171) upsert ::= ON CONFLICT DO NOTHING returning */
   -8,  /* (172) upsert ::= ON CONFLICT DO UPDATE SET setlist where_opt returning */
   -2,  /* (173) returning ::= RETURNING selcollist */
    0,  /* (174) returning ::= */
   -1,  /* (175) expr ::= error */
   -1,  /* (176) expr ::= term */
   -3,  /* (177) expr ::= LP expr RP */
   -3,  /* (178) expr ::= expr PLUS|MINUS expr */
   -3,  /* (179) expr ::= expr STAR|SLASH|REM expr */
   -3,  /* (180) expr ::= expr LT|GT|GE|LE expr */
   -3,  /* (181) expr ::= expr EQ|NE expr */
   -3,  /* (182) expr ::= expr AND expr */
   -3,  /* (183) expr ::= expr OR expr */
   -3,  /* (184) expr ::= expr BITAND|BITOR|LSHIFT|RSHIFT expr */
   -3,  /* (185) expr ::= expr CONCAT expr */
   -3,  /* (186) expr ::= expr PTR expr */
   -2,  /* (187) expr ::= PLUS|MINUS expr */
   -2,  /* (188) expr ::= BITNOT expr */
   -2,  /* (189) expr ::= NOT expr */
   -1,  /* (190) exprlist ::= nexprlist */
    0,  /* (191) exprlist ::= */
   -3,  /* (192) nexprlist ::= nexprlist COMMA expr */
   -1,  /* (193) nexprlist ::= expr */
   -5,  /* (194) expr ::= LP nexprlist COMMA expr RP */
   -5,  /* (195) expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist RP */
   -4,  /* (196) expr ::= ID|INDEXED|JOIN_KW LP STAR RP */
   -6,  /* (197) expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist RP filter_over */
   -5,  /* (198) expr ::= ID|INDEXED|JOIN_KW LP STAR RP filter_over */
   -1,  /* (199) nm ::= error */
   -1,  /* (200) nm ::= ID|INDEXED|JOIN_KW */
   -1,  /* (201) nm ::= STRING */
   -1,  /* (202) term ::= INTEGER */
   -1,  /* (203) term ::= STRING */
   -1,  /* (204) term ::= NULL|FLOAT|BLOB */
   -1,  /* (205) term ::= QNUMBER */
   -1,  /* (206) term ::= CTIME_KW */
   -1,  /* (207) expr ::= VARIABLE */
   -3,  /* (208) expr ::= expr COLLATE ID|STRING */
   -5,  /* (209) sortlist ::= sortlist COMMA expr sortorder nulls */
   -3,  /* (210) sortlist ::= expr sortorder nulls */
   -1,  /* (211) sortorder ::= ASC */
   -1,  /* (212) sortorder ::= DESC */
    0,  /* (213) sortorder ::= */
   -2,  /* (214) nulls ::= NULLS FIRST */
   -2,  /* (215) nulls ::= NULLS LAST */
    0,  /* (216) nulls ::= */
   -4,  /* (217) expr ::= RAISE LP IGNORE RP */
   -6,  /* (218) expr ::= RAISE LP raisetype COMMA expr RP */
   -1,  /* (219) raisetype ::= ROLLBACK */
   -1,  /* (220) raisetype ::= ABORT */
   -1,  /* (221) raisetype ::= FAIL */
   -1,  /* (222) fullname ::= nm */
   -3,  /* (223) fullname ::= nm DOT nm */
   -2,  /* (224) ifexists ::= IF EXISTS */
    0,  /* (225) ifexists ::= */
   -4,  /* (226) cmd ::= DROP TABLE ifexists fullname */
   -4,  /* (227) cmd ::= DROP VIEW ifexists fullname */
   -4,  /* (228) cmd ::= DROP INDEX ifexists fullname */
   -4,  /* (229) cmd ::= DROP TRIGGER ifexists fullname */
   -6,  /* (230) cmd ::= ALTER TABLE fullname RENAME TO nm */
   -8,  /* (231) cmd ::= ALTER TABLE fullname RENAME kwcolumn_opt nm TO nm */
   -6,  /* (232) cmd ::= ALTER TABLE fullname DROP kwcolumn_opt nm */
   -7,  /* (233) cmd ::= ALTER TABLE add_column_fullname ADD kwcolumn_opt columnname carglist */
   -1,  /* (234) add_column_fullname ::= fullname */
    0,  /* (235) kwcolumn_opt ::= */
   -1,  /* (236) kwcolumn_opt ::= COLUMNKW */
   -2,  /* (237) columnname ::= nm typetoken */
   -3,  /* (238) cmd ::= BEGIN transtype trans_opt */
   -2,  /* (239) cmd ::= COMMIT|END trans_opt */
   -2,  /* (240) cmd ::= ROLLBACK trans_opt */
    0,  /* (241) transtype ::= */
   -1,  /* (242) transtype ::= DEFERRED */
   -1,  /* (243) transtype ::= IMMEDIATE */
   -1,  /* (244) transtype ::= EXCLUSIVE */
    0,  /* (245) trans_opt ::= */
   -1,  /* (246) trans_opt ::= TRANSACTION */
   -2,  /* (247) trans_opt ::= TRANSACTION nm */
   -1,  /* (248) savepoint_opt ::= SAVEPOINT */
    0,  /* (249) savepoint_opt ::= */
   -2,  /* (250) cmd ::= SAVEPOINT nm */
   -3,  /* (251) cmd ::= RELEASE savepoint_opt nm */
   -5,  /* (252) cmd ::= ROLLBACK trans_opt TO savepoint_opt nm */
   -1,  /* (253) cmd ::= select */
   -1,  /* (254) select ::= selectnowith */
   -1,  /* (255) selectnowith ::= oneselect */
   -9,  /* (256) oneselect ::= SELECT distinct selcollist from where_opt groupby_opt having_opt orderby_opt limit_opt */
  -10,  /* (257) oneselect ::= SELECT distinct selcollist from where_opt groupby_opt having_opt window_clause orderby_opt limit_opt */
   -5,  /* (258) selcollist ::= sclp scanpt expr scanpt as */
   -3,  /* (259) selcollist ::= sclp scanpt STAR */
   -2,  /* (260) sclp ::= selcollist COMMA */
    0,  /* (261) sclp ::= */
    0,  /* (262) scanpt ::= */
   -2,  /* (263) as ::= AS nm */
   -1,  /* (264) as ::= ID|STRING */
    0,  /* (265) as ::= */
   -1,  /* (266) distinct ::= DISTINCT */
   -1,  /* (267) distinct ::= ALL */
    0,  /* (268) distinct ::= */
    0,  /* (269) from ::= */
   -2,  /* (270) from ::= FROM seltablist */
    0,  /* (271) where_opt ::= */
   -2,  /* (272) where_opt ::= WHERE expr */
    0,  /* (273) groupby_opt ::= */
   -3,  /* (274) groupby_opt ::= GROUP BY nexprlist */
    0,  /* (275) having_opt ::= */
   -2,  /* (276) having_opt ::= HAVING expr */
    0,  /* (277) orderby_opt ::= */
   -3,  /* (278) orderby_opt ::= ORDER BY sortlist */
    0,  /* (279) limit_opt ::= */
   -2,  /* (280) limit_opt ::= LIMIT expr */
   -4,  /* (281) limit_opt ::= LIMIT expr OFFSET expr */
   -4,  /* (282) limit_opt ::= LIMIT expr COMMA expr */
   -2,  /* (283) stl_prefix ::= seltablist joinop */
    0,  /* (284) stl_prefix ::= */
   -5,  /* (285) seltablist ::= stl_prefix nm dbnm as on_using */
   -6,  /* (286) seltablist ::= stl_prefix nm dbnm as indexed_by on_using */
   -8,  /* (287) seltablist ::= stl_prefix nm dbnm LP exprlist RP as on_using */
   -6,  /* (288) seltablist ::= stl_prefix LP select RP as on_using */
   -6,  /* (289) seltablist ::= stl_prefix LP seltablist RP as on_using */
   -1,  /* (290) joinop ::= COMMA|JOIN */
   -2,  /* (291) joinop ::= JOIN_KW JOIN */
   -3,  /* (292) joinop ::= JOIN_KW nm JOIN */
   -4,  /* (293) joinop ::= JOIN_KW nm nm JOIN */
   -2,  /* (294) on_using ::= ON expr */
   -4,  /* (295) on_using ::= USING LP idlist RP */
    0,  /* (296) on_using ::= */
   -3,  /* (297) indexed_by ::= INDEXED BY nm */
   -2,  /* (298) indexed_by ::= NOT INDEXED */
   -3,  /* (299) idlist ::= idlist COMMA nm */
   -1,  /* (300) idlist ::= nm */
   -5,  /* (301) cmd ::= createkw trigger_decl BEGIN trigger_cmd_list END */
  -11,  /* (302) trigger_decl ::= temp TRIGGER ifnotexists nm dbnm trigger_time trigger_event ON fullname foreach_clause when_clause */
   -1,  /* (303) trigger_time ::= BEFORE|AFTER */
   -2,  /* (304) trigger_time ::= INSTEAD OF */
    0,  /* (305) trigger_time ::= */
   -1,  /* (306) trigger_event ::= DELETE|INSERT */
   -1,  /* (307) trigger_event ::= UPDATE */
   -3,  /* (308) trigger_event ::= UPDATE OF idlist */
    0,  /* (309) foreach_clause ::= */
   -3,  /* (310) foreach_clause ::= FOR EACH ROW */
    0,  /* (311) when_clause ::= */
   -2,  /* (312) when_clause ::= WHEN expr */
   -3,  /* (313) trigger_cmd_list ::= trigger_cmd_list trigger_cmd SEMI */
   -2,  /* (314) trigger_cmd_list ::= trigger_cmd SEMI */
   -1,  /* (315) trnm ::= nm */
   -3,  /* (316) trnm ::= nm DOT nm */
    0,  /* (317) tridxby ::= */
   -3,  /* (318) tridxby ::= INDEXED BY nm */
   -2,  /* (319) tridxby ::= NOT INDEXED */
   -9,  /* (320) trigger_cmd ::= UPDATE orconf trnm tridxby SET setlist from where_opt scanpt */
   -8,  /* (321) trigger_cmd ::= scanpt insert_cmd INTO trnm idlist_opt select upsert scanpt */
   -6,  /* (322) trigger_cmd ::= DELETE FROM trnm tridxby where_opt scanpt */
   -3,  /* (323) trigger_cmd ::= scanpt select scanpt */
   -3,  /* (324) cmd ::= PRAGMA nm dbnm */
   -5,  /* (325) cmd ::= PRAGMA nm dbnm EQ nmnum */
   -6,  /* (326) cmd ::= PRAGMA nm dbnm LP nmnum RP */
   -5,  /* (327) cmd ::= PRAGMA nm dbnm EQ minus_num */
   -6,  /* (328) cmd ::= PRAGMA nm dbnm LP minus_num RP */
   -1,  /* (329) nmnum ::= plus_num */
   -1,  /* (330) nmnum ::= nm */
   -1,  /* (331) nmnum ::= ON */
   -1,  /* (332) nmnum ::= DELETE */
   -1,  /* (333) nmnum ::= DEFAULT */
   -2,  /* (334) plus_num ::= PLUS INTEGER|FLOAT */
   -1,  /* (335) plus_num ::= INTEGER|FLOAT */
   -2,  /* (336) minus_num ::= MINUS INTEGER|FLOAT */
   -1,  /* (337) signed ::= plus_num */
   -1,  /* (338) signed ::= minus_num */
   -1,  /* (339) cmd ::= ANALYZE */
   -3,  /* (340) cmd ::= ANALYZE nm dbnm */
   -1,  /* (341) cmd ::= REINDEX */
   -3,  /* (342) cmd ::= REINDEX nm dbnm */
   -6,  /* (343) cmd ::= ATTACH database_kw_opt expr AS expr key_opt */
   -3,  /* (344) cmd ::= DETACH database_kw_opt expr */
   -1,  /* (345) database_kw_opt ::= DATABASE */
    0,  /* (346) database_kw_opt ::= */
    0,  /* (347) key_opt ::= */
   -2,  /* (348) key_opt ::= KEY expr */
   -2,  /* (349) cmd ::= VACUUM vinto */
   -3,  /* (350) cmd ::= VACUUM nm vinto */
   -2,  /* (351) vinto ::= INTO expr */
    0,  /* (352) vinto ::= */
   -3,  /* (353) ecmd ::= explain cmdx SEMI */
   -1,  /* (354) explain ::= EXPLAIN */
   -3,  /* (355) explain ::= EXPLAIN QUERY PLAN */
  -12,  /* (356) cmd ::= createkw uniqueflag INDEX ifnotexists nm dbnm ON nm LP sortlist RP where_opt */
   -1,  /* (357) uniqueflag ::= UNIQUE */
    0,  /* (358) uniqueflag ::= */
    0,  /* (359) ifnotexists ::= */
   -3,  /* (360) ifnotexists ::= IF NOT EXISTS */
   -9,  /* (361) cmd ::= createkw temp VIEW ifnotexists nm dbnm eidlist_opt AS select */
   -1,  /* (362) createkw ::= CREATE */
   -1,  /* (363) temp ::= TEMP */
    0,  /* (364) temp ::= */
   -4,  /* (365) values ::= VALUES LP nexprlist RP */
   -5,  /* (366) mvalues ::= values COMMA LP nexprlist RP */
   -5,  /* (367) mvalues ::= mvalues COMMA LP nexprlist RP */
   -1,  /* (368) oneselect ::= values */
   -1,  /* (369) oneselect ::= mvalues */
   -1,  /* (370) cmd ::= create_vtab */
   -4,  /* (371) cmd ::= create_vtab LP vtabarglist RP */
   -8,  /* (372) create_vtab ::= createkw VIRTUAL TABLE ifnotexists nm dbnm USING nm */
   -1,  /* (373) vtabarglist ::= vtabarg */
   -3,  /* (374) vtabarglist ::= vtabarglist COMMA vtabarg */
    0,  /* (375) vtabarg ::= */
   -2,  /* (376) vtabarg ::= vtabarg vtabargtoken */
   -1,  /* (377) vtabargtoken ::= ANY */
   -3,  /* (378) vtabargtoken ::= lp anylist RP */
   -1,  /* (379) lp ::= LP */
    0,  /* (380) anylist ::= */
   -4,  /* (381) anylist ::= anylist LP anylist RP */
   -2,  /* (382) anylist ::= anylist ANY */
   -1,  /* (383) windowdefn_list ::= windowdefn */
   -3,  /* (384) windowdefn_list ::= windowdefn_list COMMA windowdefn */
   -5,  /* (385) windowdefn ::= nm AS LP window RP */
   -5,  /* (386) window ::= PARTITION BY nexprlist orderby_opt frame_opt */
   -6,  /* (387) window ::= nm PARTITION BY nexprlist orderby_opt frame_opt */
   -4,  /* (388) window ::= ORDER BY sortlist frame_opt */
   -5,  /* (389) window ::= nm ORDER BY sortlist frame_opt */
   -1,  /* (390) window ::= frame_opt */
   -2,  /* (391) window ::= nm frame_opt */
    0,  /* (392) frame_opt ::= */
   -3,  /* (393) frame_opt ::= range_or_rows frame_bound_s frame_exclude_opt */
   -6,  /* (394) frame_opt ::= range_or_rows BETWEEN frame_bound_s AND frame_bound_e frame_exclude_opt */
   -1,  /* (395) range_or_rows ::= RANGE|ROWS|GROUPS */
   -1,  /* (396) frame_bound_s ::= frame_bound */
   -2,  /* (397) frame_bound_s ::= UNBOUNDED PRECEDING */
   -1,  /* (398) frame_bound_e ::= frame_bound */
   -2,  /* (399) frame_bound_e ::= UNBOUNDED FOLLOWING */
   -2,  /* (400) frame_bound ::= expr PRECEDING|FOLLOWING */
   -2,  /* (401) frame_bound ::= CURRENT ROW */
    0,  /* (402) frame_exclude_opt ::= */
   -2,  /* (403) frame_exclude_opt ::= EXCLUDE frame_exclude */
   -2,  /* (404) frame_exclude ::= NO OTHERS */
   -2,  /* (405) frame_exclude ::= CURRENT ROW */
   -1,  /* (406) frame_exclude ::= GROUP|TIES */
   -2,  /* (407) window_clause ::= WINDOW windowdefn_list */
   -2,  /* (408) filter_over ::= filter_clause over_clause */
   -1,  /* (409) filter_over ::= over_clause */
   -1,  /* (410) filter_over ::= filter_clause */
   -4,  /* (411) over_clause ::= OVER LP window RP */
   -2,  /* (412) over_clause ::= OVER nm */
   -5,  /* (413) filter_clause ::= FILTER LP WHERE expr RP */
};

static void yy_accept(yyParser*);  /* Forward Declaration */

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
  yyParser *yypParser,         /* The parser */
  unsigned int yyruleno,       /* Number of the rule by which to reduce */
  int yyLookahead,             /* Lookahead token, or YYNOCODE if none */
  SynqSqliteParseTOKENTYPE yyLookaheadToken  /* Value of the lookahead token */
  SynqSqliteParseCTX_PDECL                   /* %extra_context */
){
  int yygoto;                     /* The next state */
  YYACTIONTYPE yyact;             /* The next action */
  yyStackEntry *yymsp;            /* The top of the parser's stack */
  int yysize;                     /* Amount to pop the stack */
  SynqSqliteParseARG_FETCH
  (void)yyLookahead;
  (void)yyLookaheadToken;
  yymsp = yypParser->yytos;

  switch( yyruleno ){
  /* Beginning here are the reduction cases.  A typical example
  ** follows:
  **   case 0:
  **  #line <lineno> <grammarfile>
  **     { ... }           // User supplied code
  **  #line <lineno> <thisfile>
  **     break;
  */
/********** Begin reduce actions **********************************************/
        YYMINORTYPE yylhsminor;
      case 0: /* input ::= cmdlist */
{
    pCtx->root = yymsp[0].minor.yy213;
}
        break;
      case 1: /* cmdlist ::= cmdlist ecmd */
{
    yymsp[-1].minor.yy213 = yymsp[0].minor.yy213;  // Just use the last command for now
}
        break;
      case 2: /* cmdlist ::= ecmd */
      case 6: /* cmdx ::= cmd */ yytestcase(yyruleno==6);
      case 55: /* case_operand ::= expr */ yytestcase(yyruleno==55);
      case 176: /* expr ::= term */ yytestcase(yyruleno==176);
      case 190: /* exprlist ::= nexprlist */ yytestcase(yyruleno==190);
      case 253: /* cmd ::= select */ yytestcase(yyruleno==253);
      case 254: /* select ::= selectnowith */ yytestcase(yyruleno==254);
      case 255: /* selectnowith ::= oneselect */ yytestcase(yyruleno==255);
      case 370: /* cmd ::= create_vtab */ yytestcase(yyruleno==370);
      case 396: /* frame_bound_s ::= frame_bound */ yytestcase(yyruleno==396);
      case 398: /* frame_bound_e ::= frame_bound */ yytestcase(yyruleno==398);
      case 409: /* filter_over ::= over_clause */ yytestcase(yyruleno==409);
{
    yylhsminor.yy213 = yymsp[0].minor.yy213;
}
  yymsp[0].minor.yy213 = yylhsminor.yy213;
        break;
      case 3: /* ecmd ::= SEMI */
{
    yymsp[0].minor.yy213 = SYNTAQLITE_NULL_NODE;
    pCtx->stmt_completed = 1;
}
        break;
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
}
        break;
      case 7: /* expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist ORDER BY sortlist RP */
{
    synq_mark_as_function(pCtx, yymsp[-7].minor.yy0);
    yylhsminor.yy213 = synq_parse_aggregate_function_call(pCtx,
        synq_span(pCtx, yymsp[-7].minor.yy0),
        (SyntaqliteAggregateFunctionCallFlags){.raw = (uint8_t)yymsp[-5].minor.yy213},
        yymsp[-4].minor.yy213,
        yymsp[-1].minor.yy213,
        SYNTAQLITE_NULL_NODE,
        SYNTAQLITE_NULL_NODE);
}
  yymsp[-7].minor.yy213 = yylhsminor.yy213;
        break;
      case 8: /* expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist ORDER BY sortlist RP filter_over */
{
    SyntaqliteFilterOver *fo = (SyntaqliteFilterOver*)synq_arena_ptr(&pCtx->ast, yymsp[0].minor.yy213);
    synq_mark_as_function(pCtx, yymsp[-8].minor.yy0);
    yylhsminor.yy213 = synq_parse_aggregate_function_call(pCtx,
        synq_span(pCtx, yymsp[-8].minor.yy0),
        (SyntaqliteAggregateFunctionCallFlags){.raw = (uint8_t)yymsp[-6].minor.yy213},
        yymsp[-5].minor.yy213,
        yymsp[-2].minor.yy213,
        fo->filter_expr,
        fo->over_def);
}
  yymsp[-8].minor.yy213 = yylhsminor.yy213;
        break;
      case 9: /* expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist RP WITHIN GROUP LP ORDER BY expr RP */
{
    synq_mark_as_function(pCtx, yymsp[-11].minor.yy0);
    yylhsminor.yy213 = synq_parse_ordered_set_function_call(pCtx,
        synq_span(pCtx, yymsp[-11].minor.yy0),
        (SyntaqliteAggregateFunctionCallFlags){.raw = (uint8_t)yymsp[-9].minor.yy213},
        yymsp[-8].minor.yy213,
        yymsp[-1].minor.yy213,
        SYNTAQLITE_NULL_NODE,
        SYNTAQLITE_NULL_NODE);
}
  yymsp[-11].minor.yy213 = yylhsminor.yy213;
        break;
      case 10: /* expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist RP WITHIN GROUP LP ORDER BY expr RP filter_over */
{
    SyntaqliteFilterOver *fo = (SyntaqliteFilterOver*)synq_arena_ptr(&pCtx->ast, yymsp[0].minor.yy213);
    synq_mark_as_function(pCtx, yymsp[-12].minor.yy0);
    yylhsminor.yy213 = synq_parse_ordered_set_function_call(pCtx,
        synq_span(pCtx, yymsp[-12].minor.yy0),
        (SyntaqliteAggregateFunctionCallFlags){.raw = (uint8_t)yymsp[-10].minor.yy213},
        yymsp[-9].minor.yy213,
        yymsp[-2].minor.yy213,
        fo->filter_expr,
        fo->over_def);
}
  yymsp[-12].minor.yy213 = yylhsminor.yy213;
        break;
      case 11: /* expr ::= CAST LP expr AS typetoken RP */
{
    yymsp[-5].minor.yy213 = synq_parse_cast_expr(pCtx, yymsp[-3].minor.yy213, synq_span(pCtx, yymsp[-1].minor.yy0));
}
        break;
      case 12: /* typetoken ::= */
{
    yymsp[1].minor.yy0.n = 0; yymsp[1].minor.yy0.z = 0;
}
        break;
      case 13: /* typetoken ::= typename */
{
    (void)yymsp[0].minor.yy0;
}
        break;
      case 14: /* typetoken ::= typename LP signed RP */
{
    yymsp[-3].minor.yy0.n = (int)(&yymsp[0].minor.yy0.z[yymsp[0].minor.yy0.n] - yymsp[-3].minor.yy0.z);
}
        break;
      case 15: /* typetoken ::= typename LP signed COMMA signed RP */
{
    yymsp[-5].minor.yy0.n = (int)(&yymsp[0].minor.yy0.z[yymsp[0].minor.yy0.n] - yymsp[-5].minor.yy0.z);
}
        break;
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
    yymsp[-1].minor.yy0.n = yymsp[0].minor.yy0.n + (int)(yymsp[0].minor.yy0.z - yymsp[-1].minor.yy0.z);
}
        break;
      case 18: /* selcollist ::= sclp scanpt nm DOT STAR */
{
    uint32_t col = synq_parse_result_column(pCtx, (SyntaqliteResultColumnFlags){.bits = {.star = 1}}, synq_span(pCtx, yymsp[-2].minor.yy0), SYNTAQLITE_NULL_NODE);
    yylhsminor.yy213 = synq_parse_result_column_list(pCtx, yymsp[-4].minor.yy213, col);
}
  yymsp[-4].minor.yy213 = yylhsminor.yy213;
        break;
      case 19: /* expr ::= ID|INDEXED|JOIN_KW */
{
    synq_mark_as_id(pCtx, yymsp[0].minor.yy0);
    yylhsminor.yy213 = synq_parse_column_ref(pCtx,
        synq_span(pCtx, yymsp[0].minor.yy0),
        SYNQ_NO_SPAN,
        SYNQ_NO_SPAN);
}
  yymsp[0].minor.yy213 = yylhsminor.yy213;
        break;
      case 20: /* expr ::= nm DOT nm */
{
    yylhsminor.yy213 = synq_parse_column_ref(pCtx,
        synq_span(pCtx, yymsp[0].minor.yy0),
        synq_span(pCtx, yymsp[-2].minor.yy0),
        SYNQ_NO_SPAN);
}
  yymsp[-2].minor.yy213 = yylhsminor.yy213;
        break;
      case 21: /* expr ::= nm DOT nm DOT nm */
{
    yylhsminor.yy213 = synq_parse_column_ref(pCtx,
        synq_span(pCtx, yymsp[0].minor.yy0),
        synq_span(pCtx, yymsp[-2].minor.yy0),
        synq_span(pCtx, yymsp[-4].minor.yy0));
}
  yymsp[-4].minor.yy213 = yylhsminor.yy213;
        break;
      case 22: /* selectnowith ::= selectnowith multiselect_op oneselect */
{
    yymsp[-2].minor.yy213 = synq_parse_compound_select(pCtx, (SyntaqliteCompoundOp)yymsp[-1].minor.yy220, yymsp[-2].minor.yy213, yymsp[0].minor.yy213);
}
        break;
      case 23: /* multiselect_op ::= UNION */
{ yylhsminor.yy220 = 0; (void)yymsp[0].minor.yy0; }
  yymsp[0].minor.yy220 = yylhsminor.yy220;
        break;
      case 24: /* multiselect_op ::= UNION ALL */
      case 29: /* in_op ::= NOT IN */ yytestcase(yyruleno==29);
{ yymsp[-1].minor.yy220 = 1; }
        break;
      case 25: /* multiselect_op ::= EXCEPT|INTERSECT */
{
    yylhsminor.yy220 = (yymsp[0].minor.yy0.type == SYNTAQLITE_TK_INTERSECT) ? 2 : 3;
}
  yymsp[0].minor.yy220 = yylhsminor.yy220;
        break;
      case 26: /* expr ::= LP select RP */
{
    pCtx->saw_subquery = 1;
    yymsp[-2].minor.yy213 = synq_parse_subquery_expr(pCtx, yymsp[-1].minor.yy213);
}
        break;
      case 27: /* expr ::= EXISTS LP select RP */
{
    pCtx->saw_subquery = 1;
    yymsp[-3].minor.yy213 = synq_parse_exists_expr(pCtx, yymsp[-1].minor.yy213);
}
        break;
      case 28: /* in_op ::= IN */
{ yymsp[0].minor.yy220 = 0; }
        break;
      case 30: /* expr ::= expr in_op LP exprlist RP */
{
    yymsp[-4].minor.yy213 = synq_parse_in_expr(pCtx, (SyntaqliteBool)yymsp[-3].minor.yy220, yymsp[-4].minor.yy213, yymsp[-1].minor.yy213);
}
        break;
      case 31: /* expr ::= expr in_op LP select RP */
{
    pCtx->saw_subquery = 1;
    uint32_t sub = synq_parse_subquery_expr(pCtx, yymsp[-1].minor.yy213);
    yymsp[-4].minor.yy213 = synq_parse_in_expr(pCtx, (SyntaqliteBool)yymsp[-3].minor.yy220, yymsp[-4].minor.yy213, sub);
}
        break;
      case 32: /* expr ::= expr in_op nm dbnm paren_exprlist */
{
    // Table-valued function IN expression - stub for now
    (void)yymsp[-2].minor.yy0; (void)yymsp[-1].minor.yy0; (void)yymsp[0].minor.yy213;
    yymsp[-4].minor.yy213 = synq_parse_in_expr(pCtx, (SyntaqliteBool)yymsp[-3].minor.yy220, yymsp[-4].minor.yy213, SYNTAQLITE_NULL_NODE);
}
        break;
      case 33: /* dbnm ::= */
{ yymsp[1].minor.yy0.z = NULL; yymsp[1].minor.yy0.n = 0; }
        break;
      case 34: /* dbnm ::= DOT nm */
{ yymsp[-1].minor.yy0 = yymsp[0].minor.yy0; }
        break;
      case 35: /* paren_exprlist ::= */
{ yymsp[1].minor.yy213 = SYNTAQLITE_NULL_NODE; }
        break;
      case 36: /* paren_exprlist ::= LP exprlist RP */
{ yymsp[-2].minor.yy213 = yymsp[-1].minor.yy213; }
        break;
      case 37: /* expr ::= expr ISNULL|NOTNULL */
{
    SyntaqliteIsOp op = (yymsp[0].minor.yy0.type == SYNTAQLITE_TK_ISNULL) ? SYNTAQLITE_IS_OP_IS_NULL : SYNTAQLITE_IS_OP_NOT_NULL;
    yylhsminor.yy213 = synq_parse_is_expr(pCtx, op, yymsp[-1].minor.yy213, SYNTAQLITE_NULL_NODE);
}
  yymsp[-1].minor.yy213 = yylhsminor.yy213;
        break;
      case 38: /* expr ::= expr NOT NULL */
{
    yylhsminor.yy213 = synq_parse_is_expr(pCtx, SYNTAQLITE_IS_OP_NOT_NULL, yymsp[-2].minor.yy213, SYNTAQLITE_NULL_NODE);
}
  yymsp[-2].minor.yy213 = yylhsminor.yy213;
        break;
      case 39: /* expr ::= expr IS expr */
{
    yylhsminor.yy213 = synq_parse_is_expr(pCtx, SYNTAQLITE_IS_OP_IS, yymsp[-2].minor.yy213, yymsp[0].minor.yy213);
}
  yymsp[-2].minor.yy213 = yylhsminor.yy213;
        break;
      case 40: /* expr ::= expr IS NOT expr */
{
    yylhsminor.yy213 = synq_parse_is_expr(pCtx, SYNTAQLITE_IS_OP_IS_NOT, yymsp[-3].minor.yy213, yymsp[0].minor.yy213);
}
  yymsp[-3].minor.yy213 = yylhsminor.yy213;
        break;
      case 41: /* expr ::= expr IS NOT DISTINCT FROM expr */
{
    yylhsminor.yy213 = synq_parse_is_expr(pCtx, SYNTAQLITE_IS_OP_IS_NOT_DISTINCT, yymsp[-5].minor.yy213, yymsp[0].minor.yy213);
}
  yymsp[-5].minor.yy213 = yylhsminor.yy213;
        break;
      case 42: /* expr ::= expr IS DISTINCT FROM expr */
{
    yylhsminor.yy213 = synq_parse_is_expr(pCtx, SYNTAQLITE_IS_OP_IS_DISTINCT, yymsp[-4].minor.yy213, yymsp[0].minor.yy213);
}
  yymsp[-4].minor.yy213 = yylhsminor.yy213;
        break;
      case 43: /* between_op ::= BETWEEN */
      case 211: /* sortorder ::= ASC */ yytestcase(yyruleno==211);
      case 267: /* distinct ::= ALL */ yytestcase(yyruleno==267);
{
    yymsp[0].minor.yy213 = 0;
}
        break;
      case 44: /* between_op ::= NOT BETWEEN */
      case 214: /* nulls ::= NULLS FIRST */ yytestcase(yyruleno==214);
{
    yymsp[-1].minor.yy213 = 1;
}
        break;
      case 45: /* expr ::= expr between_op expr AND expr */
{
    yylhsminor.yy213 = synq_parse_between_expr(pCtx, (SyntaqliteBool)yymsp[-3].minor.yy213, yymsp[-4].minor.yy213, yymsp[-2].minor.yy213, yymsp[0].minor.yy213);
}
  yymsp[-4].minor.yy213 = yylhsminor.yy213;
        break;
      case 46: /* likeop ::= LIKE_KW|MATCH */
      case 201: /* nm ::= STRING */ yytestcase(yyruleno==201);
      case 264: /* as ::= ID|STRING */ yytestcase(yyruleno==264);
{
    yylhsminor.yy0 = yymsp[0].minor.yy0;
}
  yymsp[0].minor.yy0 = yylhsminor.yy0;
        break;
      case 47: /* likeop ::= NOT LIKE_KW|MATCH */
{
    yymsp[-1].minor.yy0 = yymsp[0].minor.yy0;
    yymsp[-1].minor.yy0.n |= 0x80000000;
}
        break;
      case 48: /* expr ::= expr likeop expr */
{
    SyntaqliteBool negated = (yymsp[-1].minor.yy0.n & 0x80000000) ? SYNTAQLITE_BOOL_TRUE : SYNTAQLITE_BOOL_FALSE;
    yylhsminor.yy213 = synq_parse_like_expr(pCtx, negated, yymsp[-2].minor.yy213, yymsp[0].minor.yy213, SYNTAQLITE_NULL_NODE);
}
  yymsp[-2].minor.yy213 = yylhsminor.yy213;
        break;
      case 49: /* expr ::= expr likeop expr ESCAPE expr */
{
    SyntaqliteBool negated = (yymsp[-3].minor.yy0.n & 0x80000000) ? SYNTAQLITE_BOOL_TRUE : SYNTAQLITE_BOOL_FALSE;
    yylhsminor.yy213 = synq_parse_like_expr(pCtx, negated, yymsp[-4].minor.yy213, yymsp[-2].minor.yy213, yymsp[0].minor.yy213);
}
  yymsp[-4].minor.yy213 = yylhsminor.yy213;
        break;
      case 50: /* expr ::= CASE case_operand case_exprlist case_else END */
{
    yymsp[-4].minor.yy213 = synq_parse_case_expr(pCtx, yymsp[-3].minor.yy213, yymsp[-1].minor.yy213, yymsp[-2].minor.yy213);
}
        break;
      case 51: /* case_exprlist ::= case_exprlist WHEN expr THEN expr */
{
    uint32_t w = synq_parse_case_when(pCtx, yymsp[-2].minor.yy213, yymsp[0].minor.yy213);
    yylhsminor.yy213 = synq_parse_case_when_list(pCtx, yymsp[-4].minor.yy213, w);
}
  yymsp[-4].minor.yy213 = yylhsminor.yy213;
        break;
      case 52: /* case_exprlist ::= WHEN expr THEN expr */
{
    uint32_t w = synq_parse_case_when(pCtx, yymsp[-2].minor.yy213, yymsp[0].minor.yy213);
    yymsp[-3].minor.yy213 = synq_parse_case_when_list(pCtx, SYNTAQLITE_NULL_NODE, w);
}
        break;
      case 53: /* case_else ::= ELSE expr */
      case 158: /* where_opt_ret ::= WHERE expr */ yytestcase(yyruleno==158);
      case 270: /* from ::= FROM seltablist */ yytestcase(yyruleno==270);
      case 272: /* where_opt ::= WHERE expr */ yytestcase(yyruleno==272);
      case 276: /* having_opt ::= HAVING expr */ yytestcase(yyruleno==276);
      case 312: /* when_clause ::= WHEN expr */ yytestcase(yyruleno==312);
      case 348: /* key_opt ::= KEY expr */ yytestcase(yyruleno==348);
      case 351: /* vinto ::= INTO expr */ yytestcase(yyruleno==351);
      case 407: /* window_clause ::= WINDOW windowdefn_list */ yytestcase(yyruleno==407);
{
    yymsp[-1].minor.yy213 = yymsp[0].minor.yy213;
}
        break;
      case 54: /* case_else ::= */
      case 56: /* case_operand ::= */ yytestcase(yyruleno==56);
      case 106: /* conslist_opt ::= */ yytestcase(yyruleno==106);
      case 131: /* eidlist_opt ::= */ yytestcase(yyruleno==131);
      case 157: /* where_opt_ret ::= */ yytestcase(yyruleno==157);
      case 165: /* idlist_opt ::= */ yytestcase(yyruleno==165);
      case 167: /* upsert ::= */ yytestcase(yyruleno==167);
      case 191: /* exprlist ::= */ yytestcase(yyruleno==191);
      case 261: /* sclp ::= */ yytestcase(yyruleno==261);
      case 269: /* from ::= */ yytestcase(yyruleno==269);
      case 271: /* where_opt ::= */ yytestcase(yyruleno==271);
      case 273: /* groupby_opt ::= */ yytestcase(yyruleno==273);
      case 275: /* having_opt ::= */ yytestcase(yyruleno==275);
      case 277: /* orderby_opt ::= */ yytestcase(yyruleno==277);
      case 279: /* limit_opt ::= */ yytestcase(yyruleno==279);
      case 284: /* stl_prefix ::= */ yytestcase(yyruleno==284);
      case 311: /* when_clause ::= */ yytestcase(yyruleno==311);
      case 347: /* key_opt ::= */ yytestcase(yyruleno==347);
      case 352: /* vinto ::= */ yytestcase(yyruleno==352);
      case 392: /* frame_opt ::= */ yytestcase(yyruleno==392);
{
    yymsp[1].minor.yy213 = SYNTAQLITE_NULL_NODE;
}
        break;
      case 57: /* cmd ::= create_table create_table_args */
{
    // yymsp[0].minor.yy213 is either: (1) a CreateTableStmt node with columns/constraints filled in
    // or: (2) a CreateTableStmt node with as_select filled in
    // yymsp[-1].minor.yy213 has the table name/schema/temp/ifnotexists info packed as a node.
    // We need to merge yymsp[-1].minor.yy213 info into yymsp[0].minor.yy213.
    SyntaqliteNode *ct_node = AST_NODE(&pCtx->ast, yymsp[-1].minor.yy213);
    SyntaqliteNode *args_node = AST_NODE(&pCtx->ast, yymsp[0].minor.yy213);
    args_node->create_table_stmt.table_name = ct_node->create_table_stmt.table_name;
    args_node->create_table_stmt.schema = ct_node->create_table_stmt.schema;
    args_node->create_table_stmt.is_temp = ct_node->create_table_stmt.is_temp;
    args_node->create_table_stmt.if_not_exists = ct_node->create_table_stmt.if_not_exists;
    yylhsminor.yy213 = yymsp[0].minor.yy213;
}
  yymsp[-1].minor.yy213 = yylhsminor.yy213;
        break;
      case 58: /* create_table ::= createkw temp TABLE ifnotexists nm dbnm */
{
    SyntaqliteSourceSpan tbl_name = yymsp[0].minor.yy0.z ? synq_span(pCtx, yymsp[0].minor.yy0) : synq_span(pCtx, yymsp[-1].minor.yy0);
    SyntaqliteSourceSpan tbl_schema = yymsp[0].minor.yy0.z ? synq_span(pCtx, yymsp[-1].minor.yy0) : SYNQ_NO_SPAN;
    yymsp[-5].minor.yy213 = synq_parse_create_table_stmt(pCtx,
        tbl_name, tbl_schema, (SyntaqliteBool)yymsp[-4].minor.yy220, (SyntaqliteBool)yymsp[-2].minor.yy220,
        (SyntaqliteCreateTableStmtFlags){.raw = 0}, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE);
}
        break;
      case 59: /* create_table_args ::= LP columnlist conslist_opt RP table_option_set */
{
    yymsp[-4].minor.yy213 = synq_parse_create_table_stmt(pCtx,
        SYNQ_NO_SPAN, SYNQ_NO_SPAN, SYNTAQLITE_BOOL_FALSE, SYNTAQLITE_BOOL_FALSE,
        (SyntaqliteCreateTableStmtFlags){.raw = (uint8_t)yymsp[0].minor.yy220}, yymsp[-3].minor.yy213, yymsp[-2].minor.yy213, SYNTAQLITE_NULL_NODE);
}
        break;
      case 60: /* create_table_args ::= AS select */
{
    yymsp[-1].minor.yy213 = synq_parse_create_table_stmt(pCtx,
        SYNQ_NO_SPAN, SYNQ_NO_SPAN, SYNTAQLITE_BOOL_FALSE, SYNTAQLITE_BOOL_FALSE,
        (SyntaqliteCreateTableStmtFlags){.raw = 0}, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, yymsp[0].minor.yy213);
}
        break;
      case 61: /* table_option_set ::= */
      case 88: /* autoinc ::= */ yytestcase(yyruleno==88);
      case 103: /* init_deferred_pred_opt ::= */ yytestcase(yyruleno==103);
      case 117: /* defer_subclause_opt ::= */ yytestcase(yyruleno==117);
      case 135: /* collate ::= */ yytestcase(yyruleno==135);
      case 225: /* ifexists ::= */ yytestcase(yyruleno==225);
      case 235: /* kwcolumn_opt ::= */ yytestcase(yyruleno==235);
      case 245: /* trans_opt ::= */ yytestcase(yyruleno==245);
      case 249: /* savepoint_opt ::= */ yytestcase(yyruleno==249);
      case 358: /* uniqueflag ::= */ yytestcase(yyruleno==358);
      case 359: /* ifnotexists ::= */ yytestcase(yyruleno==359);
      case 364: /* temp ::= */ yytestcase(yyruleno==364);
{
    yymsp[1].minor.yy220 = 0;
}
        break;
      case 62: /* table_option_set ::= table_option */
      case 118: /* defer_subclause_opt ::= defer_subclause */ yytestcase(yyruleno==118);
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
    if (yymsp[0].minor.yy0.n == 5 && strncasecmp(yymsp[0].minor.yy0.z, "rowid", 5) == 0) {
        yymsp[-1].minor.yy220 = 1;
    } else {
        yymsp[-1].minor.yy220 = 0;
    }
}
        break;
      case 65: /* table_option ::= nm */
{
    // STRICT = bit 1
    if (yymsp[0].minor.yy0.n == 6 && strncasecmp(yymsp[0].minor.yy0.z, "strict", 6) == 0) {
        yylhsminor.yy220 = 2;
    } else {
        yylhsminor.yy220 = 0;
    }
}
  yymsp[0].minor.yy220 = yylhsminor.yy220;
        break;
      case 66: /* columnlist ::= columnlist COMMA columnname carglist */
{
    uint32_t col = synq_parse_column_def(pCtx, yymsp[-1].minor.yy400.name, yymsp[-1].minor.yy400.typetoken, yymsp[0].minor.yy10.list);
    yylhsminor.yy213 = synq_parse_column_def_list(pCtx, yymsp[-3].minor.yy213, col);
}
  yymsp[-3].minor.yy213 = yylhsminor.yy213;
        break;
      case 67: /* columnlist ::= columnname carglist */
{
    uint32_t col = synq_parse_column_def(pCtx, yymsp[-1].minor.yy400.name, yymsp[-1].minor.yy400.typetoken, yymsp[0].minor.yy10.list);
    yylhsminor.yy213 = synq_parse_column_def_list(pCtx, SYNTAQLITE_NULL_NODE, col);
}
  yymsp[-1].minor.yy213 = yylhsminor.yy213;
        break;
      case 68: /* carglist ::= carglist ccons */
{
    if (yymsp[0].minor.yy34.node != SYNTAQLITE_NULL_NODE) {
        // Apply pending constraint name from the list to this node
        SyntaqliteNode *node = AST_NODE(&pCtx->ast, yymsp[0].minor.yy34.node);
        node->column_constraint.constraint_name = yymsp[-1].minor.yy10.pending_name;
        if (yymsp[-1].minor.yy10.list == SYNTAQLITE_NULL_NODE) {
            yylhsminor.yy10.list = synq_parse_column_constraint_list(pCtx, SYNTAQLITE_NULL_NODE, yymsp[0].minor.yy34.node);
        } else {
            yylhsminor.yy10.list = synq_parse_column_constraint_list(pCtx, yymsp[-1].minor.yy10.list, yymsp[0].minor.yy34.node);
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
}
        break;
      case 70: /* ccons ::= CONSTRAINT nm */
      case 112: /* tcons ::= CONSTRAINT nm */ yytestcase(yyruleno==112);
{
    yymsp[-1].minor.yy34.node = SYNTAQLITE_NULL_NODE;
    yymsp[-1].minor.yy34.pending_name = synq_span(pCtx, yymsp[0].minor.yy0);
}
        break;
      case 71: /* ccons ::= DEFAULT scantok term */
{
    yymsp[-2].minor.yy34.node = synq_parse_column_constraint(pCtx,
        SYNTAQLITE_COLUMN_CONSTRAINT_TYPE_DEFAULT,
        SYNQ_NO_SPAN,
        SYNTAQLITE_CONFLICT_ACTION_DEFAULT, SYNTAQLITE_SORT_ORDER_ASC, SYNTAQLITE_BOOL_FALSE,
        SYNQ_NO_SPAN,
        SYNTAQLITE_GENERATED_COLUMN_STORAGE_VIRTUAL,
        yymsp[0].minor.yy213, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE);
    yymsp[-2].minor.yy34.pending_name = SYNQ_NO_SPAN;
}
        break;
      case 72: /* ccons ::= DEFAULT LP expr RP */
{
    yymsp[-3].minor.yy34.node = synq_parse_column_constraint(pCtx,
        SYNTAQLITE_COLUMN_CONSTRAINT_TYPE_DEFAULT,
        SYNQ_NO_SPAN,
        SYNTAQLITE_CONFLICT_ACTION_DEFAULT, SYNTAQLITE_SORT_ORDER_ASC, SYNTAQLITE_BOOL_FALSE,
        SYNQ_NO_SPAN,
        SYNTAQLITE_GENERATED_COLUMN_STORAGE_VIRTUAL,
        yymsp[-1].minor.yy213, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE);
    yymsp[-3].minor.yy34.pending_name = SYNQ_NO_SPAN;
}
        break;
      case 73: /* ccons ::= DEFAULT PLUS scantok term */
{
    yymsp[-3].minor.yy34.node = synq_parse_column_constraint(pCtx,
        SYNTAQLITE_COLUMN_CONSTRAINT_TYPE_DEFAULT,
        SYNQ_NO_SPAN,
        SYNTAQLITE_CONFLICT_ACTION_DEFAULT, SYNTAQLITE_SORT_ORDER_ASC, SYNTAQLITE_BOOL_FALSE,
        SYNQ_NO_SPAN,
        SYNTAQLITE_GENERATED_COLUMN_STORAGE_VIRTUAL,
        yymsp[0].minor.yy213, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE);
    yymsp[-3].minor.yy34.pending_name = SYNQ_NO_SPAN;
}
        break;
      case 74: /* ccons ::= DEFAULT MINUS scantok term */
{
    // Create a unary minus wrapping the term
    uint32_t neg = synq_parse_unary_expr(pCtx, SYNTAQLITE_UNARY_OP_MINUS, yymsp[0].minor.yy213);
    yymsp[-3].minor.yy34.node = synq_parse_column_constraint(pCtx,
        SYNTAQLITE_COLUMN_CONSTRAINT_TYPE_DEFAULT,
        SYNQ_NO_SPAN,
        SYNTAQLITE_CONFLICT_ACTION_DEFAULT, SYNTAQLITE_SORT_ORDER_ASC, SYNTAQLITE_BOOL_FALSE,
        SYNQ_NO_SPAN,
        SYNTAQLITE_GENERATED_COLUMN_STORAGE_VIRTUAL,
        neg, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE);
    yymsp[-3].minor.yy34.pending_name = SYNQ_NO_SPAN;
}
        break;
      case 75: /* ccons ::= DEFAULT scantok ID|INDEXED */
{
    // Treat the identifier as a literal expression
    uint32_t lit = synq_parse_literal(pCtx,
        SYNTAQLITE_LITERAL_TYPE_STRING, synq_span(pCtx, yymsp[0].minor.yy0));
    yymsp[-2].minor.yy34.node = synq_parse_column_constraint(pCtx,
        SYNTAQLITE_COLUMN_CONSTRAINT_TYPE_DEFAULT,
        SYNQ_NO_SPAN,
        SYNTAQLITE_CONFLICT_ACTION_DEFAULT, SYNTAQLITE_SORT_ORDER_ASC, SYNTAQLITE_BOOL_FALSE,
        SYNQ_NO_SPAN,
        SYNTAQLITE_GENERATED_COLUMN_STORAGE_VIRTUAL,
        lit, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE);
    yymsp[-2].minor.yy34.pending_name = SYNQ_NO_SPAN;
}
        break;
      case 76: /* ccons ::= NULL onconf */
{
    yymsp[-1].minor.yy34.node = synq_parse_column_constraint(pCtx,
        SYNTAQLITE_COLUMN_CONSTRAINT_TYPE_NULL,
        SYNQ_NO_SPAN,
        (SyntaqliteConflictAction)yymsp[0].minor.yy220, SYNTAQLITE_SORT_ORDER_ASC, SYNTAQLITE_BOOL_FALSE,
        SYNQ_NO_SPAN,
        SYNTAQLITE_GENERATED_COLUMN_STORAGE_VIRTUAL,
        SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE);
    yymsp[-1].minor.yy34.pending_name = SYNQ_NO_SPAN;
}
        break;
      case 77: /* ccons ::= NOT NULL onconf */
{
    yymsp[-2].minor.yy34.node = synq_parse_column_constraint(pCtx,
        SYNTAQLITE_COLUMN_CONSTRAINT_TYPE_NOT_NULL,
        SYNQ_NO_SPAN,
        (SyntaqliteConflictAction)yymsp[0].minor.yy220, SYNTAQLITE_SORT_ORDER_ASC, SYNTAQLITE_BOOL_FALSE,
        SYNQ_NO_SPAN,
        SYNTAQLITE_GENERATED_COLUMN_STORAGE_VIRTUAL,
        SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE);
    yymsp[-2].minor.yy34.pending_name = SYNQ_NO_SPAN;
}
        break;
      case 78: /* ccons ::= PRIMARY KEY sortorder onconf autoinc */
{
    yymsp[-4].minor.yy34.node = synq_parse_column_constraint(pCtx,
        SYNTAQLITE_COLUMN_CONSTRAINT_TYPE_PRIMARY_KEY,
        SYNQ_NO_SPAN,
        (SyntaqliteConflictAction)yymsp[-1].minor.yy220, (SyntaqliteSortOrder)yymsp[-2].minor.yy213, (SyntaqliteBool)yymsp[0].minor.yy220,
        SYNQ_NO_SPAN,
        SYNTAQLITE_GENERATED_COLUMN_STORAGE_VIRTUAL,
        SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE);
    yymsp[-4].minor.yy34.pending_name = SYNQ_NO_SPAN;
}
        break;
      case 79: /* ccons ::= UNIQUE onconf */
{
    yymsp[-1].minor.yy34.node = synq_parse_column_constraint(pCtx,
        SYNTAQLITE_COLUMN_CONSTRAINT_TYPE_UNIQUE,
        SYNQ_NO_SPAN,
        (SyntaqliteConflictAction)yymsp[0].minor.yy220, SYNTAQLITE_SORT_ORDER_ASC, SYNTAQLITE_BOOL_FALSE,
        SYNQ_NO_SPAN,
        SYNTAQLITE_GENERATED_COLUMN_STORAGE_VIRTUAL,
        SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE);
    yymsp[-1].minor.yy34.pending_name = SYNQ_NO_SPAN;
}
        break;
      case 80: /* ccons ::= CHECK LP expr RP */
{
    yymsp[-3].minor.yy34.node = synq_parse_column_constraint(pCtx,
        SYNTAQLITE_COLUMN_CONSTRAINT_TYPE_CHECK,
        SYNQ_NO_SPAN,
        SYNTAQLITE_CONFLICT_ACTION_DEFAULT, SYNTAQLITE_SORT_ORDER_ASC, SYNTAQLITE_BOOL_FALSE,
        SYNQ_NO_SPAN,
        SYNTAQLITE_GENERATED_COLUMN_STORAGE_VIRTUAL,
        SYNTAQLITE_NULL_NODE, yymsp[-1].minor.yy213, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE);
    yymsp[-3].minor.yy34.pending_name = SYNQ_NO_SPAN;
}
        break;
      case 81: /* ccons ::= REFERENCES nm eidlist_opt refargs */
{
    // Decode refargs: low byte = on_delete, next byte = on_update
    SyntaqliteForeignKeyAction on_del = (SyntaqliteForeignKeyAction)(yymsp[0].minor.yy220 & 0xff);
    SyntaqliteForeignKeyAction on_upd = (SyntaqliteForeignKeyAction)((yymsp[0].minor.yy220 >> 8) & 0xff);
    uint32_t fk = synq_parse_foreign_key_clause(pCtx,
        synq_span(pCtx, yymsp[-2].minor.yy0), yymsp[-1].minor.yy213, on_del, on_upd, SYNTAQLITE_BOOL_FALSE);
    yymsp[-3].minor.yy34.node = synq_parse_column_constraint(pCtx,
        SYNTAQLITE_COLUMN_CONSTRAINT_TYPE_REFERENCES,
        SYNQ_NO_SPAN,
        SYNTAQLITE_CONFLICT_ACTION_DEFAULT, SYNTAQLITE_SORT_ORDER_ASC, SYNTAQLITE_BOOL_FALSE,
        SYNQ_NO_SPAN,
        SYNTAQLITE_GENERATED_COLUMN_STORAGE_VIRTUAL,
        SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, fk);
    yymsp[-3].minor.yy34.pending_name = SYNQ_NO_SPAN;
}
        break;
      case 82: /* ccons ::= defer_subclause */
{
    // Create a minimal constraint that just marks deferral.
    // In practice, this follows a REFERENCES ccons. We'll handle it
    // by updating the last constraint in the list if possible.
    // For simplicity, we create a separate REFERENCES constraint with just deferral info.
    // The printer will show it as a separate constraint entry.
    uint32_t fk = synq_parse_foreign_key_clause(pCtx,
        SYNQ_NO_SPAN, SYNTAQLITE_NULL_NODE,
        SYNTAQLITE_FOREIGN_KEY_ACTION_NO_ACTION,
        SYNTAQLITE_FOREIGN_KEY_ACTION_NO_ACTION,
        (SyntaqliteBool)yymsp[0].minor.yy220);
    yylhsminor.yy34.node = synq_parse_column_constraint(pCtx,
        SYNTAQLITE_COLUMN_CONSTRAINT_TYPE_REFERENCES,
        SYNQ_NO_SPAN,
        SYNTAQLITE_CONFLICT_ACTION_DEFAULT, SYNTAQLITE_SORT_ORDER_ASC, SYNTAQLITE_BOOL_FALSE,
        SYNQ_NO_SPAN,
        SYNTAQLITE_GENERATED_COLUMN_STORAGE_VIRTUAL,
        SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, fk);
    yylhsminor.yy34.pending_name = SYNQ_NO_SPAN;
}
  yymsp[0].minor.yy34 = yylhsminor.yy34;
        break;
      case 83: /* ccons ::= COLLATE ID|STRING */
{
    yymsp[-1].minor.yy34.node = synq_parse_column_constraint(pCtx,
        SYNTAQLITE_COLUMN_CONSTRAINT_TYPE_COLLATE,
        SYNQ_NO_SPAN,
        0, 0, 0,
        synq_span(pCtx, yymsp[0].minor.yy0),
        SYNTAQLITE_GENERATED_COLUMN_STORAGE_VIRTUAL,
        SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE);
    yymsp[-1].minor.yy34.pending_name = SYNQ_NO_SPAN;
}
        break;
      case 84: /* ccons ::= GENERATED ALWAYS AS generated */
{
    yymsp[-3].minor.yy34 = yymsp[0].minor.yy34;
}
        break;
      case 85: /* ccons ::= AS generated */
{
    yymsp[-1].minor.yy34 = yymsp[0].minor.yy34;
}
        break;
      case 86: /* generated ::= LP expr RP */
{
    yymsp[-2].minor.yy34.node = synq_parse_column_constraint(pCtx,
        SYNTAQLITE_COLUMN_CONSTRAINT_TYPE_GENERATED,
        SYNQ_NO_SPAN,
        SYNTAQLITE_CONFLICT_ACTION_DEFAULT, SYNTAQLITE_SORT_ORDER_ASC, SYNTAQLITE_BOOL_FALSE,
        SYNQ_NO_SPAN,
        SYNTAQLITE_GENERATED_COLUMN_STORAGE_VIRTUAL,
        SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, yymsp[-1].minor.yy213, SYNTAQLITE_NULL_NODE);
    yymsp[-2].minor.yy34.pending_name = SYNQ_NO_SPAN;
}
        break;
      case 87: /* generated ::= LP expr RP ID */
{
    SyntaqliteGeneratedColumnStorage storage = SYNTAQLITE_GENERATED_COLUMN_STORAGE_VIRTUAL;
    if (yymsp[0].minor.yy0.n == 6 && strncasecmp(yymsp[0].minor.yy0.z, "stored", 6) == 0) {
        storage = SYNTAQLITE_GENERATED_COLUMN_STORAGE_STORED;
    }
    yymsp[-3].minor.yy34.node = synq_parse_column_constraint(pCtx,
        SYNTAQLITE_COLUMN_CONSTRAINT_TYPE_GENERATED,
        SYNQ_NO_SPAN,
        SYNTAQLITE_CONFLICT_ACTION_DEFAULT, SYNTAQLITE_SORT_ORDER_ASC, SYNTAQLITE_BOOL_FALSE,
        SYNQ_NO_SPAN,
        storage,
        SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, yymsp[-2].minor.yy213, SYNTAQLITE_NULL_NODE);
    yymsp[-3].minor.yy34.pending_name = SYNQ_NO_SPAN;
}
        break;
      case 89: /* autoinc ::= AUTOINCR */
      case 236: /* kwcolumn_opt ::= COLUMNKW */ yytestcase(yyruleno==236);
      case 354: /* explain ::= EXPLAIN */ yytestcase(yyruleno==354);
      case 357: /* uniqueflag ::= UNIQUE */ yytestcase(yyruleno==357);
      case 363: /* temp ::= TEMP */ yytestcase(yyruleno==363);
{
    yymsp[0].minor.yy220 = 1;
}
        break;
      case 90: /* refargs ::= */
{
    yymsp[1].minor.yy220 = 0; // NO_ACTION for both
}
        break;
      case 91: /* refargs ::= refargs refarg */
{
    // refarg encodes: low byte = value, byte 1 = shift amount (0 or 8)
    int val = yymsp[0].minor.yy220 & 0xff;
    int shift = (yymsp[0].minor.yy220 >> 8) & 0xff;
    // Clear the target byte in yymsp[-1].minor.yy220 and set new value
    yymsp[-1].minor.yy220 = (yymsp[-1].minor.yy220 & ~(0xff << shift)) | (val << shift);
}
        break;
      case 92: /* refarg ::= MATCH nm */
{
    yymsp[-1].minor.yy220 = 0; // MATCH is ignored
}
        break;
      case 93: /* refarg ::= ON INSERT refact */
{
    yymsp[-2].minor.yy220 = 0; // ON INSERT is ignored
}
        break;
      case 94: /* refarg ::= ON DELETE refact */
{
    yymsp[-2].minor.yy220 = yymsp[0].minor.yy220; // shift=0 for DELETE
}
        break;
      case 95: /* refarg ::= ON UPDATE refact */
{
    yymsp[-2].minor.yy220 = yymsp[0].minor.yy220 | (8 << 8); // shift=8 for UPDATE
}
        break;
      case 96: /* refact ::= SET NULL */
{
    yymsp[-1].minor.yy220 = (int)SYNTAQLITE_FOREIGN_KEY_ACTION_SET_NULL;
}
        break;
      case 97: /* refact ::= SET DEFAULT */
{
    yymsp[-1].minor.yy220 = (int)SYNTAQLITE_FOREIGN_KEY_ACTION_SET_DEFAULT;
}
        break;
      case 98: /* refact ::= CASCADE */
{
    yymsp[0].minor.yy220 = (int)SYNTAQLITE_FOREIGN_KEY_ACTION_CASCADE;
}
        break;
      case 99: /* refact ::= RESTRICT */
{
    yymsp[0].minor.yy220 = (int)SYNTAQLITE_FOREIGN_KEY_ACTION_RESTRICT;
}
        break;
      case 100: /* refact ::= NO ACTION */
{
    yymsp[-1].minor.yy220 = (int)SYNTAQLITE_FOREIGN_KEY_ACTION_NO_ACTION;
}
        break;
      case 101: /* defer_subclause ::= NOT DEFERRABLE init_deferred_pred_opt */
{
    yymsp[-2].minor.yy220 = 0;
}
        break;
      case 102: /* defer_subclause ::= DEFERRABLE init_deferred_pred_opt */
      case 144: /* insert_cmd ::= INSERT orconf */ yytestcase(yyruleno==144);
      case 147: /* orconf ::= OR resolvetype */ yytestcase(yyruleno==147);
      case 403: /* frame_exclude_opt ::= EXCLUDE frame_exclude */ yytestcase(yyruleno==403);
{
    yymsp[-1].minor.yy220 = yymsp[0].minor.yy220;
}
        break;
      case 104: /* init_deferred_pred_opt ::= INITIALLY DEFERRED */
      case 136: /* collate ::= COLLATE ID|STRING */ yytestcase(yyruleno==136);
      case 224: /* ifexists ::= IF EXISTS */ yytestcase(yyruleno==224);
{
    yymsp[-1].minor.yy220 = 1;
}
        break;
      case 105: /* init_deferred_pred_opt ::= INITIALLY IMMEDIATE */
      case 247: /* trans_opt ::= TRANSACTION nm */ yytestcase(yyruleno==247);
{
    yymsp[-1].minor.yy220 = 0;
}
        break;
      case 107: /* conslist_opt ::= COMMA conslist */
{
    yymsp[-1].minor.yy213 = yymsp[0].minor.yy10.list;
}
        break;
      case 108: /* conslist ::= conslist tconscomma tcons */
{
    // If comma separator was present, clear pending constraint name
    SyntaqliteSourceSpan pending = yymsp[-1].minor.yy220 ? SYNQ_NO_SPAN : yymsp[-2].minor.yy10.pending_name;
    if (yymsp[0].minor.yy34.node != SYNTAQLITE_NULL_NODE) {
        SyntaqliteNode *node = AST_NODE(&pCtx->ast, yymsp[0].minor.yy34.node);
        node->table_constraint.constraint_name = pending;
        if (yymsp[-2].minor.yy10.list == SYNTAQLITE_NULL_NODE) {
            yylhsminor.yy10.list = synq_parse_table_constraint_list(pCtx, SYNTAQLITE_NULL_NODE, yymsp[0].minor.yy34.node);
        } else {
            yylhsminor.yy10.list = synq_parse_table_constraint_list(pCtx, yymsp[-2].minor.yy10.list, yymsp[0].minor.yy34.node);
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
        yylhsminor.yy10.list = synq_parse_table_constraint_list(pCtx, SYNTAQLITE_NULL_NODE, yymsp[0].minor.yy34.node);
        yylhsminor.yy10.pending_name = SYNQ_NO_SPAN;
    } else {
        yylhsminor.yy10.list = SYNTAQLITE_NULL_NODE;
        yylhsminor.yy10.pending_name = yymsp[0].minor.yy34.pending_name;
    }
}
  yymsp[0].minor.yy10 = yylhsminor.yy10;
        break;
      case 110: /* tconscomma ::= COMMA */
{ yymsp[0].minor.yy220 = 1; }
        break;
      case 111: /* tconscomma ::= */
{ yymsp[1].minor.yy220 = 0; }
        break;
      case 113: /* tcons ::= PRIMARY KEY LP sortlist autoinc RP onconf */
{
    yymsp[-6].minor.yy34.node = synq_parse_table_constraint(pCtx,
        SYNTAQLITE_TABLE_CONSTRAINT_TYPE_PRIMARY_KEY,
        SYNQ_NO_SPAN,
        (SyntaqliteConflictAction)yymsp[0].minor.yy220, (SyntaqliteBool)yymsp[-2].minor.yy220,
        yymsp[-3].minor.yy213, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE);
    yymsp[-6].minor.yy34.pending_name = SYNQ_NO_SPAN;
}
        break;
      case 114: /* tcons ::= UNIQUE LP sortlist RP onconf */
{
    yymsp[-4].minor.yy34.node = synq_parse_table_constraint(pCtx,
        SYNTAQLITE_TABLE_CONSTRAINT_TYPE_UNIQUE,
        SYNQ_NO_SPAN,
        (SyntaqliteConflictAction)yymsp[0].minor.yy220, SYNTAQLITE_BOOL_FALSE,
        yymsp[-2].minor.yy213, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE);
    yymsp[-4].minor.yy34.pending_name = SYNQ_NO_SPAN;
}
        break;
      case 115: /* tcons ::= CHECK LP expr RP onconf */
{
    yymsp[-4].minor.yy34.node = synq_parse_table_constraint(pCtx,
        SYNTAQLITE_TABLE_CONSTRAINT_TYPE_CHECK,
        SYNQ_NO_SPAN,
        (SyntaqliteConflictAction)yymsp[0].minor.yy220, SYNTAQLITE_BOOL_FALSE,
        SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE, yymsp[-2].minor.yy213, SYNTAQLITE_NULL_NODE);
    yymsp[-4].minor.yy34.pending_name = SYNQ_NO_SPAN;
}
        break;
      case 116: /* tcons ::= FOREIGN KEY LP eidlist RP REFERENCES nm eidlist_opt refargs defer_subclause_opt */
{
    SyntaqliteForeignKeyAction on_del = (SyntaqliteForeignKeyAction)(yymsp[-1].minor.yy220 & 0xff);
    SyntaqliteForeignKeyAction on_upd = (SyntaqliteForeignKeyAction)((yymsp[-1].minor.yy220 >> 8) & 0xff);
    uint32_t fk = synq_parse_foreign_key_clause(pCtx,
        synq_span(pCtx, yymsp[-3].minor.yy0), yymsp[-2].minor.yy213, on_del, on_upd, (SyntaqliteBool)yymsp[0].minor.yy220);
    yymsp[-9].minor.yy34.node = synq_parse_table_constraint(pCtx,
        SYNTAQLITE_TABLE_CONSTRAINT_TYPE_FOREIGN_KEY,
        SYNQ_NO_SPAN,
        SYNTAQLITE_CONFLICT_ACTION_DEFAULT, SYNTAQLITE_BOOL_FALSE,
        SYNTAQLITE_NULL_NODE, yymsp[-6].minor.yy213, SYNTAQLITE_NULL_NODE, fk);
    yymsp[-9].minor.yy34.pending_name = SYNQ_NO_SPAN;
}
        break;
      case 119: /* onconf ::= */
      case 146: /* orconf ::= */ yytestcase(yyruleno==146);
{
    yymsp[1].minor.yy220 = (int)SYNTAQLITE_CONFLICT_ACTION_DEFAULT;
}
        break;
      case 120: /* onconf ::= ON CONFLICT resolvetype */
{
    yymsp[-2].minor.yy220 = yymsp[0].minor.yy220;
}
        break;
      case 121: /* scantok ::= */
      case 155: /* indexed_opt ::= */ yytestcase(yyruleno==155);
      case 262: /* scanpt ::= */ yytestcase(yyruleno==262);
      case 265: /* as ::= */ yytestcase(yyruleno==265);
{
    yymsp[1].minor.yy0.z = NULL; yymsp[1].minor.yy0.n = 0;
}
        break;
      case 122: /* select ::= WITH wqlist selectnowith */
{
    yymsp[-2].minor.yy213 = synq_parse_with_clause(pCtx, 0, yymsp[-1].minor.yy213, yymsp[0].minor.yy213);
}
        break;
      case 123: /* select ::= WITH RECURSIVE wqlist selectnowith */
{
    yymsp[-3].minor.yy213 = synq_parse_with_clause(pCtx, 1, yymsp[-1].minor.yy213, yymsp[0].minor.yy213);
}
        break;
      case 124: /* wqitem ::= withnm eidlist_opt wqas LP select RP */
{
    yylhsminor.yy213 = synq_parse_cte_definition(pCtx, synq_span(pCtx, yymsp[-5].minor.yy0), (SyntaqliteMaterialized)yymsp[-3].minor.yy220, yymsp[-4].minor.yy213, yymsp[-1].minor.yy213);
}
  yymsp[-5].minor.yy213 = yylhsminor.yy213;
        break;
      case 125: /* wqlist ::= wqitem */
{
    yylhsminor.yy213 = synq_parse_cte_list(pCtx, SYNTAQLITE_NULL_NODE, yymsp[0].minor.yy213);
}
  yymsp[0].minor.yy213 = yylhsminor.yy213;
        break;
      case 126: /* wqlist ::= wqlist COMMA wqitem */
{
    yymsp[-2].minor.yy213 = synq_parse_cte_list(pCtx, yymsp[-2].minor.yy213, yymsp[0].minor.yy213);
}
        break;
      case 127: /* withnm ::= nm */
{
    // Token passthrough - nm already produces SynqParseToken
}
        break;
      case 128: /* wqas ::= AS */
{
    yymsp[0].minor.yy220 = (int)SYNTAQLITE_MATERIALIZED_DEFAULT;
}
        break;
      case 129: /* wqas ::= AS MATERIALIZED */
{
    yymsp[-1].minor.yy220 = (int)SYNTAQLITE_MATERIALIZED_MATERIALIZED;
}
        break;
      case 130: /* wqas ::= AS NOT MATERIALIZED */
{
    yymsp[-2].minor.yy220 = (int)SYNTAQLITE_MATERIALIZED_NOT_MATERIALIZED;
}
        break;
      case 132: /* eidlist_opt ::= LP eidlist RP */
      case 166: /* idlist_opt ::= LP idlist RP */ yytestcase(yyruleno==166);
      case 177: /* expr ::= LP expr RP */ yytestcase(yyruleno==177);
      case 323: /* trigger_cmd ::= scanpt select scanpt */ yytestcase(yyruleno==323);
{
    yymsp[-2].minor.yy213 = yymsp[-1].minor.yy213;
}
        break;
      case 133: /* eidlist ::= nm collate sortorder */
{
    (void)yymsp[-1].minor.yy220; (void)yymsp[0].minor.yy213;
    uint32_t col = synq_parse_column_ref(pCtx,
        synq_span(pCtx, yymsp[-2].minor.yy0),
        SYNQ_NO_SPAN,
        SYNQ_NO_SPAN);
    yylhsminor.yy213 = synq_parse_expr_list(pCtx, SYNTAQLITE_NULL_NODE, col);
}
  yymsp[-2].minor.yy213 = yylhsminor.yy213;
        break;
      case 134: /* eidlist ::= eidlist COMMA nm collate sortorder */
{
    (void)yymsp[-1].minor.yy220; (void)yymsp[0].minor.yy213;
    uint32_t col = synq_parse_column_ref(pCtx,
        synq_span(pCtx, yymsp[-2].minor.yy0),
        SYNQ_NO_SPAN,
        SYNQ_NO_SPAN);
    yymsp[-4].minor.yy213 = synq_parse_expr_list(pCtx, yymsp[-4].minor.yy213, col);
}
        break;
      case 137: /* with ::= */
{
    yymsp[1].minor.yy465.cte_list = SYNTAQLITE_NULL_NODE;
    yymsp[1].minor.yy465.is_recursive = 0;
}
        break;
      case 138: /* with ::= WITH wqlist */
{
    yymsp[-1].minor.yy465.cte_list = yymsp[0].minor.yy213;
    yymsp[-1].minor.yy465.is_recursive = 0;
}
        break;
      case 139: /* with ::= WITH RECURSIVE wqlist */
{
    yymsp[-2].minor.yy465.cte_list = yymsp[0].minor.yy213;
    yymsp[-2].minor.yy465.is_recursive = 1;
}
        break;
      case 140: /* cmd ::= with DELETE FROM xfullname indexed_opt where_opt_ret orderby_opt limit_opt */
{
    (void)yymsp[-3].minor.yy0;
    if (yymsp[-1].minor.yy213 != SYNTAQLITE_NULL_NODE || yymsp[0].minor.yy213 != SYNTAQLITE_NULL_NODE) {
        pCtx->saw_update_delete_limit = 1;
        if (!SYNQ_HAS_CFLAG(pCtx->env, SYNQ_CFLAG_IDX_ENABLE_UPDATE_DELETE_LIMIT)) {
            pCtx->error = 1;
        }
    }
    uint32_t del = synq_parse_delete_stmt(pCtx, yymsp[-4].minor.yy213, yymsp[-2].minor.yy213, yymsp[-1].minor.yy213, yymsp[0].minor.yy213);
    if (yymsp[-7].minor.yy465.cte_list != SYNTAQLITE_NULL_NODE) {
        yylhsminor.yy213 = synq_parse_with_clause(pCtx, yymsp[-7].minor.yy465.is_recursive, yymsp[-7].minor.yy465.cte_list, del);
    } else {
        yylhsminor.yy213 = del;
    }
}
  yymsp[-7].minor.yy213 = yylhsminor.yy213;
        break;
      case 141: /* cmd ::= with UPDATE orconf xfullname indexed_opt SET setlist from where_opt_ret orderby_opt limit_opt */
{
    (void)yymsp[-6].minor.yy0;
    if (yymsp[-1].minor.yy213 != SYNTAQLITE_NULL_NODE || yymsp[0].minor.yy213 != SYNTAQLITE_NULL_NODE) {
        pCtx->saw_update_delete_limit = 1;
        if (!SYNQ_HAS_CFLAG(pCtx->env, SYNQ_CFLAG_IDX_ENABLE_UPDATE_DELETE_LIMIT)) {
            pCtx->error = 1;
        }
    }
    uint32_t upd = synq_parse_update_stmt(pCtx, (SyntaqliteConflictAction)yymsp[-8].minor.yy220, yymsp[-7].minor.yy213, yymsp[-4].minor.yy213, yymsp[-3].minor.yy213, yymsp[-2].minor.yy213, yymsp[-1].minor.yy213, yymsp[0].minor.yy213);
    if (yymsp[-10].minor.yy465.cte_list != SYNTAQLITE_NULL_NODE) {
        yylhsminor.yy213 = synq_parse_with_clause(pCtx, yymsp[-10].minor.yy465.is_recursive, yymsp[-10].minor.yy465.cte_list, upd);
    } else {
        yylhsminor.yy213 = upd;
    }
}
  yymsp[-10].minor.yy213 = yylhsminor.yy213;
        break;
      case 142: /* cmd ::= with insert_cmd INTO xfullname idlist_opt select upsert */
{
    (void)yymsp[0].minor.yy213;
    uint32_t ins = synq_parse_insert_stmt(pCtx, (SyntaqliteConflictAction)yymsp[-5].minor.yy220, yymsp[-3].minor.yy213, yymsp[-2].minor.yy213, yymsp[-1].minor.yy213);
    if (yymsp[-6].minor.yy465.cte_list != SYNTAQLITE_NULL_NODE) {
        yylhsminor.yy213 = synq_parse_with_clause(pCtx, yymsp[-6].minor.yy465.is_recursive, yymsp[-6].minor.yy465.cte_list, ins);
    } else {
        yylhsminor.yy213 = ins;
    }
}
  yymsp[-6].minor.yy213 = yylhsminor.yy213;
        break;
      case 143: /* cmd ::= with insert_cmd INTO xfullname idlist_opt DEFAULT VALUES returning */
{
    uint32_t ins = synq_parse_insert_stmt(pCtx, (SyntaqliteConflictAction)yymsp[-6].minor.yy220, yymsp[-4].minor.yy213, yymsp[-3].minor.yy213, SYNTAQLITE_NULL_NODE);
    if (yymsp[-7].minor.yy465.cte_list != SYNTAQLITE_NULL_NODE) {
        yylhsminor.yy213 = synq_parse_with_clause(pCtx, yymsp[-7].minor.yy465.is_recursive, yymsp[-7].minor.yy465.cte_list, ins);
    } else {
        yylhsminor.yy213 = ins;
    }
}
  yymsp[-7].minor.yy213 = yylhsminor.yy213;
        break;
      case 145: /* insert_cmd ::= REPLACE */
      case 150: /* resolvetype ::= REPLACE */ yytestcase(yyruleno==150);
{
    yymsp[0].minor.yy220 = (int)SYNTAQLITE_CONFLICT_ACTION_REPLACE;
}
        break;
      case 148: /* resolvetype ::= raisetype */
{
    // raisetype: ROLLBACK=1, ABORT=2, FAIL=3 (SynqRaiseType enum values)
    // ConflictAction: ROLLBACK=1, ABORT=2, FAIL=3 (same values, direct passthrough)
    yylhsminor.yy220 = yymsp[0].minor.yy220;
}
  yymsp[0].minor.yy220 = yylhsminor.yy220;
        break;
      case 149: /* resolvetype ::= IGNORE */
{
    yymsp[0].minor.yy220 = (int)SYNTAQLITE_CONFLICT_ACTION_IGNORE;
}
        break;
      case 151: /* xfullname ::= nm */
{
    yylhsminor.yy213 = synq_parse_table_ref(pCtx,
        synq_span(pCtx, yymsp[0].minor.yy0), SYNQ_NO_SPAN, SYNQ_NO_SPAN);
}
  yymsp[0].minor.yy213 = yylhsminor.yy213;
        break;
      case 152: /* xfullname ::= nm DOT nm */
{
    yylhsminor.yy213 = synq_parse_table_ref(pCtx,
        synq_span(pCtx, yymsp[0].minor.yy0), synq_span(pCtx, yymsp[-2].minor.yy0), SYNQ_NO_SPAN);
}
  yymsp[-2].minor.yy213 = yylhsminor.yy213;
        break;
      case 153: /* xfullname ::= nm DOT nm AS nm */
{
    yylhsminor.yy213 = synq_parse_table_ref(pCtx,
        synq_span(pCtx, yymsp[-2].minor.yy0), synq_span(pCtx, yymsp[-4].minor.yy0), synq_span(pCtx, yymsp[0].minor.yy0));
}
  yymsp[-4].minor.yy213 = yylhsminor.yy213;
        break;
      case 154: /* xfullname ::= nm AS nm */
{
    yylhsminor.yy213 = synq_parse_table_ref(pCtx,
        synq_span(pCtx, yymsp[-2].minor.yy0), SYNQ_NO_SPAN, synq_span(pCtx, yymsp[0].minor.yy0));
}
  yymsp[-2].minor.yy213 = yylhsminor.yy213;
        break;
      case 156: /* indexed_opt ::= indexed_by */
      case 315: /* trnm ::= nm */ yytestcase(yyruleno==315);
      case 329: /* nmnum ::= plus_num */ yytestcase(yyruleno==329);
      case 330: /* nmnum ::= nm */ yytestcase(yyruleno==330);
      case 331: /* nmnum ::= ON */ yytestcase(yyruleno==331);
      case 332: /* nmnum ::= DELETE */ yytestcase(yyruleno==332);
      case 333: /* nmnum ::= DEFAULT */ yytestcase(yyruleno==333);
      case 335: /* plus_num ::= INTEGER|FLOAT */ yytestcase(yyruleno==335);
      case 337: /* signed ::= plus_num */ yytestcase(yyruleno==337);
      case 338: /* signed ::= minus_num */ yytestcase(yyruleno==338);
      case 362: /* createkw ::= CREATE */ yytestcase(yyruleno==362);
{
    // Token passthrough
}
        break;
      case 159: /* where_opt_ret ::= RETURNING selcollist */
{
    // Ignore RETURNING clause for now (just discard the column list)
    (void)yymsp[0].minor.yy213;
    yymsp[-1].minor.yy213 = SYNTAQLITE_NULL_NODE;
}
        break;
      case 160: /* where_opt_ret ::= WHERE expr RETURNING selcollist */
{
    // Keep WHERE, ignore RETURNING
    (void)yymsp[0].minor.yy213;
    yymsp[-3].minor.yy213 = yymsp[-2].minor.yy213;
}
        break;
      case 161: /* setlist ::= setlist COMMA nm EQ expr */
{
    uint32_t clause = synq_parse_set_clause(pCtx,
        synq_span(pCtx, yymsp[-2].minor.yy0), SYNTAQLITE_NULL_NODE, yymsp[0].minor.yy213);
    yylhsminor.yy213 = synq_parse_set_clause_list(pCtx, yymsp[-4].minor.yy213, clause);
}
  yymsp[-4].minor.yy213 = yylhsminor.yy213;
        break;
      case 162: /* setlist ::= setlist COMMA LP idlist RP EQ expr */
{
    uint32_t clause = synq_parse_set_clause(pCtx,
        SYNQ_NO_SPAN, yymsp[-3].minor.yy213, yymsp[0].minor.yy213);
    yylhsminor.yy213 = synq_parse_set_clause_list(pCtx, yymsp[-6].minor.yy213, clause);
}
  yymsp[-6].minor.yy213 = yylhsminor.yy213;
        break;
      case 163: /* setlist ::= nm EQ expr */
{
    uint32_t clause = synq_parse_set_clause(pCtx,
        synq_span(pCtx, yymsp[-2].minor.yy0), SYNTAQLITE_NULL_NODE, yymsp[0].minor.yy213);
    yylhsminor.yy213 = synq_parse_set_clause_list(pCtx, SYNTAQLITE_NULL_NODE, clause);
}
  yymsp[-2].minor.yy213 = yylhsminor.yy213;
        break;
      case 164: /* setlist ::= LP idlist RP EQ expr */
{
    uint32_t clause = synq_parse_set_clause(pCtx,
        SYNQ_NO_SPAN, yymsp[-3].minor.yy213, yymsp[0].minor.yy213);
    yymsp[-4].minor.yy213 = synq_parse_set_clause_list(pCtx, SYNTAQLITE_NULL_NODE, clause);
}
        break;
      case 168: /* upsert ::= RETURNING selcollist */
{
    (void)yymsp[0].minor.yy213;
    yymsp[-1].minor.yy213 = SYNTAQLITE_NULL_NODE;
}
        break;
      case 169: /* upsert ::= ON CONFLICT LP sortlist RP where_opt DO UPDATE SET setlist where_opt upsert */
{
    (void)yymsp[-8].minor.yy213; (void)yymsp[-6].minor.yy213; (void)yymsp[-2].minor.yy213; (void)yymsp[-1].minor.yy213; (void)yymsp[0].minor.yy213;
    yymsp[-11].minor.yy213 = SYNTAQLITE_NULL_NODE;
}
        break;
      case 170: /* upsert ::= ON CONFLICT LP sortlist RP where_opt DO NOTHING upsert */
{
    (void)yymsp[-5].minor.yy213; (void)yymsp[-3].minor.yy213; (void)yymsp[0].minor.yy213;
    yymsp[-8].minor.yy213 = SYNTAQLITE_NULL_NODE;
}
        break;
      case 171: /* upsert ::= ON CONFLICT DO NOTHING returning */
{
    yymsp[-4].minor.yy213 = SYNTAQLITE_NULL_NODE;
}
        break;
      case 172: /* upsert ::= ON CONFLICT DO UPDATE SET setlist where_opt returning */
{
    (void)yymsp[-2].minor.yy213; (void)yymsp[-1].minor.yy213;
    yymsp[-7].minor.yy213 = SYNTAQLITE_NULL_NODE;
}
        break;
      case 173: /* returning ::= RETURNING selcollist */
{
    (void)yymsp[0].minor.yy213;
}
        break;
      case 174: /* returning ::= */
      case 309: /* foreach_clause ::= */ yytestcase(yyruleno==309);
      case 317: /* tridxby ::= */ yytestcase(yyruleno==317);
      case 375: /* vtabarg ::= */ yytestcase(yyruleno==375);
      case 380: /* anylist ::= */ yytestcase(yyruleno==380);
{
    // empty
}
        break;
      case 175: /* expr ::= error */
{
    yymsp[0].minor.yy213 = synq_parse_error_node(pCtx, pCtx->error_offset, pCtx->error_length);
}
        break;
      case 178: /* expr ::= expr PLUS|MINUS expr */
{
    SyntaqliteBinaryOp op = (yymsp[-1].minor.yy0.type == SYNTAQLITE_TK_PLUS) ? SYNTAQLITE_BINARY_OP_PLUS : SYNTAQLITE_BINARY_OP_MINUS;
    yylhsminor.yy213 = synq_parse_binary_expr(pCtx, op, yymsp[-2].minor.yy213, yymsp[0].minor.yy213);
}
  yymsp[-2].minor.yy213 = yylhsminor.yy213;
        break;
      case 179: /* expr ::= expr STAR|SLASH|REM expr */
{
    SyntaqliteBinaryOp op;
    switch (yymsp[-1].minor.yy0.type) {
        case SYNTAQLITE_TK_STAR:  op = SYNTAQLITE_BINARY_OP_STAR; break;
        case SYNTAQLITE_TK_SLASH: op = SYNTAQLITE_BINARY_OP_SLASH; break;
        default:       op = SYNTAQLITE_BINARY_OP_REM; break;
    }
    yylhsminor.yy213 = synq_parse_binary_expr(pCtx, op, yymsp[-2].minor.yy213, yymsp[0].minor.yy213);
}
  yymsp[-2].minor.yy213 = yylhsminor.yy213;
        break;
      case 180: /* expr ::= expr LT|GT|GE|LE expr */
{
    SyntaqliteBinaryOp op;
    switch (yymsp[-1].minor.yy0.type) {
        case SYNTAQLITE_TK_LT: op = SYNTAQLITE_BINARY_OP_LT; break;
        case SYNTAQLITE_TK_GT: op = SYNTAQLITE_BINARY_OP_GT; break;
        case SYNTAQLITE_TK_LE: op = SYNTAQLITE_BINARY_OP_LE; break;
        default:    op = SYNTAQLITE_BINARY_OP_GE; break;
    }
    yylhsminor.yy213 = synq_parse_binary_expr(pCtx, op, yymsp[-2].minor.yy213, yymsp[0].minor.yy213);
}
  yymsp[-2].minor.yy213 = yylhsminor.yy213;
        break;
      case 181: /* expr ::= expr EQ|NE expr */
{
    SyntaqliteBinaryOp op = (yymsp[-1].minor.yy0.type == SYNTAQLITE_TK_EQ) ? SYNTAQLITE_BINARY_OP_EQ : SYNTAQLITE_BINARY_OP_NE;
    yylhsminor.yy213 = synq_parse_binary_expr(pCtx, op, yymsp[-2].minor.yy213, yymsp[0].minor.yy213);
}
  yymsp[-2].minor.yy213 = yylhsminor.yy213;
        break;
      case 182: /* expr ::= expr AND expr */
{
    yylhsminor.yy213 = synq_parse_binary_expr(pCtx, SYNTAQLITE_BINARY_OP_AND, yymsp[-2].minor.yy213, yymsp[0].minor.yy213);
}
  yymsp[-2].minor.yy213 = yylhsminor.yy213;
        break;
      case 183: /* expr ::= expr OR expr */
{
    yylhsminor.yy213 = synq_parse_binary_expr(pCtx, SYNTAQLITE_BINARY_OP_OR, yymsp[-2].minor.yy213, yymsp[0].minor.yy213);
}
  yymsp[-2].minor.yy213 = yylhsminor.yy213;
        break;
      case 184: /* expr ::= expr BITAND|BITOR|LSHIFT|RSHIFT expr */
{
    SyntaqliteBinaryOp op;
    switch (yymsp[-1].minor.yy0.type) {
        case SYNTAQLITE_TK_BITAND: op = SYNTAQLITE_BINARY_OP_BIT_AND; break;
        case SYNTAQLITE_TK_BITOR:  op = SYNTAQLITE_BINARY_OP_BIT_OR; break;
        case SYNTAQLITE_TK_LSHIFT: op = SYNTAQLITE_BINARY_OP_LSHIFT; break;
        default:        op = SYNTAQLITE_BINARY_OP_RSHIFT; break;
    }
    yylhsminor.yy213 = synq_parse_binary_expr(pCtx, op, yymsp[-2].minor.yy213, yymsp[0].minor.yy213);
}
  yymsp[-2].minor.yy213 = yylhsminor.yy213;
        break;
      case 185: /* expr ::= expr CONCAT expr */
{
    yylhsminor.yy213 = synq_parse_binary_expr(pCtx, SYNTAQLITE_BINARY_OP_CONCAT, yymsp[-2].minor.yy213, yymsp[0].minor.yy213);
}
  yymsp[-2].minor.yy213 = yylhsminor.yy213;
        break;
      case 186: /* expr ::= expr PTR expr */
{
    yylhsminor.yy213 = synq_parse_binary_expr(pCtx, SYNTAQLITE_BINARY_OP_PTR, yymsp[-2].minor.yy213, yymsp[0].minor.yy213);
}
  yymsp[-2].minor.yy213 = yylhsminor.yy213;
        break;
      case 187: /* expr ::= PLUS|MINUS expr */
{
    SyntaqliteUnaryOp op = (yymsp[-1].minor.yy0.type == SYNTAQLITE_TK_MINUS) ? SYNTAQLITE_UNARY_OP_MINUS : SYNTAQLITE_UNARY_OP_PLUS;
    yylhsminor.yy213 = synq_parse_unary_expr(pCtx, op, yymsp[0].minor.yy213);
}
  yymsp[-1].minor.yy213 = yylhsminor.yy213;
        break;
      case 188: /* expr ::= BITNOT expr */
{
    yymsp[-1].minor.yy213 = synq_parse_unary_expr(pCtx, SYNTAQLITE_UNARY_OP_BIT_NOT, yymsp[0].minor.yy213);
}
        break;
      case 189: /* expr ::= NOT expr */
{
    yymsp[-1].minor.yy213 = synq_parse_unary_expr(pCtx, SYNTAQLITE_UNARY_OP_NOT, yymsp[0].minor.yy213);
}
        break;
      case 192: /* nexprlist ::= nexprlist COMMA expr */
{
    yylhsminor.yy213 = synq_parse_expr_list(pCtx, yymsp[-2].minor.yy213, yymsp[0].minor.yy213);
}
  yymsp[-2].minor.yy213 = yylhsminor.yy213;
        break;
      case 193: /* nexprlist ::= expr */
{
    yylhsminor.yy213 = synq_parse_expr_list(pCtx, SYNTAQLITE_NULL_NODE, yymsp[0].minor.yy213);
}
  yymsp[0].minor.yy213 = yylhsminor.yy213;
        break;
      case 194: /* expr ::= LP nexprlist COMMA expr RP */
{
    yymsp[-4].minor.yy213 = synq_parse_expr_list(pCtx, yymsp[-3].minor.yy213, yymsp[-1].minor.yy213);
}
        break;
      case 195: /* expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist RP */
{
    synq_mark_as_function(pCtx, yymsp[-4].minor.yy0);
    yylhsminor.yy213 = synq_parse_function_call(pCtx,
        synq_span(pCtx, yymsp[-4].minor.yy0),
        (SyntaqliteFunctionCallFlags){.raw = (uint8_t)yymsp[-2].minor.yy213},
        yymsp[-1].minor.yy213,
        SYNTAQLITE_NULL_NODE,
        SYNTAQLITE_NULL_NODE);
}
  yymsp[-4].minor.yy213 = yylhsminor.yy213;
        break;
      case 196: /* expr ::= ID|INDEXED|JOIN_KW LP STAR RP */
{
    synq_mark_as_function(pCtx, yymsp[-3].minor.yy0);
    yylhsminor.yy213 = synq_parse_function_call(pCtx,
        synq_span(pCtx, yymsp[-3].minor.yy0),
        (SyntaqliteFunctionCallFlags){.bits = {.star = 1}},
        SYNTAQLITE_NULL_NODE,
        SYNTAQLITE_NULL_NODE,
        SYNTAQLITE_NULL_NODE);
}
  yymsp[-3].minor.yy213 = yylhsminor.yy213;
        break;
      case 197: /* expr ::= ID|INDEXED|JOIN_KW LP distinct exprlist RP filter_over */
{
    SyntaqliteFilterOver *fo = (SyntaqliteFilterOver*)synq_arena_ptr(&pCtx->ast, yymsp[0].minor.yy213);
    synq_mark_as_function(pCtx, yymsp[-5].minor.yy0);
    yylhsminor.yy213 = synq_parse_function_call(pCtx,
        synq_span(pCtx, yymsp[-5].minor.yy0),
        (SyntaqliteFunctionCallFlags){.raw = (uint8_t)yymsp[-3].minor.yy213},
        yymsp[-2].minor.yy213,
        fo->filter_expr,
        fo->over_def);
}
  yymsp[-5].minor.yy213 = yylhsminor.yy213;
        break;
      case 198: /* expr ::= ID|INDEXED|JOIN_KW LP STAR RP filter_over */
{
    SyntaqliteFilterOver *fo = (SyntaqliteFilterOver*)synq_arena_ptr(&pCtx->ast, yymsp[0].minor.yy213);
    synq_mark_as_function(pCtx, yymsp[-4].minor.yy0);
    yylhsminor.yy213 = synq_parse_function_call(pCtx,
        synq_span(pCtx, yymsp[-4].minor.yy0),
        (SyntaqliteFunctionCallFlags){.bits = {.star = 1}},
        SYNTAQLITE_NULL_NODE,
        fo->filter_expr,
        fo->over_def);
}
  yymsp[-4].minor.yy213 = yylhsminor.yy213;
        break;
      case 199: /* nm ::= error */
{
    yymsp[0].minor.yy0.z = NULL;
    yymsp[0].minor.yy0.n = 0;
}
        break;
      case 200: /* nm ::= ID|INDEXED|JOIN_KW */
{
    synq_mark_as_id(pCtx, yymsp[0].minor.yy0);
    yylhsminor.yy0 = yymsp[0].minor.yy0;
}
  yymsp[0].minor.yy0 = yylhsminor.yy0;
        break;
      case 202: /* term ::= INTEGER */
{
    yylhsminor.yy213 = synq_parse_literal(pCtx, SYNTAQLITE_LITERAL_TYPE_INTEGER, synq_span(pCtx, yymsp[0].minor.yy0));
}
  yymsp[0].minor.yy213 = yylhsminor.yy213;
        break;
      case 203: /* term ::= STRING */
{
    yylhsminor.yy213 = synq_parse_literal(pCtx, SYNTAQLITE_LITERAL_TYPE_STRING, synq_span(pCtx, yymsp[0].minor.yy0));
}
  yymsp[0].minor.yy213 = yylhsminor.yy213;
        break;
      case 204: /* term ::= NULL|FLOAT|BLOB */
{
    SyntaqliteLiteralType lit_type;
    switch (yymsp[0].minor.yy0.type) {
        case SYNTAQLITE_TK_NULL:  lit_type = SYNTAQLITE_LITERAL_TYPE_NULL; break;
        case SYNTAQLITE_TK_FLOAT: lit_type = SYNTAQLITE_LITERAL_TYPE_FLOAT; break;
        case SYNTAQLITE_TK_BLOB:  lit_type = SYNTAQLITE_LITERAL_TYPE_BLOB; break;
        default:       lit_type = SYNTAQLITE_LITERAL_TYPE_NULL; break;
    }
    yylhsminor.yy213 = synq_parse_literal(pCtx, lit_type, synq_span(pCtx, yymsp[0].minor.yy0));
}
  yymsp[0].minor.yy213 = yylhsminor.yy213;
        break;
      case 205: /* term ::= QNUMBER */
{
    yylhsminor.yy213 = synq_parse_literal(pCtx, SYNTAQLITE_LITERAL_TYPE_QNUMBER, synq_span(pCtx, yymsp[0].minor.yy0));
}
  yymsp[0].minor.yy213 = yylhsminor.yy213;
        break;
      case 206: /* term ::= CTIME_KW */
{
    yylhsminor.yy213 = synq_parse_literal(pCtx, SYNTAQLITE_LITERAL_TYPE_CURRENT, synq_span(pCtx, yymsp[0].minor.yy0));
}
  yymsp[0].minor.yy213 = yylhsminor.yy213;
        break;
      case 207: /* expr ::= VARIABLE */
{
    yylhsminor.yy213 = synq_parse_variable(pCtx, synq_span(pCtx, yymsp[0].minor.yy0));
}
  yymsp[0].minor.yy213 = yylhsminor.yy213;
        break;
      case 208: /* expr ::= expr COLLATE ID|STRING */
{
    yylhsminor.yy213 = synq_parse_collate_expr(pCtx, yymsp[-2].minor.yy213, synq_span(pCtx, yymsp[0].minor.yy0));
}
  yymsp[-2].minor.yy213 = yylhsminor.yy213;
        break;
      case 209: /* sortlist ::= sortlist COMMA expr sortorder nulls */
{
    uint32_t term = synq_parse_ordering_term(pCtx, yymsp[-2].minor.yy213, (SyntaqliteSortOrder)yymsp[-1].minor.yy213, (SyntaqliteNullsOrder)yymsp[0].minor.yy213);
    yylhsminor.yy213 = synq_parse_order_by_list(pCtx, yymsp[-4].minor.yy213, term);
}
  yymsp[-4].minor.yy213 = yylhsminor.yy213;
        break;
      case 210: /* sortlist ::= expr sortorder nulls */
{
    uint32_t term = synq_parse_ordering_term(pCtx, yymsp[-2].minor.yy213, (SyntaqliteSortOrder)yymsp[-1].minor.yy213, (SyntaqliteNullsOrder)yymsp[0].minor.yy213);
    yylhsminor.yy213 = synq_parse_order_by_list(pCtx, SYNTAQLITE_NULL_NODE, term);
}
  yymsp[-2].minor.yy213 = yylhsminor.yy213;
        break;
      case 212: /* sortorder ::= DESC */
      case 266: /* distinct ::= DISTINCT */ yytestcase(yyruleno==266);
{
    yymsp[0].minor.yy213 = 1;
}
        break;
      case 213: /* sortorder ::= */
      case 216: /* nulls ::= */ yytestcase(yyruleno==216);
      case 268: /* distinct ::= */ yytestcase(yyruleno==268);
{
    yymsp[1].minor.yy213 = 0;
}
        break;
      case 215: /* nulls ::= NULLS LAST */
{
    yymsp[-1].minor.yy213 = 2;
}
        break;
      case 217: /* expr ::= RAISE LP IGNORE RP */
{
    yymsp[-3].minor.yy213 = synq_parse_raise_expr(pCtx, SYNTAQLITE_RAISE_TYPE_IGNORE, SYNTAQLITE_NULL_NODE);
}
        break;
      case 218: /* expr ::= RAISE LP raisetype COMMA expr RP */
{
    yymsp[-5].minor.yy213 = synq_parse_raise_expr(pCtx, (SyntaqliteRaiseType)yymsp[-3].minor.yy220, yymsp[-1].minor.yy213);
}
        break;
      case 219: /* raisetype ::= ROLLBACK */
{ yymsp[0].minor.yy220 = SYNTAQLITE_RAISE_TYPE_ROLLBACK; }
        break;
      case 220: /* raisetype ::= ABORT */
{ yymsp[0].minor.yy220 = SYNTAQLITE_RAISE_TYPE_ABORT; }
        break;
      case 221: /* raisetype ::= FAIL */
{ yymsp[0].minor.yy220 = SYNTAQLITE_RAISE_TYPE_FAIL; }
        break;
      case 222: /* fullname ::= nm */
{
    yylhsminor.yy213 = synq_parse_qualified_name(pCtx,
        synq_span(pCtx, yymsp[0].minor.yy0),
        SYNQ_NO_SPAN);
}
  yymsp[0].minor.yy213 = yylhsminor.yy213;
        break;
      case 223: /* fullname ::= nm DOT nm */
{
    yylhsminor.yy213 = synq_parse_qualified_name(pCtx,
        synq_span(pCtx, yymsp[0].minor.yy0),
        synq_span(pCtx, yymsp[-2].minor.yy0));
}
  yymsp[-2].minor.yy213 = yylhsminor.yy213;
        break;
      case 226: /* cmd ::= DROP TABLE ifexists fullname */
{
    yymsp[-3].minor.yy213 = synq_parse_drop_stmt(pCtx, SYNTAQLITE_DROP_OBJECT_TYPE_TABLE, (SyntaqliteBool)yymsp[-1].minor.yy220, yymsp[0].minor.yy213);
}
        break;
      case 227: /* cmd ::= DROP VIEW ifexists fullname */
{
    yymsp[-3].minor.yy213 = synq_parse_drop_stmt(pCtx, SYNTAQLITE_DROP_OBJECT_TYPE_VIEW, (SyntaqliteBool)yymsp[-1].minor.yy220, yymsp[0].minor.yy213);
}
        break;
      case 228: /* cmd ::= DROP INDEX ifexists fullname */
{
    yymsp[-3].minor.yy213 = synq_parse_drop_stmt(pCtx, SYNTAQLITE_DROP_OBJECT_TYPE_INDEX, (SyntaqliteBool)yymsp[-1].minor.yy220, yymsp[0].minor.yy213);
}
        break;
      case 229: /* cmd ::= DROP TRIGGER ifexists fullname */
{
    yymsp[-3].minor.yy213 = synq_parse_drop_stmt(pCtx, SYNTAQLITE_DROP_OBJECT_TYPE_TRIGGER, (SyntaqliteBool)yymsp[-1].minor.yy220, yymsp[0].minor.yy213);
}
        break;
      case 230: /* cmd ::= ALTER TABLE fullname RENAME TO nm */
{
    yymsp[-5].minor.yy213 = synq_parse_alter_table_stmt(pCtx,
        SYNTAQLITE_ALTER_OP_RENAME_TABLE, yymsp[-3].minor.yy213,
        synq_span(pCtx, yymsp[0].minor.yy0),
        SYNQ_NO_SPAN);
}
        break;
      case 231: /* cmd ::= ALTER TABLE fullname RENAME kwcolumn_opt nm TO nm */
{
    yymsp[-7].minor.yy213 = synq_parse_alter_table_stmt(pCtx,
        SYNTAQLITE_ALTER_OP_RENAME_COLUMN, yymsp[-5].minor.yy213,
        synq_span(pCtx, yymsp[0].minor.yy0),
        synq_span(pCtx, yymsp[-2].minor.yy0));
}
        break;
      case 232: /* cmd ::= ALTER TABLE fullname DROP kwcolumn_opt nm */
{
    yymsp[-5].minor.yy213 = synq_parse_alter_table_stmt(pCtx,
        SYNTAQLITE_ALTER_OP_DROP_COLUMN, yymsp[-3].minor.yy213,
        SYNQ_NO_SPAN,
        synq_span(pCtx, yymsp[0].minor.yy0));
}
        break;
      case 233: /* cmd ::= ALTER TABLE add_column_fullname ADD kwcolumn_opt columnname carglist */
{
    yymsp[-6].minor.yy213 = synq_parse_alter_table_stmt(pCtx,
        SYNTAQLITE_ALTER_OP_ADD_COLUMN, SYNTAQLITE_NULL_NODE,
        SYNQ_NO_SPAN,
        yymsp[-1].minor.yy400.name);
}
        break;
      case 234: /* add_column_fullname ::= fullname */
{
    // Passthrough - fullname already produces a node ID but we don't need it
    // for the ADD COLUMN action since add_column_fullname is consumed by cmd
}
        break;
      case 237: /* columnname ::= nm typetoken */
{
    yylhsminor.yy400.name = synq_span(pCtx, yymsp[-1].minor.yy0);
    yylhsminor.yy400.typetoken = yymsp[0].minor.yy0.z ? synq_span(pCtx, yymsp[0].minor.yy0) : SYNQ_NO_SPAN;
}
  yymsp[-1].minor.yy400 = yylhsminor.yy400;
        break;
      case 238: /* cmd ::= BEGIN transtype trans_opt */
{
    yymsp[-2].minor.yy213 = synq_parse_transaction_stmt(pCtx,
        SYNTAQLITE_TRANSACTION_OP_BEGIN,
        (SyntaqliteTransactionType)yymsp[-1].minor.yy220);
}
        break;
      case 239: /* cmd ::= COMMIT|END trans_opt */
{
    yymsp[-1].minor.yy213 = synq_parse_transaction_stmt(pCtx,
        SYNTAQLITE_TRANSACTION_OP_COMMIT,
        SYNTAQLITE_TRANSACTION_TYPE_DEFERRED);
}
        break;
      case 240: /* cmd ::= ROLLBACK trans_opt */
{
    yymsp[-1].minor.yy213 = synq_parse_transaction_stmt(pCtx,
        SYNTAQLITE_TRANSACTION_OP_ROLLBACK,
        SYNTAQLITE_TRANSACTION_TYPE_DEFERRED);
}
        break;
      case 241: /* transtype ::= */
{
    yymsp[1].minor.yy220 = (int)SYNTAQLITE_TRANSACTION_TYPE_DEFERRED;
}
        break;
      case 242: /* transtype ::= DEFERRED */
{
    yymsp[0].minor.yy220 = (int)SYNTAQLITE_TRANSACTION_TYPE_DEFERRED;
}
        break;
      case 243: /* transtype ::= IMMEDIATE */
{
    yymsp[0].minor.yy220 = (int)SYNTAQLITE_TRANSACTION_TYPE_IMMEDIATE;
}
        break;
      case 244: /* transtype ::= EXCLUSIVE */
{
    yymsp[0].minor.yy220 = (int)SYNTAQLITE_TRANSACTION_TYPE_EXCLUSIVE;
}
        break;
      case 246: /* trans_opt ::= TRANSACTION */
      case 248: /* savepoint_opt ::= SAVEPOINT */ yytestcase(yyruleno==248);
{
    yymsp[0].minor.yy220 = 0;
}
        break;
      case 250: /* cmd ::= SAVEPOINT nm */
{
    yymsp[-1].minor.yy213 = synq_parse_savepoint_stmt(pCtx,
        SYNTAQLITE_SAVEPOINT_OP_SAVEPOINT,
        synq_span(pCtx, yymsp[0].minor.yy0));
}
        break;
      case 251: /* cmd ::= RELEASE savepoint_opt nm */
{
    yymsp[-2].minor.yy213 = synq_parse_savepoint_stmt(pCtx,
        SYNTAQLITE_SAVEPOINT_OP_RELEASE,
        synq_span(pCtx, yymsp[0].minor.yy0));
}
        break;
      case 252: /* cmd ::= ROLLBACK trans_opt TO savepoint_opt nm */
{
    yymsp[-4].minor.yy213 = synq_parse_savepoint_stmt(pCtx,
        SYNTAQLITE_SAVEPOINT_OP_ROLLBACK_TO,
        synq_span(pCtx, yymsp[0].minor.yy0));
}
        break;
      case 256: /* oneselect ::= SELECT distinct selcollist from where_opt groupby_opt having_opt orderby_opt limit_opt */
{
    yymsp[-8].minor.yy213 = synq_parse_select_stmt(pCtx, (SyntaqliteSelectStmtFlags){.raw = (uint8_t)yymsp[-7].minor.yy213}, yymsp[-6].minor.yy213, yymsp[-5].minor.yy213, yymsp[-4].minor.yy213, yymsp[-3].minor.yy213, yymsp[-2].minor.yy213, yymsp[-1].minor.yy213, yymsp[0].minor.yy213, SYNTAQLITE_NULL_NODE);
}
        break;
      case 257: /* oneselect ::= SELECT distinct selcollist from where_opt groupby_opt having_opt window_clause orderby_opt limit_opt */
{
    yymsp[-9].minor.yy213 = synq_parse_select_stmt(pCtx, (SyntaqliteSelectStmtFlags){.raw = (uint8_t)yymsp[-8].minor.yy213}, yymsp[-7].minor.yy213, yymsp[-6].minor.yy213, yymsp[-5].minor.yy213, yymsp[-4].minor.yy213, yymsp[-3].minor.yy213, yymsp[-1].minor.yy213, yymsp[0].minor.yy213, yymsp[-2].minor.yy213);
}
        break;
      case 258: /* selcollist ::= sclp scanpt expr scanpt as */
{
    SyntaqliteSourceSpan alias = (yymsp[0].minor.yy0.z) ? synq_span(pCtx, yymsp[0].minor.yy0) : SYNQ_NO_SPAN;
    uint32_t col = synq_parse_result_column(pCtx, (SyntaqliteResultColumnFlags){0}, alias, yymsp[-2].minor.yy213);
    yylhsminor.yy213 = synq_parse_result_column_list(pCtx, yymsp[-4].minor.yy213, col);
}
  yymsp[-4].minor.yy213 = yylhsminor.yy213;
        break;
      case 259: /* selcollist ::= sclp scanpt STAR */
{
    uint32_t col = synq_parse_result_column(pCtx, (SyntaqliteResultColumnFlags){.bits = {.star = 1}}, SYNQ_NO_SPAN, SYNTAQLITE_NULL_NODE);
    yylhsminor.yy213 = synq_parse_result_column_list(pCtx, yymsp[-2].minor.yy213, col);
}
  yymsp[-2].minor.yy213 = yylhsminor.yy213;
        break;
      case 260: /* sclp ::= selcollist COMMA */
{
    yylhsminor.yy213 = yymsp[-1].minor.yy213;
}
  yymsp[-1].minor.yy213 = yylhsminor.yy213;
        break;
      case 263: /* as ::= AS nm */
      case 334: /* plus_num ::= PLUS INTEGER|FLOAT */ yytestcase(yyruleno==334);
{
    yymsp[-1].minor.yy0 = yymsp[0].minor.yy0;
}
        break;
      case 274: /* groupby_opt ::= GROUP BY nexprlist */
      case 278: /* orderby_opt ::= ORDER BY sortlist */ yytestcase(yyruleno==278);
{
    yymsp[-2].minor.yy213 = yymsp[0].minor.yy213;
}
        break;
      case 280: /* limit_opt ::= LIMIT expr */
{
    yymsp[-1].minor.yy213 = synq_parse_limit_clause(pCtx, yymsp[0].minor.yy213, SYNTAQLITE_NULL_NODE);
}
        break;
      case 281: /* limit_opt ::= LIMIT expr OFFSET expr */
{
    yymsp[-3].minor.yy213 = synq_parse_limit_clause(pCtx, yymsp[-2].minor.yy213, yymsp[0].minor.yy213);
}
        break;
      case 282: /* limit_opt ::= LIMIT expr COMMA expr */
{
    yymsp[-3].minor.yy213 = synq_parse_limit_clause(pCtx, yymsp[0].minor.yy213, yymsp[-2].minor.yy213);
}
        break;
      case 283: /* stl_prefix ::= seltablist joinop */
{
    yymsp[-1].minor.yy213 = synq_parse_join_prefix(pCtx, yymsp[-1].minor.yy213, (SyntaqliteJoinType)yymsp[0].minor.yy220);
}
        break;
      case 285: /* seltablist ::= stl_prefix nm dbnm as on_using */
{
    SyntaqliteSourceSpan alias = (yymsp[-1].minor.yy0.z != NULL) ? synq_span(pCtx, yymsp[-1].minor.yy0) : SYNQ_NO_SPAN;
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
        SyntaqliteNode *pfx = AST_NODE(&pCtx->ast, yymsp[-4].minor.yy213);
        yymsp[-4].minor.yy213 = synq_parse_join_clause(pCtx,
            pfx->join_prefix.join_type,
            pfx->join_prefix.source,
            tref, yymsp[0].minor.yy304.on_expr, yymsp[0].minor.yy304.using_cols);
    }
}
        break;
      case 286: /* seltablist ::= stl_prefix nm dbnm as indexed_by on_using */
{
    (void)yymsp[-1].minor.yy0;
    SyntaqliteSourceSpan alias = (yymsp[-2].minor.yy0.z != NULL) ? synq_span(pCtx, yymsp[-2].minor.yy0) : SYNQ_NO_SPAN;
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
        SyntaqliteNode *pfx = AST_NODE(&pCtx->ast, yymsp[-5].minor.yy213);
        yymsp[-5].minor.yy213 = synq_parse_join_clause(pCtx,
            pfx->join_prefix.join_type,
            pfx->join_prefix.source,
            tref, yymsp[0].minor.yy304.on_expr, yymsp[0].minor.yy304.using_cols);
    }
}
        break;
      case 287: /* seltablist ::= stl_prefix nm dbnm LP exprlist RP as on_using */
{
    (void)yymsp[-3].minor.yy213;
    SyntaqliteSourceSpan alias = (yymsp[-1].minor.yy0.z != NULL) ? synq_span(pCtx, yymsp[-1].minor.yy0) : SYNQ_NO_SPAN;
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
        SyntaqliteNode *pfx = AST_NODE(&pCtx->ast, yymsp[-7].minor.yy213);
        yymsp[-7].minor.yy213 = synq_parse_join_clause(pCtx,
            pfx->join_prefix.join_type,
            pfx->join_prefix.source,
            tref, yymsp[0].minor.yy304.on_expr, yymsp[0].minor.yy304.using_cols);
    }
}
        break;
      case 288: /* seltablist ::= stl_prefix LP select RP as on_using */
{
    pCtx->saw_subquery = 1;
    SyntaqliteSourceSpan alias = (yymsp[-1].minor.yy0.z != NULL) ? synq_span(pCtx, yymsp[-1].minor.yy0) : SYNQ_NO_SPAN;
    uint32_t sub = synq_parse_subquery_table_source(pCtx, yymsp[-3].minor.yy213, alias);
    if (yymsp[-5].minor.yy213 == SYNTAQLITE_NULL_NODE) {
        yymsp[-5].minor.yy213 = sub;
    } else {
        SyntaqliteNode *pfx = AST_NODE(&pCtx->ast, yymsp[-5].minor.yy213);
        yymsp[-5].minor.yy213 = synq_parse_join_clause(pCtx,
            pfx->join_prefix.join_type,
            pfx->join_prefix.source,
            sub, yymsp[0].minor.yy304.on_expr, yymsp[0].minor.yy304.using_cols);
    }
}
        break;
      case 289: /* seltablist ::= stl_prefix LP seltablist RP as on_using */
{
    (void)yymsp[-1].minor.yy0; (void)yymsp[0].minor.yy304;
    if (yymsp[-5].minor.yy213 == SYNTAQLITE_NULL_NODE) {
        yymsp[-5].minor.yy213 = yymsp[-3].minor.yy213;
    } else {
        SyntaqliteNode *pfx = AST_NODE(&pCtx->ast, yymsp[-5].minor.yy213);
        yymsp[-5].minor.yy213 = synq_parse_join_clause(pCtx,
            pfx->join_prefix.join_type,
            pfx->join_prefix.source,
            yymsp[-3].minor.yy213, yymsp[0].minor.yy304.on_expr, yymsp[0].minor.yy304.using_cols);
    }
}
        break;
      case 290: /* joinop ::= COMMA|JOIN */
{
    yylhsminor.yy220 = (yymsp[0].minor.yy0.type == SYNTAQLITE_TK_COMMA)
        ? (int)SYNTAQLITE_JOIN_TYPE_COMMA
        : (int)SYNTAQLITE_JOIN_TYPE_INNER;
}
  yymsp[0].minor.yy220 = yylhsminor.yy220;
        break;
      case 291: /* joinop ::= JOIN_KW JOIN */
{
    // Single keyword: LEFT, RIGHT, INNER, OUTER, CROSS, NATURAL, FULL
    if (yymsp[-1].minor.yy0.n == 4 && (yymsp[-1].minor.yy0.z[0] == 'L' || yymsp[-1].minor.yy0.z[0] == 'l')) {
        yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_LEFT;
    } else if (yymsp[-1].minor.yy0.n == 5 && (yymsp[-1].minor.yy0.z[0] == 'R' || yymsp[-1].minor.yy0.z[0] == 'r')) {
        yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_RIGHT;
    } else if (yymsp[-1].minor.yy0.n == 5 && (yymsp[-1].minor.yy0.z[0] == 'I' || yymsp[-1].minor.yy0.z[0] == 'i')) {
        yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_INNER;
    } else if (yymsp[-1].minor.yy0.n == 5 && (yymsp[-1].minor.yy0.z[0] == 'O' || yymsp[-1].minor.yy0.z[0] == 'o')) {
        // OUTER alone is not valid but treat as INNER
        yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_INNER;
    } else if (yymsp[-1].minor.yy0.n == 5 && (yymsp[-1].minor.yy0.z[0] == 'C' || yymsp[-1].minor.yy0.z[0] == 'c')) {
        yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_CROSS;
    } else if (yymsp[-1].minor.yy0.n == 7 && (yymsp[-1].minor.yy0.z[0] == 'N' || yymsp[-1].minor.yy0.z[0] == 'n')) {
        yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_NATURAL_INNER;
    } else if (yymsp[-1].minor.yy0.n == 4 && (yymsp[-1].minor.yy0.z[0] == 'F' || yymsp[-1].minor.yy0.z[0] == 'f')) {
        yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_FULL;
    } else {
        yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_INNER;
    }
}
  yymsp[-1].minor.yy220 = yylhsminor.yy220;
        break;
      case 292: /* joinop ::= JOIN_KW nm JOIN */
{
    // Two keywords: LEFT OUTER, NATURAL LEFT, NATURAL RIGHT, etc.
    (void)yymsp[-1].minor.yy0;
    if (yymsp[-2].minor.yy0.n == 7 && (yymsp[-2].minor.yy0.z[0] == 'N' || yymsp[-2].minor.yy0.z[0] == 'n')) {
        // NATURAL + something
        if (yymsp[-1].minor.yy0.n == 4 && (yymsp[-1].minor.yy0.z[0] == 'L' || yymsp[-1].minor.yy0.z[0] == 'l')) {
            yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_NATURAL_LEFT;
        } else if (yymsp[-1].minor.yy0.n == 5 && (yymsp[-1].minor.yy0.z[0] == 'R' || yymsp[-1].minor.yy0.z[0] == 'r')) {
            yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_NATURAL_RIGHT;
        } else if (yymsp[-1].minor.yy0.n == 5 && (yymsp[-1].minor.yy0.z[0] == 'I' || yymsp[-1].minor.yy0.z[0] == 'i')) {
            yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_NATURAL_INNER;
        } else if (yymsp[-1].minor.yy0.n == 4 && (yymsp[-1].minor.yy0.z[0] == 'F' || yymsp[-1].minor.yy0.z[0] == 'f')) {
            yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_NATURAL_FULL;
        } else if (yymsp[-1].minor.yy0.n == 5 && (yymsp[-1].minor.yy0.z[0] == 'C' || yymsp[-1].minor.yy0.z[0] == 'c')) {
            // NATURAL CROSS -> just CROSS
            yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_CROSS;
        } else {
            yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_NATURAL_INNER;
        }
    } else if (yymsp[-2].minor.yy0.n == 4 && (yymsp[-2].minor.yy0.z[0] == 'L' || yymsp[-2].minor.yy0.z[0] == 'l')) {
        // LEFT OUTER
        yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_LEFT;
    } else if (yymsp[-2].minor.yy0.n == 5 && (yymsp[-2].minor.yy0.z[0] == 'R' || yymsp[-2].minor.yy0.z[0] == 'r')) {
        // RIGHT OUTER
        yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_RIGHT;
    } else if (yymsp[-2].minor.yy0.n == 4 && (yymsp[-2].minor.yy0.z[0] == 'F' || yymsp[-2].minor.yy0.z[0] == 'f')) {
        // FULL OUTER
        yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_FULL;
    } else {
        yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_INNER;
    }
}
  yymsp[-2].minor.yy220 = yylhsminor.yy220;
        break;
      case 293: /* joinop ::= JOIN_KW nm nm JOIN */
{
    // Three keywords: NATURAL LEFT OUTER, NATURAL RIGHT OUTER, etc.
    (void)yymsp[-2].minor.yy0; (void)yymsp[-1].minor.yy0;
    if (yymsp[-3].minor.yy0.n == 7 && (yymsp[-3].minor.yy0.z[0] == 'N' || yymsp[-3].minor.yy0.z[0] == 'n')) {
        // NATURAL yylhsminor.yy220 OUTER
        if (yymsp[-2].minor.yy0.n == 4 && (yymsp[-2].minor.yy0.z[0] == 'L' || yymsp[-2].minor.yy0.z[0] == 'l')) {
            yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_NATURAL_LEFT;
        } else if (yymsp[-2].minor.yy0.n == 5 && (yymsp[-2].minor.yy0.z[0] == 'R' || yymsp[-2].minor.yy0.z[0] == 'r')) {
            yylhsminor.yy220 = (int)SYNTAQLITE_JOIN_TYPE_NATURAL_RIGHT;
        } else if (yymsp[-2].minor.yy0.n == 4 && (yymsp[-2].minor.yy0.z[0] == 'F' || yymsp[-2].minor.yy0.z[0] == 'f')) {
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
      case 294: /* on_using ::= ON expr */
{
    yymsp[-1].minor.yy304.on_expr = yymsp[0].minor.yy213;
    yymsp[-1].minor.yy304.using_cols = SYNTAQLITE_NULL_NODE;
}
        break;
      case 295: /* on_using ::= USING LP idlist RP */
{
    yymsp[-3].minor.yy304.on_expr = SYNTAQLITE_NULL_NODE;
    yymsp[-3].minor.yy304.using_cols = yymsp[-1].minor.yy213;
}
        break;
      case 296: /* on_using ::= */
{
    yymsp[1].minor.yy304.on_expr = SYNTAQLITE_NULL_NODE;
    yymsp[1].minor.yy304.using_cols = SYNTAQLITE_NULL_NODE;
}
        break;
      case 297: /* indexed_by ::= INDEXED BY nm */
{
    yymsp[-2].minor.yy0 = yymsp[0].minor.yy0;
}
        break;
      case 298: /* indexed_by ::= NOT INDEXED */
{
    yymsp[-1].minor.yy0.z = NULL; yymsp[-1].minor.yy0.n = 1;
}
        break;
      case 299: /* idlist ::= idlist COMMA nm */
{
    uint32_t col = synq_parse_column_ref(pCtx,
        synq_span(pCtx, yymsp[0].minor.yy0), SYNQ_NO_SPAN, SYNQ_NO_SPAN);
    yymsp[-2].minor.yy213 = synq_parse_expr_list(pCtx, yymsp[-2].minor.yy213, col);
}
        break;
      case 300: /* idlist ::= nm */
{
    uint32_t col = synq_parse_column_ref(pCtx,
        synq_span(pCtx, yymsp[0].minor.yy0), SYNQ_NO_SPAN, SYNQ_NO_SPAN);
    yylhsminor.yy213 = synq_parse_expr_list(pCtx, SYNTAQLITE_NULL_NODE, col);
}
  yymsp[0].minor.yy213 = yylhsminor.yy213;
        break;
      case 301: /* cmd ::= createkw trigger_decl BEGIN trigger_cmd_list END */
{
    // yymsp[-3].minor.yy213 is a partially-built CreateTriggerStmt, fill in the body
    SyntaqliteNode *trig = AST_NODE(&pCtx->ast, yymsp[-3].minor.yy213);
    trig->create_trigger_stmt.body = yymsp[-1].minor.yy213;
    yymsp[-4].minor.yy213 = yymsp[-3].minor.yy213;
}
        break;
      case 302: /* trigger_decl ::= temp TRIGGER ifnotexists nm dbnm trigger_time trigger_event ON fullname foreach_clause when_clause */
{
    SyntaqliteSourceSpan trig_name = yymsp[-6].minor.yy0.z ? synq_span(pCtx, yymsp[-6].minor.yy0) : synq_span(pCtx, yymsp[-7].minor.yy0);
    SyntaqliteSourceSpan trig_schema = yymsp[-6].minor.yy0.z ? synq_span(pCtx, yymsp[-7].minor.yy0) : SYNQ_NO_SPAN;
    yylhsminor.yy213 = synq_parse_create_trigger_stmt(pCtx,
        trig_name,
        trig_schema,
        (SyntaqliteBool)yymsp[-10].minor.yy220,
        (SyntaqliteBool)yymsp[-8].minor.yy220,
        (SyntaqliteTriggerTiming)yymsp[-5].minor.yy220,
        yymsp[-4].minor.yy213,
        yymsp[-2].minor.yy213,
        yymsp[0].minor.yy213,
        SYNTAQLITE_NULL_NODE);  // body filled in by cmd rule
}
  yymsp[-10].minor.yy213 = yylhsminor.yy213;
        break;
      case 303: /* trigger_time ::= BEFORE|AFTER */
{
    yylhsminor.yy220 = (yymsp[0].minor.yy0.type == SYNTAQLITE_TK_BEFORE) ? (int)SYNTAQLITE_TRIGGER_TIMING_BEFORE
                               : (int)SYNTAQLITE_TRIGGER_TIMING_AFTER;
}
  yymsp[0].minor.yy220 = yylhsminor.yy220;
        break;
      case 304: /* trigger_time ::= INSTEAD OF */
{
    yymsp[-1].minor.yy220 = (int)SYNTAQLITE_TRIGGER_TIMING_INSTEAD_OF;
}
        break;
      case 305: /* trigger_time ::= */
{
    yymsp[1].minor.yy220 = (int)SYNTAQLITE_TRIGGER_TIMING_BEFORE;
}
        break;
      case 306: /* trigger_event ::= DELETE|INSERT */
{
    SyntaqliteTriggerEventType evt = (yymsp[0].minor.yy0.type == SYNTAQLITE_TK_DELETE)
        ? SYNTAQLITE_TRIGGER_EVENT_TYPE_DELETE
        : SYNTAQLITE_TRIGGER_EVENT_TYPE_INSERT;
    yylhsminor.yy213 = synq_parse_trigger_event(pCtx, evt, SYNTAQLITE_NULL_NODE);
}
  yymsp[0].minor.yy213 = yylhsminor.yy213;
        break;
      case 307: /* trigger_event ::= UPDATE */
{
    yymsp[0].minor.yy213 = synq_parse_trigger_event(pCtx,
        SYNTAQLITE_TRIGGER_EVENT_TYPE_UPDATE, SYNTAQLITE_NULL_NODE);
}
        break;
      case 308: /* trigger_event ::= UPDATE OF idlist */
{
    yymsp[-2].minor.yy213 = synq_parse_trigger_event(pCtx,
        SYNTAQLITE_TRIGGER_EVENT_TYPE_UPDATE, yymsp[0].minor.yy213);
}
        break;
      case 310: /* foreach_clause ::= FOR EACH ROW */
      case 373: /* vtabarglist ::= vtabarg */ yytestcase(yyruleno==373);
      case 374: /* vtabarglist ::= vtabarglist COMMA vtabarg */ yytestcase(yyruleno==374);
      case 376: /* vtabarg ::= vtabarg vtabargtoken */ yytestcase(yyruleno==376);
      case 377: /* vtabargtoken ::= ANY */ yytestcase(yyruleno==377);
      case 378: /* vtabargtoken ::= lp anylist RP */ yytestcase(yyruleno==378);
      case 379: /* lp ::= LP */ yytestcase(yyruleno==379);
      case 381: /* anylist ::= anylist LP anylist RP */ yytestcase(yyruleno==381);
      case 382: /* anylist ::= anylist ANY */ yytestcase(yyruleno==382);
{
    // consumed
}
        break;
      case 313: /* trigger_cmd_list ::= trigger_cmd_list trigger_cmd SEMI */
{
    yylhsminor.yy213 = synq_parse_trigger_cmd_list(pCtx, yymsp[-2].minor.yy213, yymsp[-1].minor.yy213);
}
  yymsp[-2].minor.yy213 = yylhsminor.yy213;
        break;
      case 314: /* trigger_cmd_list ::= trigger_cmd SEMI */
{
    yylhsminor.yy213 = synq_parse_trigger_cmd_list(pCtx, SYNTAQLITE_NULL_NODE, yymsp[-1].minor.yy213);
}
  yymsp[-1].minor.yy213 = yylhsminor.yy213;
        break;
      case 316: /* trnm ::= nm DOT nm */
{
    yymsp[-2].minor.yy0 = yymsp[0].minor.yy0;
    // Qualified names not allowed in triggers, but grammar accepts them
}
        break;
      case 318: /* tridxby ::= INDEXED BY nm */
      case 319: /* tridxby ::= NOT INDEXED */ yytestcase(yyruleno==319);
{
    // Not allowed in triggers, but grammar accepts
}
        break;
      case 320: /* trigger_cmd ::= UPDATE orconf trnm tridxby SET setlist from where_opt scanpt */
{
    uint32_t tbl = synq_parse_table_ref(pCtx,
        synq_span(pCtx, yymsp[-6].minor.yy0), SYNQ_NO_SPAN, SYNQ_NO_SPAN);
    yymsp[-8].minor.yy213 = synq_parse_update_stmt(pCtx, (SyntaqliteConflictAction)yymsp[-7].minor.yy220, tbl, yymsp[-3].minor.yy213, yymsp[-2].minor.yy213, yymsp[-1].minor.yy213,
        SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE);
}
        break;
      case 321: /* trigger_cmd ::= scanpt insert_cmd INTO trnm idlist_opt select upsert scanpt */
{
    uint32_t tbl = synq_parse_table_ref(pCtx,
        synq_span(pCtx, yymsp[-4].minor.yy0), SYNQ_NO_SPAN, SYNQ_NO_SPAN);
    yymsp[-7].minor.yy213 = synq_parse_insert_stmt(pCtx, (SyntaqliteConflictAction)yymsp[-6].minor.yy220, tbl, yymsp[-3].minor.yy213, yymsp[-2].minor.yy213);
}
        break;
      case 322: /* trigger_cmd ::= DELETE FROM trnm tridxby where_opt scanpt */
{
    uint32_t tbl = synq_parse_table_ref(pCtx,
        synq_span(pCtx, yymsp[-3].minor.yy0), SYNQ_NO_SPAN, SYNQ_NO_SPAN);
    yymsp[-5].minor.yy213 = synq_parse_delete_stmt(pCtx, tbl, yymsp[-1].minor.yy213,
        SYNTAQLITE_NULL_NODE, SYNTAQLITE_NULL_NODE);
}
        break;
      case 324: /* cmd ::= PRAGMA nm dbnm */
{
    SyntaqliteSourceSpan name_span = yymsp[0].minor.yy0.z ? synq_span(pCtx, yymsp[0].minor.yy0) : synq_span(pCtx, yymsp[-1].minor.yy0);
    SyntaqliteSourceSpan schema_span = yymsp[0].minor.yy0.z ? synq_span(pCtx, yymsp[-1].minor.yy0) : SYNQ_NO_SPAN;
    yymsp[-2].minor.yy213 = synq_parse_pragma_stmt(pCtx, name_span, schema_span, SYNQ_NO_SPAN, SYNTAQLITE_PRAGMA_FORM_BARE);
}
        break;
      case 325: /* cmd ::= PRAGMA nm dbnm EQ nmnum */
      case 327: /* cmd ::= PRAGMA nm dbnm EQ minus_num */ yytestcase(yyruleno==327);
{
    SyntaqliteSourceSpan name_span = yymsp[-2].minor.yy0.z ? synq_span(pCtx, yymsp[-2].minor.yy0) : synq_span(pCtx, yymsp[-3].minor.yy0);
    SyntaqliteSourceSpan schema_span = yymsp[-2].minor.yy0.z ? synq_span(pCtx, yymsp[-3].minor.yy0) : SYNQ_NO_SPAN;
    yymsp[-4].minor.yy213 = synq_parse_pragma_stmt(pCtx, name_span, schema_span, synq_span(pCtx, yymsp[0].minor.yy0), SYNTAQLITE_PRAGMA_FORM_EQ);
}
        break;
      case 326: /* cmd ::= PRAGMA nm dbnm LP nmnum RP */
      case 328: /* cmd ::= PRAGMA nm dbnm LP minus_num RP */ yytestcase(yyruleno==328);
{
    SyntaqliteSourceSpan name_span = yymsp[-3].minor.yy0.z ? synq_span(pCtx, yymsp[-3].minor.yy0) : synq_span(pCtx, yymsp[-4].minor.yy0);
    SyntaqliteSourceSpan schema_span = yymsp[-3].minor.yy0.z ? synq_span(pCtx, yymsp[-4].minor.yy0) : SYNQ_NO_SPAN;
    yymsp[-5].minor.yy213 = synq_parse_pragma_stmt(pCtx, name_span, schema_span, synq_span(pCtx, yymsp[-1].minor.yy0), SYNTAQLITE_PRAGMA_FORM_CALL);
}
        break;
      case 336: /* minus_num ::= MINUS INTEGER|FLOAT */
{
    // Build a token that spans from the MINUS sign through the number
    yylhsminor.yy0.z = yymsp[-1].minor.yy0.z;
    yylhsminor.yy0.n = (int)(yymsp[0].minor.yy0.z - yymsp[-1].minor.yy0.z) + yymsp[0].minor.yy0.n;
}
  yymsp[-1].minor.yy0 = yylhsminor.yy0;
        break;
      case 339: /* cmd ::= ANALYZE */
{
    yymsp[0].minor.yy213 = synq_parse_analyze_or_reindex_stmt(pCtx,
        SYNQ_NO_SPAN,
        SYNQ_NO_SPAN,
        SYNTAQLITE_ANALYZE_OR_REINDEX_OP_ANALYZE);
}
        break;
      case 340: /* cmd ::= ANALYZE nm dbnm */
{
    SyntaqliteSourceSpan name_span = yymsp[0].minor.yy0.z ? synq_span(pCtx, yymsp[0].minor.yy0) : synq_span(pCtx, yymsp[-1].minor.yy0);
    SyntaqliteSourceSpan schema_span = yymsp[0].minor.yy0.z ? synq_span(pCtx, yymsp[-1].minor.yy0) : SYNQ_NO_SPAN;
    yymsp[-2].minor.yy213 = synq_parse_analyze_or_reindex_stmt(pCtx, name_span, schema_span, SYNTAQLITE_ANALYZE_OR_REINDEX_OP_ANALYZE);
}
        break;
      case 341: /* cmd ::= REINDEX */
{
    yymsp[0].minor.yy213 = synq_parse_analyze_or_reindex_stmt(pCtx,
        SYNQ_NO_SPAN,
        SYNQ_NO_SPAN,
        SYNTAQLITE_ANALYZE_OR_REINDEX_OP_REINDEX);
}
        break;
      case 342: /* cmd ::= REINDEX nm dbnm */
{
    SyntaqliteSourceSpan name_span = yymsp[0].minor.yy0.z ? synq_span(pCtx, yymsp[0].minor.yy0) : synq_span(pCtx, yymsp[-1].minor.yy0);
    SyntaqliteSourceSpan schema_span = yymsp[0].minor.yy0.z ? synq_span(pCtx, yymsp[-1].minor.yy0) : SYNQ_NO_SPAN;
    yymsp[-2].minor.yy213 = synq_parse_analyze_or_reindex_stmt(pCtx, name_span, schema_span, 1);
}
        break;
      case 343: /* cmd ::= ATTACH database_kw_opt expr AS expr key_opt */
{
    yymsp[-5].minor.yy213 = synq_parse_attach_stmt(pCtx, yymsp[-3].minor.yy213, yymsp[-1].minor.yy213, yymsp[0].minor.yy213);
}
        break;
      case 344: /* cmd ::= DETACH database_kw_opt expr */
{
    yymsp[-2].minor.yy213 = synq_parse_detach_stmt(pCtx, yymsp[0].minor.yy213);
}
        break;
      case 345: /* database_kw_opt ::= DATABASE */
{
    // Keyword consumed, no value needed
}
        break;
      case 346: /* database_kw_opt ::= */
{
    // Empty
}
        break;
      case 349: /* cmd ::= VACUUM vinto */
{
    yymsp[-1].minor.yy213 = synq_parse_vacuum_stmt(pCtx,
        SYNQ_NO_SPAN,
        yymsp[0].minor.yy213);
}
        break;
      case 350: /* cmd ::= VACUUM nm vinto */
{
    yymsp[-2].minor.yy213 = synq_parse_vacuum_stmt(pCtx,
        synq_span(pCtx, yymsp[-1].minor.yy0),
        yymsp[0].minor.yy213);
}
        break;
      case 353: /* ecmd ::= explain cmdx SEMI */
{
    yylhsminor.yy213 = synq_parse_explain_stmt(pCtx, (SyntaqliteExplainMode)(yymsp[-2].minor.yy220 - 1), yymsp[-1].minor.yy213);
    pCtx->root = yylhsminor.yy213;
    synq_parse_list_flush(pCtx);
    pCtx->stmt_completed = 1;
}
  yymsp[-2].minor.yy213 = yylhsminor.yy213;
        break;
      case 355: /* explain ::= EXPLAIN QUERY PLAN */
{
    yymsp[-2].minor.yy220 = 2;
}
        break;
      case 356: /* cmd ::= createkw uniqueflag INDEX ifnotexists nm dbnm ON nm LP sortlist RP where_opt */
{
    SyntaqliteSourceSpan idx_name = yymsp[-6].minor.yy0.z ? synq_span(pCtx, yymsp[-6].minor.yy0) : synq_span(pCtx, yymsp[-7].minor.yy0);
    SyntaqliteSourceSpan idx_schema = yymsp[-6].minor.yy0.z ? synq_span(pCtx, yymsp[-7].minor.yy0) : SYNQ_NO_SPAN;
    yymsp[-11].minor.yy213 = synq_parse_create_index_stmt(pCtx,
        idx_name,
        idx_schema,
        synq_span(pCtx, yymsp[-4].minor.yy0),
        (SyntaqliteBool)yymsp[-10].minor.yy220,
        (SyntaqliteBool)yymsp[-8].minor.yy220,
        yymsp[-2].minor.yy213,
        yymsp[0].minor.yy213);
}
        break;
      case 360: /* ifnotexists ::= IF NOT EXISTS */
{
    yymsp[-2].minor.yy220 = 1;
}
        break;
      case 361: /* cmd ::= createkw temp VIEW ifnotexists nm dbnm eidlist_opt AS select */
{
    SyntaqliteSourceSpan view_name = yymsp[-3].minor.yy0.z ? synq_span(pCtx, yymsp[-3].minor.yy0) : synq_span(pCtx, yymsp[-4].minor.yy0);
    SyntaqliteSourceSpan view_schema = yymsp[-3].minor.yy0.z ? synq_span(pCtx, yymsp[-4].minor.yy0) : SYNQ_NO_SPAN;
    yymsp[-8].minor.yy213 = synq_parse_create_view_stmt(pCtx,
        view_name,
        view_schema,
        (SyntaqliteBool)yymsp[-7].minor.yy220,
        (SyntaqliteBool)yymsp[-5].minor.yy220,
        yymsp[-2].minor.yy213,
        yymsp[0].minor.yy213);
}
        break;
      case 365: /* values ::= VALUES LP nexprlist RP */
{
    yymsp[-3].minor.yy213 = synq_parse_values_row_list(pCtx, SYNTAQLITE_NULL_NODE, yymsp[-1].minor.yy213);
}
        break;
      case 366: /* mvalues ::= values COMMA LP nexprlist RP */
      case 367: /* mvalues ::= mvalues COMMA LP nexprlist RP */ yytestcase(yyruleno==367);
{
    yymsp[-4].minor.yy213 = synq_parse_values_row_list(pCtx, yymsp[-4].minor.yy213, yymsp[-1].minor.yy213);
}
        break;
      case 368: /* oneselect ::= values */
      case 369: /* oneselect ::= mvalues */ yytestcase(yyruleno==369);
{
    yylhsminor.yy213 = synq_parse_values_clause(pCtx, yymsp[0].minor.yy213);
}
  yymsp[0].minor.yy213 = yylhsminor.yy213;
        break;
      case 371: /* cmd ::= create_vtab LP vtabarglist RP */
{
    // Capture module arguments span (content between parens)
    SyntaqliteNode *vtab = AST_NODE(&pCtx->ast, yymsp[-3].minor.yy213);
    const char *args_start = yymsp[-2].minor.yy0.z + yymsp[-2].minor.yy0.n;
    const char *args_end = yymsp[0].minor.yy0.z;
    vtab->create_virtual_table_stmt.module_args = (SyntaqliteSourceSpan){
        (uint32_t)(args_start - pCtx->source),
        (uint16_t)(args_end - args_start)
    };
    yylhsminor.yy213 = yymsp[-3].minor.yy213;
}
  yymsp[-3].minor.yy213 = yylhsminor.yy213;
        break;
      case 372: /* create_vtab ::= createkw VIRTUAL TABLE ifnotexists nm dbnm USING nm */
{
    SyntaqliteSourceSpan tbl_name = yymsp[-2].minor.yy0.z ? synq_span(pCtx, yymsp[-2].minor.yy0) : synq_span(pCtx, yymsp[-3].minor.yy0);
    SyntaqliteSourceSpan tbl_schema = yymsp[-2].minor.yy0.z ? synq_span(pCtx, yymsp[-3].minor.yy0) : SYNQ_NO_SPAN;
    yymsp[-7].minor.yy213 = synq_parse_create_virtual_table_stmt(pCtx,
        tbl_name,
        tbl_schema,
        synq_span(pCtx, yymsp[0].minor.yy0),
        (SyntaqliteBool)yymsp[-4].minor.yy220,
        SYNQ_NO_SPAN);  // module_args = none by default
}
        break;
      case 383: /* windowdefn_list ::= windowdefn */
{
    yylhsminor.yy213 = synq_parse_named_window_def_list(pCtx, SYNTAQLITE_NULL_NODE, yymsp[0].minor.yy213);
}
  yymsp[0].minor.yy213 = yylhsminor.yy213;
        break;
      case 384: /* windowdefn_list ::= windowdefn_list COMMA windowdefn */
{
    yylhsminor.yy213 = synq_parse_named_window_def_list(pCtx, yymsp[-2].minor.yy213, yymsp[0].minor.yy213);
}
  yymsp[-2].minor.yy213 = yylhsminor.yy213;
        break;
      case 385: /* windowdefn ::= nm AS LP window RP */
{
    yylhsminor.yy213 = synq_parse_named_window_def(pCtx,
        synq_span(pCtx, yymsp[-4].minor.yy0),
        yymsp[-1].minor.yy213);
}
  yymsp[-4].minor.yy213 = yylhsminor.yy213;
        break;
      case 386: /* window ::= PARTITION BY nexprlist orderby_opt frame_opt */
{
    yymsp[-4].minor.yy213 = synq_parse_window_def(pCtx,
        SYNQ_NO_SPAN,
        yymsp[-2].minor.yy213,
        yymsp[-1].minor.yy213,
        yymsp[0].minor.yy213);
}
        break;
      case 387: /* window ::= nm PARTITION BY nexprlist orderby_opt frame_opt */
{
    yylhsminor.yy213 = synq_parse_window_def(pCtx,
        synq_span(pCtx, yymsp[-5].minor.yy0),
        yymsp[-2].minor.yy213,
        yymsp[-1].minor.yy213,
        yymsp[0].minor.yy213);
}
  yymsp[-5].minor.yy213 = yylhsminor.yy213;
        break;
      case 388: /* window ::= ORDER BY sortlist frame_opt */
{
    yymsp[-3].minor.yy213 = synq_parse_window_def(pCtx,
        SYNQ_NO_SPAN,
        SYNTAQLITE_NULL_NODE,
        yymsp[-1].minor.yy213,
        yymsp[0].minor.yy213);
}
        break;
      case 389: /* window ::= nm ORDER BY sortlist frame_opt */
{
    yylhsminor.yy213 = synq_parse_window_def(pCtx,
        synq_span(pCtx, yymsp[-4].minor.yy0),
        SYNTAQLITE_NULL_NODE,
        yymsp[-1].minor.yy213,
        yymsp[0].minor.yy213);
}
  yymsp[-4].minor.yy213 = yylhsminor.yy213;
        break;
      case 390: /* window ::= frame_opt */
{
    yylhsminor.yy213 = synq_parse_window_def(pCtx,
        SYNQ_NO_SPAN,
        SYNTAQLITE_NULL_NODE,
        SYNTAQLITE_NULL_NODE,
        yymsp[0].minor.yy213);
}
  yymsp[0].minor.yy213 = yylhsminor.yy213;
        break;
      case 391: /* window ::= nm frame_opt */
{
    yylhsminor.yy213 = synq_parse_window_def(pCtx,
        synq_span(pCtx, yymsp[-1].minor.yy0),
        SYNTAQLITE_NULL_NODE,
        SYNTAQLITE_NULL_NODE,
        yymsp[0].minor.yy213);
}
  yymsp[-1].minor.yy213 = yylhsminor.yy213;
        break;
      case 393: /* frame_opt ::= range_or_rows frame_bound_s frame_exclude_opt */
{
    // Single bound: start=yymsp[-1].minor.yy213, end=CURRENT ROW (implicit)
    uint32_t end_bound = synq_parse_frame_bound(pCtx,
        SYNTAQLITE_FRAME_BOUND_TYPE_CURRENT_ROW,
        SYNTAQLITE_NULL_NODE);
    yylhsminor.yy213 = synq_parse_frame_spec(pCtx,
        (SyntaqliteFrameType)yymsp[-2].minor.yy220,
        (SyntaqliteFrameExclude)yymsp[0].minor.yy220,
        yymsp[-1].minor.yy213,
        end_bound);
}
  yymsp[-2].minor.yy213 = yylhsminor.yy213;
        break;
      case 394: /* frame_opt ::= range_or_rows BETWEEN frame_bound_s AND frame_bound_e frame_exclude_opt */
{
    yylhsminor.yy213 = synq_parse_frame_spec(pCtx,
        (SyntaqliteFrameType)yymsp[-5].minor.yy220,
        (SyntaqliteFrameExclude)yymsp[0].minor.yy220,
        yymsp[-3].minor.yy213,
        yymsp[-1].minor.yy213);
}
  yymsp[-5].minor.yy213 = yylhsminor.yy213;
        break;
      case 395: /* range_or_rows ::= RANGE|ROWS|GROUPS */
{
    switch (yymsp[0].minor.yy0.type) {
        case SYNTAQLITE_TK_RANGE:  yylhsminor.yy220 = SYNTAQLITE_FRAME_TYPE_RANGE; break;
        case SYNTAQLITE_TK_ROWS:   yylhsminor.yy220 = SYNTAQLITE_FRAME_TYPE_ROWS; break;
        default:        yylhsminor.yy220 = SYNTAQLITE_FRAME_TYPE_GROUPS; break;
    }
}
  yymsp[0].minor.yy220 = yylhsminor.yy220;
        break;
      case 397: /* frame_bound_s ::= UNBOUNDED PRECEDING */
{
    yymsp[-1].minor.yy213 = synq_parse_frame_bound(pCtx,
        SYNTAQLITE_FRAME_BOUND_TYPE_UNBOUNDED_PRECEDING,
        SYNTAQLITE_NULL_NODE);
}
        break;
      case 399: /* frame_bound_e ::= UNBOUNDED FOLLOWING */
{
    yymsp[-1].minor.yy213 = synq_parse_frame_bound(pCtx,
        SYNTAQLITE_FRAME_BOUND_TYPE_UNBOUNDED_FOLLOWING,
        SYNTAQLITE_NULL_NODE);
}
        break;
      case 400: /* frame_bound ::= expr PRECEDING|FOLLOWING */
{
    SyntaqliteFrameBoundType bt = (yymsp[0].minor.yy0.type == SYNTAQLITE_TK_PRECEDING)
        ? SYNTAQLITE_FRAME_BOUND_TYPE_EXPR_PRECEDING
        : SYNTAQLITE_FRAME_BOUND_TYPE_EXPR_FOLLOWING;
    yylhsminor.yy213 = synq_parse_frame_bound(pCtx, bt, yymsp[-1].minor.yy213);
}
  yymsp[-1].minor.yy213 = yylhsminor.yy213;
        break;
      case 401: /* frame_bound ::= CURRENT ROW */
{
    yymsp[-1].minor.yy213 = synq_parse_frame_bound(pCtx,
        SYNTAQLITE_FRAME_BOUND_TYPE_CURRENT_ROW,
        SYNTAQLITE_NULL_NODE);
}
        break;
      case 402: /* frame_exclude_opt ::= */
{
    yymsp[1].minor.yy220 = SYNTAQLITE_FRAME_EXCLUDE_NONE;
}
        break;
      case 404: /* frame_exclude ::= NO OTHERS */
{
    yymsp[-1].minor.yy220 = SYNTAQLITE_FRAME_EXCLUDE_NO_OTHERS;
}
        break;
      case 405: /* frame_exclude ::= CURRENT ROW */
{
    yymsp[-1].minor.yy220 = SYNTAQLITE_FRAME_EXCLUDE_CURRENT_ROW;
}
        break;
      case 406: /* frame_exclude ::= GROUP|TIES */
{
    yylhsminor.yy220 = (yymsp[0].minor.yy0.type == SYNTAQLITE_TK_GROUP)
        ? SYNTAQLITE_FRAME_EXCLUDE_GROUP
        : SYNTAQLITE_FRAME_EXCLUDE_TIES;
}
  yymsp[0].minor.yy220 = yylhsminor.yy220;
        break;
      case 408: /* filter_over ::= filter_clause over_clause */
{
    // Unpack the over_clause FilterOver to combine with filter expr
    SyntaqliteFilterOver *fo_over = (SyntaqliteFilterOver*)synq_arena_ptr(&pCtx->ast, yymsp[0].minor.yy213);
    yylhsminor.yy213 = synq_parse_filter_over(pCtx,
        yymsp[-1].minor.yy213,
        fo_over->over_def,
        SYNQ_NO_SPAN);
}
  yymsp[-1].minor.yy213 = yylhsminor.yy213;
        break;
      case 410: /* filter_over ::= filter_clause */
{
    yylhsminor.yy213 = synq_parse_filter_over(pCtx,
        yymsp[0].minor.yy213,
        SYNTAQLITE_NULL_NODE,
        SYNQ_NO_SPAN);
}
  yymsp[0].minor.yy213 = yylhsminor.yy213;
        break;
      case 411: /* over_clause ::= OVER LP window RP */
{
    yymsp[-3].minor.yy213 = synq_parse_filter_over(pCtx,
        SYNTAQLITE_NULL_NODE,
        yymsp[-1].minor.yy213,
        SYNQ_NO_SPAN);
}
        break;
      case 412: /* over_clause ::= OVER nm */
{
    // Create a WindowDef with just base_window_name to represent a named window ref
    uint32_t wdef = synq_parse_window_def(pCtx,
        synq_span(pCtx, yymsp[0].minor.yy0),
        SYNTAQLITE_NULL_NODE,
        SYNTAQLITE_NULL_NODE,
        SYNTAQLITE_NULL_NODE);
    yymsp[-1].minor.yy213 = synq_parse_filter_over(pCtx,
        SYNTAQLITE_NULL_NODE,
        wdef,
        SYNQ_NO_SPAN);
}
        break;
      case 413: /* filter_clause ::= FILTER LP WHERE expr RP */
{
    yymsp[-4].minor.yy213 = yymsp[-1].minor.yy213;
}
        break;
      default:
        break;
/********** End reduce actions ************************************************/
  };
  assert( yyruleno<sizeof(yyRuleInfoLhs)/sizeof(yyRuleInfoLhs[0]) );
  yygoto = yyRuleInfoLhs[yyruleno];
  yysize = yyRuleInfoNRhs[yyruleno];
  yyact = yy_find_reduce_action(yymsp[yysize].stateno,(YYCODETYPE)yygoto);

  /* There are no SHIFTREDUCE actions on nonterminals because the table
  ** generator has simplified them to pure REDUCE actions. */
  assert( !(yyact>YY_MAX_SHIFT && yyact<=YY_MAX_SHIFTREDUCE) );

  /* It is not possible for a REDUCE to be followed by an error */
  assert( yyact!=YY_ERROR_ACTION );

  yymsp += yysize+1;
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
static void yy_parse_failed(
  yyParser *yypParser           /* The parser */
){
  SynqSqliteParseARG_FETCH
  SynqSqliteParseCTX_FETCH
#ifndef NDEBUG
  if( yyTraceFILE ){
    fprintf(yyTraceFILE,"%sFail!\n",yyTracePrompt);
  }
#endif
  while( yypParser->yytos>yypParser->yystack ) yy_pop_parser_stack(yypParser);
  /* Here code is inserted which will be executed whenever the
  ** parser fails */
/************ Begin %parse_failure code ***************************************/

    if (pCtx) {
        pCtx->error = 1;
    }
/************ End %parse_failure code *****************************************/
  SynqSqliteParseARG_STORE /* Suppress warning about unused %extra_argument variable */
  SynqSqliteParseCTX_STORE
}
#endif /* YYNOERRORRECOVERY */

/*
** The following code executes when a syntax error first occurs.
*/
static void yy_syntax_error(
  yyParser *yypParser,           /* The parser */
  int yymajor,                   /* The major type of the error token */
  SynqSqliteParseTOKENTYPE yyminor         /* The minor type of the error token */
){
  SynqSqliteParseARG_FETCH
  SynqSqliteParseCTX_FETCH
#define TOKEN yyminor
/************ Begin %syntax_error code ****************************************/

  (void)yymajor;
  (void)TOKEN;
  if (pCtx) {
    pCtx->error = 1;
  }
/************ End %syntax_error code ******************************************/
  SynqSqliteParseARG_STORE /* Suppress warning about unused %extra_argument variable */
  SynqSqliteParseCTX_STORE
}

/*
** The following is executed when the parser accepts
*/
static void yy_accept(
  yyParser *yypParser           /* The parser */
){
  SynqSqliteParseARG_FETCH
  SynqSqliteParseCTX_FETCH
#ifndef NDEBUG
  if( yyTraceFILE ){
    fprintf(yyTraceFILE,"%sAccept!\n",yyTracePrompt);
  }
#endif
#ifndef YYNOERRORRECOVERY
  yypParser->yyerrcnt = -1;
#endif
  assert( yypParser->yytos==yypParser->yystack );
  /* Here code is inserted which will be executed whenever the
  ** parser accepts */
/*********** Begin %parse_accept code *****************************************/
/*********** End %parse_accept code *******************************************/
  SynqSqliteParseARG_STORE /* Suppress warning about unused %extra_argument variable */
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
  void *yyp,                   /* The parser */
  int yymajor,                 /* The major token code number */
  SynqSqliteParseTOKENTYPE yyminor       /* The value for the token */
  SynqSqliteParseARG_PDECL               /* Optional %extra_argument parameter */
){
  YYMINORTYPE yyminorunion;
  YYACTIONTYPE yyact;   /* The parser action. */
#if !defined(YYERRORSYMBOL) && !defined(YYNOERRORRECOVERY)
  int yyendofinput;     /* True if we are at the end of input */
#endif
#ifdef YYERRORSYMBOL
  int yyerrorhit = 0;   /* True if yymajor has invoked an error */
#endif
  yyParser *yypParser = (yyParser*)yyp;  /* The parser */
  SynqSqliteParseCTX_FETCH
  SynqSqliteParseARG_STORE

  assert( yypParser->yytos!=0 );
#if !defined(YYERRORSYMBOL) && !defined(YYNOERRORRECOVERY)
  yyendofinput = (yymajor==0);
#endif

  yyact = yypParser->yytos->stateno;
#ifndef NDEBUG
  if( yyTraceFILE ){
    if( yyact < YY_MIN_REDUCE ){
      fprintf(yyTraceFILE,"%sInput '%s' in state %d\n",
              yyTracePrompt,yyTokenName[yymajor],yyact);
    }else{
      fprintf(yyTraceFILE,"%sInput '%s' with pending reduce %d\n",
              yyTracePrompt,yyTokenName[yymajor],yyact-YY_MIN_REDUCE);
    }
  }
#endif

  while(1){ /* Exit by "break" */
    assert( yypParser->yytos>=yypParser->yystack );
    assert( yyact==yypParser->yytos->stateno );
    yyact = yy_find_shift_action((YYCODETYPE)yymajor,yyact);
    if( yyact >= YY_MIN_REDUCE ){
      unsigned int yyruleno = yyact - YY_MIN_REDUCE; /* Reduce by this rule */
#ifndef NDEBUG
      assert( yyruleno<(int)(sizeof(yyRuleName)/sizeof(yyRuleName[0])) );
      if( yyTraceFILE ){
        int yysize = yyRuleInfoNRhs[yyruleno];
        if( yysize ){
          fprintf(yyTraceFILE, "%sReduce %d [%s]%s, pop back to state %d.\n",
            yyTracePrompt,
            yyruleno, yyRuleName[yyruleno],
            yyruleno<YYNRULE_WITH_ACTION ? "" : " without external action",
            yypParser->yytos[yysize].stateno);
        }else{
          fprintf(yyTraceFILE, "%sReduce %d [%s]%s.\n",
            yyTracePrompt, yyruleno, yyRuleName[yyruleno],
            yyruleno<YYNRULE_WITH_ACTION ? "" : " without external action");
        }
      }
#endif /* NDEBUG */

      /* Check that the stack is large enough to grow by a single entry
      ** if the RHS of the rule is empty.  This ensures that there is room
      ** enough on the stack to push the LHS value */
      if( yyRuleInfoNRhs[yyruleno]==0 ){
#ifdef YYTRACKMAXSTACKDEPTH
        if( (int)(yypParser->yytos - yypParser->yystack)>yypParser->yyhwm ){
          yypParser->yyhwm++;
          assert( yypParser->yyhwm ==
                  (int)(yypParser->yytos - yypParser->yystack));
        }
#endif
        if( yypParser->yytos>=yypParser->yystackEnd ){
          if( yyGrowStack(yypParser) ){
            yyStackOverflow(yypParser);
            break;
          }
        }
      }
      yyact = yy_reduce(yypParser,yyruleno,yymajor,yyminor SynqSqliteParseCTX_PARAM);
    }else if( yyact <= YY_MAX_SHIFTREDUCE ){
      yy_shift(yypParser,yyact,(YYCODETYPE)yymajor,yyminor);
#ifndef YYNOERRORRECOVERY
      yypParser->yyerrcnt--;
#endif
      break;
    }else if( yyact==YY_ACCEPT_ACTION ){
      yypParser->yytos--;
      yy_accept(yypParser);
      return;
    }else{
      assert( yyact == YY_ERROR_ACTION );
      yyminorunion.yy0 = yyminor;
#ifdef YYERRORSYMBOL
      int yymx;
#endif
#ifndef NDEBUG
      if( yyTraceFILE ){
        fprintf(yyTraceFILE,"%sSyntax Error!\n",yyTracePrompt);
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
      if( yypParser->yyerrcnt<0 ){
        yy_syntax_error(yypParser,yymajor,yyminor);
      }
      yymx = yypParser->yytos->major;
      if( yymx==YYERRORSYMBOL || yyerrorhit ){
#ifndef NDEBUG
        if( yyTraceFILE ){
          fprintf(yyTraceFILE,"%sDiscard input token %s\n",
             yyTracePrompt,yyTokenName[yymajor]);
        }
#endif
        yy_destructor(yypParser, (YYCODETYPE)yymajor, &yyminorunion);
        yymajor = YYNOCODE;
      }else{
        while( yypParser->yytos > yypParser->yystack ){
          yyact = yy_find_reduce_action(yypParser->yytos->stateno,
                                        YYERRORSYMBOL);
          if( yyact<=YY_MAX_SHIFTREDUCE ) break;
          yy_pop_parser_stack(yypParser);
        }
        if( yypParser->yytos <= yypParser->yystack || yymajor==0 ){
          yy_destructor(yypParser,(YYCODETYPE)yymajor,&yyminorunion);
          yy_parse_failed(yypParser);
#ifndef YYNOERRORRECOVERY
          yypParser->yyerrcnt = -1;
#endif
          yymajor = YYNOCODE;
        }else if( yymx!=YYERRORSYMBOL ){
          yy_shift(yypParser,yyact,YYERRORSYMBOL,yyminor);
        }
      }
      yypParser->yyerrcnt = 3;
      yyerrorhit = 1;
      if( yymajor==YYNOCODE ) break;
      yyact = yypParser->yytos->stateno;
#elif defined(YYNOERRORRECOVERY)
      /* If the YYNOERRORRECOVERY macro is defined, then do not attempt to
      ** do any kind of error recovery.  Instead, simply invoke the syntax
      ** error routine and continue going as if nothing had happened.
      **
      ** Applications can set this macro (for example inside %include) if
      ** they intend to abandon the parse upon the first syntax error seen.
      */
      yy_syntax_error(yypParser,yymajor, yyminor);
      yy_destructor(yypParser,(YYCODETYPE)yymajor,&yyminorunion);
      break;
#else  /* YYERRORSYMBOL is not defined */
      /* This is what we do if the grammar does not define ERROR:
      **
      **  * Report an error message, and throw away the input token.
      **
      **  * If the input token is $, then fail the parse.
      **
      ** As before, subsequent error messages are suppressed until
      ** three input tokens have been successfully shifted.
      */
      if( yypParser->yyerrcnt<=0 ){
        yy_syntax_error(yypParser,yymajor, yyminor);
      }
      yypParser->yyerrcnt = 3;
      yy_destructor(yypParser,(YYCODETYPE)yymajor,&yyminorunion);
      if( yyendofinput ){
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
  if( yyTraceFILE ){
    yyStackEntry *i;
    char cDiv = '[';
    fprintf(yyTraceFILE,"%sReturn. Stack=",yyTracePrompt);
    for(i=&yypParser->yystack[1]; i<=yypParser->yytos; i++){
      fprintf(yyTraceFILE,"%c%s", cDiv, yyTokenName[i->major]);
      cDiv = ' ';
    }
    fprintf(yyTraceFILE,"]\n");
  }
#endif
  return;
}

/*
** Return the fallback token corresponding to canonical token iToken, or
** 0 if iToken has no fallback.
*/
int SynqSqliteParseFallback(int iToken){
#ifdef YYFALLBACK
  assert( iToken<(int)(sizeof(yyFallback)/sizeof(yyFallback[0])) );
  return yyFallback[iToken];
#else
  (void)iToken;
  return 0;
#endif
}

/* syntaqlite extension: enumerate terminals that can be shifted/reduced from
** the parser's current state. Returns the total number of expected tokens,
** even when out_tokens/out_cap only request a prefix. */
static YYACTIONTYPE synq_find_reduce_action_safe(YYACTIONTYPE stateno, YYCODETYPE iLookAhead) {
int i;
if( stateno>YY_REDUCE_COUNT ) return yy_default[stateno];
i = yy_reduce_ofst[stateno] + iLookAhead;
if( i<0 || i>=YY_ACTTAB_COUNT || yy_lookahead[i]!=iLookAhead ) {
return yy_default[stateno];
}
return yy_action[i];
}

/* Like yy_find_shift_action but skips YYWILDCARD and YYFALLBACK paths.
** Wildcard matches are for error recovery (ANY token) and fallback matches
** accept keywords as identifiers — neither should appear as keyword
** autocompletion suggestions. */
static YYACTIONTYPE synq_find_shift_action_strict(
YYCODETYPE iLookAhead,
YYACTIONTYPE stateno
){
int i;
if( stateno>YY_MAX_SHIFT ) return stateno;
i = yy_shift_ofst[stateno];
assert( i>=0 );
assert( i+YYNTOKEN<=(int)YY_NLOOKAHEAD );
i += iLookAhead;
if( yy_lookahead[i]!=iLookAhead ){
/* No specific entry — skip fallback and wildcard, use default. */
return yy_default[stateno];
}
return yy_action[i];
}

static int synq_can_lookahead(yyParser* p, uint32_t token) {
YYACTIONTYPE stack_states[YYSTACKDEPTH + 1];
int top = 0;
int i = 0;
int steps = 0;

if( p==0 || p->yytos==0 ) return 0;

top = (int)(p->yytos - p->yystack);
if( top<0 || top>YYSTACKDEPTH ) return 0;
for(i=0; i<=top; i++) {
stack_states[i] = p->yystack[i].stateno;
}

while( steps++ < 10000 ) {
YYACTIONTYPE action = synq_find_shift_action_strict((YYCODETYPE)token, stack_states[top]);

if( action==YY_ERROR_ACTION || action==YY_NO_ACTION ) return 0;
if( action==YY_ACCEPT_ACTION ) return token==0;
if( action<=YY_MAX_SHIFT ) return 1;

/* Shift-reduce: the token is consumed (shifted) then a reduce follows.
** This means the token IS accepted, same as a pure shift. */
if( action>=YY_MIN_SHIFTREDUCE && action<=YY_MAX_SHIFTREDUCE ) return 1;

if( action>=YY_MIN_REDUCE && action<=YY_MAX_REDUCE ) {
int rule = (int)(action - YY_MIN_REDUCE);
int yysize = yyRuleInfoNRhs[rule];
YYACTIONTYPE goto_state;

top += yysize;  /* yyRuleInfoNRhs is negative rhs-size */
if( top<0 ) return 0;

goto_state = synq_find_reduce_action_safe(stack_states[top], yyRuleInfoLhs[rule]);
if( goto_state==YY_ERROR_ACTION || goto_state==YY_NO_ACTION ) return 0;

if( top>=YYSTACKDEPTH ) return 0;
top++;
stack_states[top] = goto_state;
continue;
}

return 0;
}

return 0;
}

uint32_t SynqSqliteParseExpectedTokens(void* parser, uint32_t* out_tokens, uint32_t out_cap) {
uint32_t n = 0;
uint32_t token = 0;
yyParser* p = (yyParser*)parser;

if( p==0 || p->yytos==0 ) return 0;

for(token=1; token<YYNTOKEN; token++) {
if( !synq_can_lookahead(p, token) ) continue;
if( out_tokens && n<out_cap ) out_tokens[n] = token;
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
if( state>YY_REDUCE_COUNT ) return 0;
i = yy_reduce_ofst[state] + nt;
if( i<0 || i>=YY_ACTTAB_COUNT ) return 0;
return yy_lookahead[i] == nt;
}

/* syntaqlite extension: determine the semantic completion context
** (Expression vs TableRef) by walking the parser stack. Returns one of
** SYNTAQLITE_COMPLETION_CONTEXT_*. */
uint32_t SynqSqliteParseCompletionContext(void* parser) {
yyParser* p = (yyParser*)parser;
if( p==0 || p->yytos==0 ) return SYNTAQLITE_COMPLETION_CONTEXT_UNKNOWN;

for(yyStackEntry* e = p->yytos; e >= p->yystack; e--) {
YYACTIONTYPE s = e->stateno;

/* Check if this state has gotos for table-ref non-terminals. */
if( synq_has_goto(s, SYNQ_NT_SELTABLIST)
|| synq_has_goto(s, SYNQ_NT_FULLNAME)
|| synq_has_goto(s, SYNQ_NT_XFULLNAME) ) {
return SYNTAQLITE_COMPLETION_CONTEXT_TABLE_REF;
}

/* Check if this state has gotos for expression non-terminals. */
if( synq_has_goto(s, SYNQ_NT_EXPR) ) {
return SYNTAQLITE_COMPLETION_CONTEXT_EXPRESSION;
}
}
return SYNTAQLITE_COMPLETION_CONTEXT_UNKNOWN;
}
