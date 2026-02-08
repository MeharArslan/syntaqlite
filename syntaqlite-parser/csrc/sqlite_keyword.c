#include "csrc/sqlite_compat.h"
#include "csrc/sqlite_keyword_tables.h"

const unsigned char sqlite3UpperToLower[] = {
#ifdef SQLITE_ASCII
      0,  1,  2,  3,  4,  5,  6,  7,  8,  9, 10, 11, 12, 13, 14, 15, 16, 17,
     18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35,
     36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53,
     54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 97, 98, 99,100,101,102,103,
    104,105,106,107,108,109,110,111,112,113,114,115,116,117,118,119,120,121,
    122, 91, 92, 93, 94, 95, 96, 97, 98, 99,100,101,102,103,104,105,106,107,
    108,109,110,111,112,113,114,115,116,117,118,119,120,121,122,123,124,125,
    126,127,128,129,130,131,132,133,134,135,136,137,138,139,140,141,142,143,
    144,145,146,147,148,149,150,151,152,153,154,155,156,157,158,159,160,161,
    162,163,164,165,166,167,168,169,170,171,172,173,174,175,176,177,178,179,
    180,181,182,183,184,185,186,187,188,189,190,191,192,193,194,195,196,197,
    198,199,200,201,202,203,204,205,206,207,208,209,210,211,212,213,214,215,
    216,217,218,219,220,221,222,223,224,225,226,227,228,229,230,231,232,233,
    234,235,236,237,238,239,240,241,242,243,244,245,246,247,248,249,250,251,
    252,253,254,255,
#endif
#ifdef SQLITE_EBCDIC
      0,  1,  2,  3,  4,  5,  6,  7,  8,  9, 10, 11, 12, 13, 14, 15, /* 0x */
     16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, /* 1x */
     32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, /* 2x */
     48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, /* 3x */
     64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, /* 4x */
     80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, /* 5x */
     96, 97, 98, 99,100,101,102,103,104,105,106,107,108,109,110,111, /* 6x */
    112,113,114,115,116,117,118,119,120,121,122,123,124,125,126,127, /* 7x */
    128,129,130,131,132,133,134,135,136,137,138,139,140,141,142,143, /* 8x */
    144,145,146,147,148,149,150,151,152,153,154,155,156,157,158,159, /* 9x */
    160,161,162,163,164,165,166,167,168,169,170,171,140,141,142,175, /* Ax */
    176,177,178,179,180,181,182,183,184,185,186,187,188,189,190,191, /* Bx */
    192,129,130,131,132,133,134,135,136,137,202,203,204,205,206,207, /* Cx */
    208,145,146,147,148,149,150,151,152,153,218,219,220,221,222,223, /* Dx */
    224,225,162,163,164,165,166,167,168,169,234,235,236,237,238,239, /* Ex */
    240,241,242,243,244,245,246,247,248,249,250,251,252,253,254,255, /* Fx */
#endif
/* All of the upper-to-lower conversion data is above.  The following
** 18 integers are completely unrelated.  They are appended to the
** sqlite3UpperToLower[] array to avoid UBSAN warnings.  Here's what is
** going on:
**
** The SQL comparison operators (<>, =, >, <=, <, and >=) are implemented
** by invoking sqlite3MemCompare(A,B) which compares values A and B and
** returns negative, zero, or positive if A is less then, equal to, or
** greater than B, respectively.  Then the true false results is found by
** consulting sqlite3aLTb[opcode], sqlite3aEQb[opcode], or 
** sqlite3aGTb[opcode] depending on whether the result of compare(A,B)
** is negative, zero, or positive, where opcode is the specific opcode.
** The only works because the comparison opcodes are consecutive and in
** this order: NE EQ GT LE LT GE.  Various assert()s throughout the code
** ensure that is the case.
**
** These elements must be appended to another array.  Otherwise the
** index (here shown as [256-OP_Ne]) would be out-of-bounds and thus
** be undefined behavior.  That's goofy, but the C-standards people thought
** it was a good idea, so here we are.
*/
/* NE  EQ  GT  LE  LT  GE  */
   1,  0,  0,  1,  1,  0,  /* aLTb[]: Use when compare(A,B) less than zero */
   0,  1,  0,  1,  0,  1,  /* aEQb[]: Use when compare(A,B) equals zero */
   1,  0,  1,  0,  0,  1   /* aGTb[]: Use when compare(A,B) greater than zero*/
};

#ifdef SQLITE_ASCII
# define charMap(X) sqlite3UpperToLower[(unsigned char)X]
#endif
#ifdef SQLITE_EBCDIC
# define charMap(X) ebcdicToAscii[(unsigned char)X]
const unsigned char ebcdicToAscii[] = {
/* 0   1   2   3   4   5   6   7   8   9   A   B   C   D   E   F */
   0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  /* 0x */
   0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  /* 1x */
   0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  /* 2x */
   0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  /* 3x */
   0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  /* 4x */
   0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  /* 5x */
   0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0, 95,  0,  0,  /* 6x */
   0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  /* 7x */
   0, 97, 98, 99,100,101,102,103,104,105,  0,  0,  0,  0,  0,  0,  /* 8x */
   0,106,107,108,109,110,111,112,113,114,  0,  0,  0,  0,  0,  0,  /* 9x */
   0,  0,115,116,117,118,119,120,121,122,  0,  0,  0,  0,  0,  0,  /* Ax */
   0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  /* Bx */
   0, 97, 98, 99,100,101,102,103,104,105,  0,  0,  0,  0,  0,  0,  /* Cx */
   0,106,107,108,109,110,111,112,113,114,  0,  0,  0,  0,  0,  0,  /* Dx */
   0,  0,115,116,117,118,119,120,121,122,  0,  0,  0,  0,  0,  0,  /* Ex */
   0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  /* Fx */
};
#endif

int synq_sqlite3_keywordCode(const char *z, int n, int *pType){
  int i, j;
  const char *zKW;
  assert( n>=2 );
  i = ((charMap(z[0])*4) ^ (charMap(z[n-1])*3) ^ n*1) % 127;
  for(i=(int)aKWHash[i]; i>0; i=aKWNext[i]){
    if( aKWLen[i]!=n ) continue;
    zKW = &zKWText[aKWOffset[i]];
#ifdef SQLITE_ASCII
    if( (z[0]&~0x20)!=zKW[0] ) continue;
    if( (z[1]&~0x20)!=zKW[1] ) continue;
    j = 2;
    while( j<n && (z[j]&~0x20)==zKW[j] ){ j++; }
#endif
#ifdef SQLITE_EBCDIC
    if( toupper(z[0])!=zKW[0] ) continue;
    if( toupper(z[1])!=zKW[1] ) continue;
    j = 2;
    while( j<n && toupper(z[j])==zKW[j] ){ j++; }
#endif
    if( j<n ) continue;
    testcase( i==1 ); /* REINDEX */
    testcase( i==2 ); /* INDEXED */
    testcase( i==3 ); /* INDEX */
    testcase( i==4 ); /* DESC */
    testcase( i==5 ); /* ESCAPE */
    testcase( i==6 ); /* EACH */
    testcase( i==7 ); /* CHECK */
    testcase( i==8 ); /* KEY */
    testcase( i==9 ); /* BEFORE */
    testcase( i==10 ); /* FOREIGN */
    testcase( i==11 ); /* FOR */
    testcase( i==12 ); /* IGNORE */
    testcase( i==13 ); /* REGEXP */
    testcase( i==14 ); /* EXPLAIN */
    testcase( i==15 ); /* INSTEAD */
    testcase( i==16 ); /* ADD */
    testcase( i==17 ); /* DATABASE */
    testcase( i==18 ); /* AS */
    testcase( i==19 ); /* SELECT */
    testcase( i==20 ); /* TABLE */
    testcase( i==21 ); /* LEFT */
    testcase( i==22 ); /* THEN */
    testcase( i==23 ); /* END */
    testcase( i==24 ); /* DEFERRABLE */
    testcase( i==25 ); /* ELSE */
    testcase( i==26 ); /* EXCLUDE */
    testcase( i==27 ); /* DELETE */
    testcase( i==28 ); /* TEMPORARY */
    testcase( i==29 ); /* TEMP */
    testcase( i==30 ); /* OR */
    testcase( i==31 ); /* ISNULL */
    testcase( i==32 ); /* NULLS */
    testcase( i==33 ); /* SAVEPOINT */
    testcase( i==34 ); /* INTERSECT */
    testcase( i==35 ); /* TIES */
    testcase( i==36 ); /* NOTNULL */
    testcase( i==37 ); /* NOT */
    testcase( i==38 ); /* NO */
    testcase( i==39 ); /* NULL */
    testcase( i==40 ); /* LIKE */
    testcase( i==41 ); /* EXCEPT */
    testcase( i==42 ); /* TRANSACTION */
    testcase( i==43 ); /* ACTION */
    testcase( i==44 ); /* ON */
    testcase( i==45 ); /* NATURAL */
    testcase( i==46 ); /* ALTER */
    testcase( i==47 ); /* RAISE */
    testcase( i==48 ); /* EXCLUSIVE */
    testcase( i==49 ); /* EXISTS */
    testcase( i==50 ); /* CONSTRAINT */
    testcase( i==51 ); /* INTO */
    testcase( i==52 ); /* OFFSET */
    testcase( i==53 ); /* OF */
    testcase( i==54 ); /* SET */
    testcase( i==55 ); /* TRIGGER */
    testcase( i==56 ); /* RANGE */
    testcase( i==57 ); /* GENERATED */
    testcase( i==58 ); /* DETACH */
    testcase( i==59 ); /* HAVING */
    testcase( i==60 ); /* GLOB */
    testcase( i==61 ); /* BEGIN */
    testcase( i==62 ); /* INNER */
    testcase( i==63 ); /* REFERENCES */
    testcase( i==64 ); /* UNIQUE */
    testcase( i==65 ); /* QUERY */
    testcase( i==66 ); /* WITHOUT */
    testcase( i==67 ); /* WITH */
    testcase( i==68 ); /* OUTER */
    testcase( i==69 ); /* RELEASE */
    testcase( i==70 ); /* ATTACH */
    testcase( i==71 ); /* BETWEEN */
    testcase( i==72 ); /* NOTHING */
    testcase( i==73 ); /* GROUPS */
    testcase( i==74 ); /* GROUP */
    testcase( i==75 ); /* CASCADE */
    testcase( i==76 ); /* ASC */
    testcase( i==77 ); /* DEFAULT */
    testcase( i==78 ); /* CASE */
    testcase( i==79 ); /* COLLATE */
    testcase( i==80 ); /* CREATE */
    testcase( i==81 ); /* CURRENT_DATE */
    testcase( i==82 ); /* IMMEDIATE */
    testcase( i==83 ); /* JOIN */
    testcase( i==84 ); /* INSERT */
    testcase( i==85 ); /* MATCH */
    testcase( i==86 ); /* PLAN */
    testcase( i==87 ); /* ANALYZE */
    testcase( i==88 ); /* PRAGMA */
    testcase( i==89 ); /* MATERIALIZED */
    testcase( i==90 ); /* DEFERRED */
    testcase( i==91 ); /* DISTINCT */
    testcase( i==92 ); /* IS */
    testcase( i==93 ); /* UPDATE */
    testcase( i==94 ); /* VALUES */
    testcase( i==95 ); /* VIRTUAL */
    testcase( i==96 ); /* ALWAYS */
    testcase( i==97 ); /* WHEN */
    testcase( i==98 ); /* WHERE */
    testcase( i==99 ); /* RECURSIVE */
    testcase( i==100 ); /* ABORT */
    testcase( i==101 ); /* AFTER */
    testcase( i==102 ); /* RENAME */
    testcase( i==103 ); /* AND */
    testcase( i==104 ); /* DROP */
    testcase( i==105 ); /* PARTITION */
    testcase( i==106 ); /* AUTOINCREMENT */
    testcase( i==107 ); /* TO */
    testcase( i==108 ); /* IN */
    testcase( i==109 ); /* CAST */
    testcase( i==110 ); /* COLUMN */
    testcase( i==111 ); /* COMMIT */
    testcase( i==112 ); /* CONFLICT */
    testcase( i==113 ); /* CROSS */
    testcase( i==114 ); /* CURRENT_TIMESTAMP */
    testcase( i==115 ); /* CURRENT_TIME */
    testcase( i==116 ); /* CURRENT */
    testcase( i==117 ); /* PRECEDING */
    testcase( i==118 ); /* FAIL */
    testcase( i==119 ); /* LAST */
    testcase( i==120 ); /* FILTER */
    testcase( i==121 ); /* REPLACE */
    testcase( i==122 ); /* FIRST */
    testcase( i==123 ); /* FOLLOWING */
    testcase( i==124 ); /* FROM */
    testcase( i==125 ); /* FULL */
    testcase( i==126 ); /* LIMIT */
    testcase( i==127 ); /* IF */
    testcase( i==128 ); /* ORDER */
    testcase( i==129 ); /* RESTRICT */
    testcase( i==130 ); /* OTHERS */
    testcase( i==131 ); /* OVER */
    testcase( i==132 ); /* RETURNING */
    testcase( i==133 ); /* RIGHT */
    testcase( i==134 ); /* ROLLBACK */
    testcase( i==135 ); /* ROWS */
    testcase( i==136 ); /* ROW */
    testcase( i==137 ); /* UNBOUNDED */
    testcase( i==138 ); /* UNION */
    testcase( i==139 ); /* USING */
    testcase( i==140 ); /* VACUUM */
    testcase( i==141 ); /* VIEW */
    testcase( i==142 ); /* WINDOW */
    testcase( i==143 ); /* DO */
    testcase( i==144 ); /* BY */
    testcase( i==145 ); /* INITIALLY */
    testcase( i==146 ); /* ALL */
    testcase( i==147 ); /* PRIMARY */
    *pType = aKWCode[i];
    break;
  }
  return n;
}

