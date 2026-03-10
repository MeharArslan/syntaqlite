# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

from python.syntaqlite.diff_tests.testing import DiffTestBlueprint, TestSuite


# Precedence levels (from _common.y, low to high):
#   1: OR
#   2: AND                (paren_boundary)
#   3: EQ, NE, IS, LIKE, GLOB, MATCH, REGEXP, BETWEEN, IN
#   4: LT, GT, LE, GE
#   5: BIT_AND, BIT_OR, LSHIFT, RSHIFT
#   6: PLUS, MINUS
#   7: STAR, SLASH, REM
#   8: CONCAT, PTR
#   9: COLLATE
#
# Operator groups (cross-group always gets parens for readability):
#   STANDARD (0): OR, AND, EQ, NE, LT, GT, LE, GE, PLUS, MINUS, STAR, SLASH,
#                 REM, CONCAT, PTR, IS, LIKE, BETWEEN, IN, COLLATE
#   BITWISE  (1): BIT_AND, BIT_OR, LSHIFT, RSHIFT
#
# Paren boundary: AND has the paren_boundary flag. When AND appears as a child
# of a different-precedence operator in the same group, readability parens are
# added. This gives us `(a AND b) OR c` without adding parens everywhere.


class OrAndPrecedence(TestSuite):
    """OR (prec 1) vs AND (prec 2): AND has paren_boundary."""

    def test_and_in_or_gets_parens(self):
        return DiffTestBlueprint(
            sql="SELECT a AND b OR c AND d",
            out="SELECT (a AND b) OR (c AND d);",
        )

    def test_or_in_and_left(self):
        return DiffTestBlueprint(
            sql="SELECT (a OR b) AND c",
            out="SELECT (a OR b) AND c;",
        )

    def test_or_in_and_right(self):
        return DiffTestBlueprint(
            sql="SELECT a AND (b OR c)",
            out="SELECT a AND (b OR c);",
        )

    def test_or_in_and_both(self):
        return DiffTestBlueprint(
            sql="SELECT (a OR b) AND (c OR d)",
            out="SELECT (a OR b) AND (c OR d);",
        )

    def test_chained_or(self):
        return DiffTestBlueprint(
            sql="SELECT a OR b OR c",
            out="SELECT a OR b OR c;",
        )

    def test_chained_and(self):
        return DiffTestBlueprint(
            sql="SELECT a AND b AND c",
            out="SELECT a AND b AND c;",
        )

    def test_three_ands_in_or(self):
        """a AND b AND c OR d → only one set of parens around the AND chain."""
        return DiffTestBlueprint(
            sql="SELECT a AND b AND c OR d",
            out="SELECT (a AND b AND c) OR d;",
        )


class EqualityComparisonPrecedence(TestSuite):
    """EQ/NE (prec 3) vs LT/GT/LE/GE (prec 4): same group, no readability parens."""

    def test_comparison_in_equality_no_parens(self):
        return DiffTestBlueprint(
            sql="SELECT a > b = c < d",
            out="SELECT a > b = c < d;",
        )

    def test_eq_in_comparison(self):
        return DiffTestBlueprint(
            sql="SELECT (a = b) > (c = d)",
            out="SELECT (a = b) > (c = d);",
        )

    def test_ne_in_lt(self):
        return DiffTestBlueprint(
            sql="SELECT (a != b) < c",
            out="SELECT (a != b) < c;",
        )

    def test_eq_and_ne(self):
        return DiffTestBlueprint(
            sql="SELECT a = b != c",
            out="SELECT a = b != c;",
        )

    def test_right_assoc_eq_in_ge(self):
        return DiffTestBlueprint(
            sql="SELECT a >= (b = c)",
            out="SELECT a >= (b = c);",
        )


class AndEqualityPrecedence(TestSuite):
    """AND (prec 2) vs EQ/NE (prec 3): same group, no readability parens."""

    def test_eq_in_and_no_parens(self):
        return DiffTestBlueprint(
            sql="SELECT a = 1 AND b = 2",
            out="SELECT a = 1 AND b = 2;",
        )

    def test_and_in_eq(self):
        return DiffTestBlueprint(
            sql="SELECT (a AND b) = (c AND d)",
            out="SELECT (a AND b) = (c AND d);",
        )

    def test_eq_in_or_no_boundary(self):
        """EQ has no paren_boundary, so no readability parens inside OR."""
        return DiffTestBlueprint(
            sql="SELECT a = 1 OR b = 2",
            out="SELECT a = 1 OR b = 2;",
        )


class ArithmeticPrecedence(TestSuite):
    """PLUS/MINUS (prec 6) vs STAR/SLASH/REM (prec 7): same group."""

    def test_mul_in_add_no_parens(self):
        return DiffTestBlueprint(
            sql="SELECT a + b * c",
            out="SELECT a + b * c;",
        )

    def test_add_in_mul(self):
        return DiffTestBlueprint(
            sql="SELECT (a + b) * c",
            out="SELECT (a + b) * c;",
        )

    def test_sub_in_div(self):
        return DiffTestBlueprint(
            sql="SELECT (a - b) / c",
            out="SELECT (a - b) / c;",
        )

    def test_mul_add_mul(self):
        return DiffTestBlueprint(
            sql="SELECT a * b + c * d",
            out="SELECT a * b + c * d;",
        )

    def test_rem_in_add(self):
        return DiffTestBlueprint(
            sql="SELECT a + b % c",
            out="SELECT a + b % c;",
        )

    def test_add_in_rem(self):
        return DiffTestBlueprint(
            sql="SELECT (a + b) % c",
            out="SELECT (a + b) % c;",
        )


class SamePrecAssociativity(TestSuite):
    """Same-precedence left-associativity: right-child needs parens."""

    def test_sub_right_assoc(self):
        return DiffTestBlueprint(
            sql="SELECT a - (b + c)",
            out="SELECT a - (b + c);",
        )

    def test_sub_right_assoc_sub(self):
        return DiffTestBlueprint(
            sql="SELECT a - (b - c)",
            out="SELECT a - (b - c);",
        )

    def test_sub_left_assoc_no_parens(self):
        return DiffTestBlueprint(
            sql="SELECT a - b + c",
            out="SELECT a - b + c;",
        )

    def test_div_right_assoc(self):
        return DiffTestBlueprint(
            sql="SELECT a / (b * c)",
            out="SELECT a / (b * c);",
        )

    def test_div_right_assoc_rem(self):
        return DiffTestBlueprint(
            sql="SELECT a / (b % c)",
            out="SELECT a / (b % c);",
        )

    def test_mul_left_assoc_no_parens(self):
        return DiffTestBlueprint(
            sql="SELECT a * b / c",
            out="SELECT a * b / c;",
        )


class BitwiseOpsPrecedence(TestSuite):
    """BIT_AND, BIT_OR, LSHIFT, RSHIFT all at prec 5, same group."""

    def test_bitand_bitor_left_assoc(self):
        return DiffTestBlueprint(
            sql="SELECT a & b | c",
            out="SELECT a & b | c;",
        )

    def test_bitor_in_bitand_right(self):
        return DiffTestBlueprint(
            sql="SELECT a & (b | c)",
            out="SELECT a & (b | c);",
        )

    def test_lshift_rshift_same_prec(self):
        return DiffTestBlueprint(
            sql="SELECT a << b >> c",
            out="SELECT a << b >> c;",
        )

    def test_rshift_in_lshift_right(self):
        return DiffTestBlueprint(
            sql="SELECT a << (b >> c)",
            out="SELECT a << (b >> c);",
        )

    def test_bitand_in_lshift_same_prec(self):
        return DiffTestBlueprint(
            sql="SELECT a << b & c",
            out="SELECT a << b & c;",
        )

    def test_lshift_in_bitand_right(self):
        return DiffTestBlueprint(
            sql="SELECT a & (b << c)",
            out="SELECT a & (b << c);",
        )


class BitwiseVsStandardPrecedence(TestSuite):
    """Bitwise (group 1) vs standard (group 0): cross-group, parens added."""

    def test_add_in_bitand_gets_parens(self):
        return DiffTestBlueprint(
            sql="SELECT a + b & c + d",
            out="SELECT (a + b) & (c + d);",
        )

    def test_bitand_in_add(self):
        return DiffTestBlueprint(
            sql="SELECT (a & b) + c",
            out="SELECT (a & b) + c;",
        )

    def test_mul_in_bitor_gets_parens(self):
        return DiffTestBlueprint(
            sql="SELECT a * b | c * d",
            out="SELECT (a * b) | (c * d);",
        )

    def test_lshift_in_mul(self):
        return DiffTestBlueprint(
            sql="SELECT (a << b) * c",
            out="SELECT (a << b) * c;",
        )

    def test_concat_in_bitand(self):
        return DiffTestBlueprint(
            sql="SELECT a || b & c || d",
            out="SELECT (a || b) & (c || d);",
        )

    def test_bitand_in_gt_gets_parens(self):
        return DiffTestBlueprint(
            sql="SELECT a & b < c & d",
            out="SELECT (a & b) < (c & d);",
        )

    def test_lt_in_bitor(self):
        return DiffTestBlueprint(
            sql="SELECT (a < b) | c",
            out="SELECT (a < b) | c;",
        )


class ConcatPtrPrecedence(TestSuite):
    """CONCAT/PTR (prec 8) — highest among binary ops, standard group."""

    def test_concat_chain(self):
        return DiffTestBlueprint(
            sql="SELECT a || b || c",
            out="SELECT a || b || c;",
        )

    def test_concat_right_assoc(self):
        return DiffTestBlueprint(
            sql="SELECT a || (b || c)",
            out="SELECT a || (b || c);",
        )

    def test_add_in_concat(self):
        return DiffTestBlueprint(
            sql="SELECT (a + b) || c",
            out="SELECT (a + b) || c;",
        )

    def test_concat_in_add_no_parens(self):
        """Concat is same group as arithmetic — higher prec, no readability parens."""
        return DiffTestBlueprint(
            sql="SELECT a || b + c || d",
            out="SELECT a || b + c || d;",
        )

    def test_ptr_and_concat_same_prec(self):
        return DiffTestBlueprint(
            sql="SELECT a -> b || c",
            out="SELECT a -> b || c;",
        )

    def test_concat_in_ptr_right(self):
        return DiffTestBlueprint(
            sql="SELECT a -> (b || c)",
            out="SELECT a -> (b || c);",
        )


class ArithmeticVsComparisonPrecedence(TestSuite):
    """Arithmetic (prec 6/7) vs comparison (prec 4): same group, no readability parens."""

    def test_add_in_gt_no_parens(self):
        """a + b > c - d: arithmetic binds tighter, same group → no parens."""
        return DiffTestBlueprint(
            sql="SELECT a + b > c - d",
            out="SELECT a + b > c - d;",
        )

    def test_gt_in_add(self):
        return DiffTestBlueprint(
            sql="SELECT (a > b) + c",
            out="SELECT (a > b) + c;",
        )

    def test_mul_in_le_no_parens(self):
        return DiffTestBlueprint(
            sql="SELECT a * b <= c / d",
            out="SELECT a * b <= c / d;",
        )


class NotWithBinaryExpr(TestSuite):
    """NOT (unary) wrapping binary expressions."""

    def test_not_and(self):
        return DiffTestBlueprint(
            sql="SELECT NOT (a AND b)",
            out="SELECT NOT (a AND b);",
        )

    def test_not_or(self):
        return DiffTestBlueprint(
            sql="SELECT NOT (a OR b)",
            out="SELECT NOT (a OR b);",
        )

    def test_not_eq(self):
        return DiffTestBlueprint(
            sql="SELECT NOT (a = b)",
            out="SELECT NOT (a = b);",
        )

    def test_not_gt(self):
        return DiffTestBlueprint(
            sql="SELECT NOT (a > b)",
            out="SELECT NOT (a > b);",
        )

    def test_not_add(self):
        return DiffTestBlueprint(
            sql="SELECT NOT (a + b)",
            out="SELECT NOT (a + b);",
        )

    def test_not_concat(self):
        return DiffTestBlueprint(
            sql="SELECT NOT (a || b)",
            out="SELECT NOT (a || b);",
        )


class IsExprPrecedence(TestSuite):
    """IS/ISNULL/NOTNULL (prec 3, group 0) in the global precedence system."""

    def test_is_null_no_parens_in_and(self):
        return DiffTestBlueprint(
            sql="SELECT a ISNULL AND b NOTNULL",
            out="SELECT a ISNULL AND b NOTNULL;",
        )

    def test_is_in_or_no_boundary(self):
        """IS has no boundary flag — no readability parens inside OR."""
        return DiffTestBlueprint(
            sql="SELECT a IS NULL OR b IS NOT NULL",
            out="SELECT a IS NULL OR b IS NOT NULL;",
        )

    def test_or_in_is_gets_parens(self):
        """OR (prec 1) inside IS (prec 3) needs correctness parens."""
        return DiffTestBlueprint(
            sql="SELECT (a OR b) IS NULL",
            out="SELECT (a OR b) IS NULL;",
        )

    def test_and_in_is_gets_parens(self):
        """AND (prec 2) inside IS (prec 3) needs correctness parens."""
        return DiffTestBlueprint(
            sql="SELECT (a AND b) IS NOT NULL",
            out="SELECT (a AND b) IS NOT NULL;",
        )

    def test_add_in_isnull_no_parens(self):
        """Arithmetic (prec 6) in ISNULL (prec 3) — higher prec, no parens."""
        return DiffTestBlueprint(
            sql="SELECT a + b ISNULL",
            out="SELECT a + b ISNULL;",
        )

    def test_is_distinct_with_comparison(self):
        return DiffTestBlueprint(
            sql="SELECT a IS DISTINCT FROM b AND c IS NOT DISTINCT FROM d",
            out="SELECT a IS DISTINCT FROM b AND c IS NOT DISTINCT FROM d;",
        )


class LikeExprPrecedence(TestSuite):
    """LIKE/GLOB/MATCH/REGEXP (prec 3, group 0) in the global precedence system."""

    def test_like_in_and_no_parens(self):
        return DiffTestBlueprint(
            sql="SELECT a LIKE 'foo' AND b LIKE 'bar'",
            out="SELECT a LIKE 'foo' AND b LIKE 'bar';",
        )

    def test_like_in_or_no_boundary(self):
        return DiffTestBlueprint(
            sql="SELECT a LIKE 'foo' OR b LIKE 'bar'",
            out="SELECT a LIKE 'foo' OR b LIKE 'bar';",
        )

    def test_or_in_like_gets_parens(self):
        """OR (prec 1) inside LIKE (prec 3) needs correctness parens."""
        return DiffTestBlueprint(
            sql="SELECT (a OR b) LIKE 'foo'",
            out="SELECT (a OR b) LIKE 'foo';",
        )

    def test_and_in_like_gets_parens(self):
        return DiffTestBlueprint(
            sql="SELECT (a AND b) LIKE 'foo'",
            out="SELECT (a AND b) LIKE 'foo';",
        )

    def test_add_in_like_no_parens(self):
        return DiffTestBlueprint(
            sql="SELECT a + b LIKE 'foo'",
            out="SELECT a + b LIKE 'foo';",
        )

    def test_glob_preserves_keyword(self):
        """GLOB keyword should be preserved (not rewritten to LIKE)."""
        return DiffTestBlueprint(
            sql="SELECT a GLOB 'foo*'",
            out="SELECT a GLOB 'foo*';",
        )

    def test_like_with_escape(self):
        return DiffTestBlueprint(
            sql="SELECT a LIKE 'foo%' ESCAPE '\\'",
            out="SELECT a LIKE 'foo%' ESCAPE '\\';",
        )

    def test_not_like_in_and(self):
        return DiffTestBlueprint(
            sql="SELECT a NOT LIKE 'foo' AND b NOT LIKE 'bar'",
            out="SELECT a NOT LIKE 'foo' AND b NOT LIKE 'bar';",
        )


class BetweenExprPrecedence(TestSuite):
    """BETWEEN (prec 3, group 0) in the global precedence system."""

    def test_between_in_and_no_parens(self):
        return DiffTestBlueprint(
            sql="SELECT a BETWEEN 1 AND 10 AND b BETWEEN 20 AND 30",
            out="SELECT a BETWEEN 1 AND 10 AND b BETWEEN 20 AND 30;",
        )

    def test_between_in_or_no_boundary(self):
        return DiffTestBlueprint(
            sql="SELECT a BETWEEN 1 AND 10 OR b BETWEEN 20 AND 30",
            out="SELECT a BETWEEN 1 AND 10 OR b BETWEEN 20 AND 30;",
        )

    def test_or_in_between_gets_parens(self):
        return DiffTestBlueprint(
            sql="SELECT (a OR b) BETWEEN 1 AND 10",
            out="SELECT (a OR b) BETWEEN 1 AND 10;",
        )

    def test_add_in_between_no_parens(self):
        return DiffTestBlueprint(
            sql="SELECT a + b BETWEEN 1 AND 10",
            out="SELECT a + b BETWEEN 1 AND 10;",
        )

    def test_not_between(self):
        return DiffTestBlueprint(
            sql="SELECT a NOT BETWEEN 1 AND 10",
            out="SELECT a NOT BETWEEN 1 AND 10;",
        )


class InExprPrecedence(TestSuite):
    """IN (prec 3, group 0) in the global precedence system."""

    def test_in_in_and_no_parens(self):
        return DiffTestBlueprint(
            sql="SELECT a IN (1, 2) AND b IN (3, 4)",
            out="SELECT a IN (1, 2) AND b IN (3, 4);",
        )

    def test_in_in_or_no_boundary(self):
        return DiffTestBlueprint(
            sql="SELECT a IN (1, 2) OR b IN (3, 4)",
            out="SELECT a IN (1, 2) OR b IN (3, 4);",
        )

    def test_or_in_in_gets_parens(self):
        return DiffTestBlueprint(
            sql="SELECT (a OR b) IN (1, 2)",
            out="SELECT (a OR b) IN (1, 2);",
        )

    def test_add_in_in_no_parens(self):
        return DiffTestBlueprint(
            sql="SELECT a + b IN (1, 2)",
            out="SELECT a + b IN (1, 2);",
        )

    def test_not_in(self):
        return DiffTestBlueprint(
            sql="SELECT a NOT IN (1, 2, 3)",
            out="SELECT a NOT IN (1, 2, 3);",
        )


class CollateExprPrecedence(TestSuite):
    """COLLATE (prec 9, group 0) in the global precedence system."""

    def test_collate_in_eq_no_parens(self):
        return DiffTestBlueprint(
            sql="SELECT a COLLATE nocase = b COLLATE nocase",
            out="SELECT a COLLATE nocase = b COLLATE nocase;",
        )

    def test_add_in_collate_gets_parens(self):
        """Arithmetic (prec 6) inside COLLATE (prec 9) needs correctness parens."""
        return DiffTestBlueprint(
            sql="SELECT (a + b) COLLATE nocase",
            out="SELECT (a + b) COLLATE nocase;",
        )

    def test_collate_in_add_no_parens(self):
        """COLLATE (prec 9) inside add (prec 6) — higher prec, no parens."""
        return DiffTestBlueprint(
            sql="SELECT a COLLATE nocase + b",
            out="SELECT a COLLATE nocase + b;",
        )

    def test_concat_in_collate_gets_parens(self):
        """Concat (prec 8) inside COLLATE (prec 9) needs correctness parens."""
        return DiffTestBlueprint(
            sql="SELECT (a || b) COLLATE nocase",
            out="SELECT (a || b) COLLATE nocase;",
        )


class DeepNesting(TestSuite):
    """Multi-level nesting across precedence boundaries."""

    def test_three_levels(self):
        return DiffTestBlueprint(
            sql="SELECT (a + b) * c > d AND e",
            out="SELECT (a + b) * c > d AND e;",
        )

    def test_or_and_eq_add_mul(self):
        return DiffTestBlueprint(
            sql="SELECT a * b + c = d AND e OR f",
            out="SELECT (a * b + c = d AND e) OR f;",
        )

    def test_complex_parens_preserved(self):
        return DiffTestBlueprint(
            sql="SELECT (a OR b) AND (c + d) > (e * f)",
            out="SELECT (a OR b) AND c + d > e * f;",
        )

    def test_bitwise_in_comparison_in_and(self):
        return DiffTestBlueprint(
            sql="SELECT a & b > 0 AND c | d < 10",
            out="SELECT (a & b) > 0 AND (c | d) < 10;",
        )

    def test_all_levels(self):
        return DiffTestBlueprint(
            sql="SELECT a || b * c + d & e > f = g AND h OR i",
            out="SELECT (((a || b * c + d) & e) > f = g AND h) OR i;",
        )

    def test_like_and_between_in_or(self):
        return DiffTestBlueprint(
            sql="SELECT a LIKE 'foo' AND b BETWEEN 1 AND 10 OR c IN (1, 2)",
            out="SELECT (a LIKE 'foo' AND b BETWEEN 1 AND 10) OR c IN (1, 2);",
        )


class InWhereClause(TestSuite):
    """Precedence in WHERE clause context (common real-world usage)."""

    def test_where_and_or(self):
        return DiffTestBlueprint(
            sql="SELECT x FROM t WHERE a = 1 AND (b = 2 OR c = 3)",
            out="SELECT x FROM t WHERE a = 1 AND (b = 2 OR c = 3);",
        )

    def test_where_arithmetic_comparison(self):
        """Arithmetic and comparison are same group — no readability parens."""
        return DiffTestBlueprint(
            sql="SELECT x FROM t WHERE a + b > c * d",
            out="SELECT x FROM t WHERE a + b > c * d;",
        )

    def test_where_not_compound(self):
        return DiffTestBlueprint(
            sql="SELECT x FROM t WHERE NOT (a = 1 OR b = 2)",
            out="SELECT x FROM t WHERE NOT (a = 1 OR b = 2);",
        )

    def test_where_like_and_in(self):
        return DiffTestBlueprint(
            sql="SELECT x FROM t WHERE name LIKE 'foo%' AND id IN (1, 2, 3)",
            out="SELECT x FROM t WHERE name LIKE 'foo%' AND id IN (1, 2, 3);",
        )
