# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

from python.syntaqlite.diff_tests.testing import DiffTestBlueprint, TestSuite


# Precedence levels (from _common.y, low to high):
#   1: OR
#   2: AND
#   3: EQ, NE  (also IS, MATCH, LIKE, BETWEEN, IN — separate node types)
#   4: LT, GT, LE, GE
#   5: BIT_AND, BIT_OR, LSHIFT, RSHIFT
#   6: PLUS, MINUS
#   7: STAR, SLASH, REM
#   8: CONCAT, PTR
#
# Operator groups (cross-group always gets parens for readability):
#   logical+comparison: OR, AND, EQ, NE, LT, GT, LE, GE
#   bitwise:            BIT_AND, BIT_OR, LSHIFT, RSHIFT
#   arithmetic:         PLUS, MINUS, STAR, SLASH, REM
#   string:             CONCAT, PTR


class OrAndPrecedence(TestSuite):
    """OR (prec 1) vs AND (prec 2): same group, AND binds tighter."""

    def test_and_in_or_no_parens(self):
        return DiffTestBlueprint(
            sql="SELECT a AND b OR c AND d",
            out="SELECT a AND b OR c AND d;",
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


class EqualityComparisonPrecedence(TestSuite):
    """EQ/NE (prec 3) vs LT/GT/LE/GE (prec 4): same group, comparisons bind tighter."""

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


class BitwiseVsArithmeticPrecedence(TestSuite):
    """Bitwise (prec 5) vs arithmetic (prec 6/7): cross-group, parens added."""

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


class ConcatPtrPrecedence(TestSuite):
    """CONCAT/PTR (prec 8) — highest among binary ops, own group."""

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

    def test_concat_in_add_gets_parens(self):
        return DiffTestBlueprint(
            sql="SELECT a || b + c || d",
            out="SELECT (a || b) + (c || d);",
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


class ComparisonVsArithmeticPrecedence(TestSuite):
    """LT/GT/LE/GE (prec 4) vs PLUS/MINUS (prec 6): cross-group, parens added."""

    def test_add_in_gt_gets_parens(self):
        return DiffTestBlueprint(
            sql="SELECT a + b > c - d",
            out="SELECT (a + b) > (c - d);",
        )

    def test_gt_in_add(self):
        return DiffTestBlueprint(
            sql="SELECT (a > b) + c",
            out="SELECT (a > b) + c;",
        )


class ComparisonVsBitwisePrecedence(TestSuite):
    """LT/GT/LE/GE (prec 4) vs bitwise (prec 5): cross-group, parens added."""

    def test_bitand_in_lt_gets_parens(self):
        return DiffTestBlueprint(
            sql="SELECT a & b < c & d",
            out="SELECT (a & b) < (c & d);",
        )

    def test_lt_in_bitor(self):
        return DiffTestBlueprint(
            sql="SELECT (a < b) | c",
            out="SELECT (a < b) | c;",
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


class DeepNesting(TestSuite):
    """Multi-level nesting across precedence boundaries."""

    def test_three_levels(self):
        return DiffTestBlueprint(
            sql="SELECT (a + b) * c > d AND e",
            out="SELECT ((a + b) * c) > d AND e;",
        )

    def test_or_and_eq_add_mul(self):
        return DiffTestBlueprint(
            sql="SELECT a * b + c = d AND e OR f",
            out="SELECT (a * b + c) = d AND e OR f;",
        )

    def test_complex_parens_preserved(self):
        return DiffTestBlueprint(
            sql="SELECT (a OR b) AND (c + d) > (e * f)",
            out="SELECT (a OR b) AND (c + d) > (e * f);",
        )

    def test_bitwise_in_comparison_in_and(self):
        return DiffTestBlueprint(
            sql="SELECT a & b > 0 AND c | d < 10",
            out="SELECT (a & b) > 0 AND (c | d) < 10;",
        )

    def test_all_levels(self):
        return DiffTestBlueprint(
            sql="SELECT a || b * c + d & e > f = g AND h OR i",
            out="SELECT (((a || b) * c + d) & e) > f = g AND h OR i;",
        )


class InWhereClause(TestSuite):
    """Precedence in WHERE clause context (common real-world usage)."""

    def test_where_and_or(self):
        return DiffTestBlueprint(
            sql="SELECT x FROM t WHERE a = 1 AND (b = 2 OR c = 3)",
            out="SELECT x FROM t WHERE a = 1 AND (b = 2 OR c = 3);",
        )

    def test_where_arithmetic_comparison(self):
        return DiffTestBlueprint(
            sql="SELECT x FROM t WHERE a + b > c * d",
            out="SELECT x FROM t WHERE (a + b) > (c * d);",
        )

    def test_where_not_compound(self):
        return DiffTestBlueprint(
            sql="SELECT x FROM t WHERE NOT (a = 1 OR b = 2)",
            out="SELECT x FROM t WHERE NOT (a = 1 OR b = 2);",
        )
