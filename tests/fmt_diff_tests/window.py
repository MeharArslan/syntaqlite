# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

from python.syntaqlite.diff_tests.testing import DiffTestBlueprint, TestSuite


class WindowFunctionFormat(TestSuite):
    def test_over_order_by(self):
        return DiffTestBlueprint(
            sql="select row_number() over (order by id) from t",
            out="SELECT row_number() OVER (ORDER BY id) FROM t;",
        )

    def test_over_partition_by(self):
        return DiffTestBlueprint(
            sql="select count(*) over (partition by a) from t",
            out="SELECT count(*) OVER (PARTITION BY a) FROM t;",
        )

    def test_over_partition_and_order(self):
        return DiffTestBlueprint(
            sql="select sum(x) over (partition by a order by b) from t",
            out="SELECT sum(x) OVER (PARTITION BY a ORDER BY b) FROM t;",
        )

    def test_over_named_window(self):
        return DiffTestBlueprint(
            sql="select sum(x) over w from t window w as (order by x)",
            out="SELECT sum(x) OVER w FROM t WINDOW w AS (ORDER BY x);",
        )

    def test_multiple_named_windows(self):
        return DiffTestBlueprint(
            sql="select sum(x) over w1, avg(y) over w2 from t window w1 as (order by a), w2 as (partition by b order by c)",
            out="""\
                SELECT sum(x) OVER w1, avg(y) OVER w2
                FROM t
                WINDOW
                  w1 AS (ORDER BY a),
                  w2 AS (PARTITION BY b ORDER BY c);
            """,
        )

    def test_long_over_partition_wraps(self):
        return DiffTestBlueprint(
            sql="""\
                SELECT
                  customer_id,
                  order_id,
                  order_total,
                  rank() OVER (PARTITION BY customer_iddffkjllfjksljdfdklsfjklsfjkljfdklsdsjklfjkslfjljskdfjkl ORDER BY order_total DESC) AS customer_rank,
                  dense_rank() OVER (PARTITION BY customer_id ORDER BY order_total DESC) AS customer_dense_rank,
                  count(*) OVER (PARTITION BY customer_id) AS customer_order_count
                FROM orders
                WHERE
                  order_total > 0;
            """,
            out="""\
                SELECT
                  customer_id,
                  order_id,
                  order_total,
                  rank() OVER (
                    PARTITION BY
                      customer_iddffkjllfjksljdfdklsfjklsfjkljfdklsdsjklfjkslfjljskdfjkl
                    ORDER BY
                      order_total DESC
                  ) AS customer_rank,
                  dense_rank() OVER (PARTITION BY customer_id ORDER BY order_total DESC) AS customer_dense_rank,
                  count(*) OVER (PARTITION BY customer_id) AS customer_order_count
                FROM orders
                WHERE
                  order_total > 0;
            """,
        )

    def test_long_named_window_def_wraps(self):
        return DiffTestBlueprint(
            sql="""\
                select sum(order_total) over w
                from orders
                window w as (partition by customer_iddffkjllfjksljdfdklsfjklsfjkljfdklsdsjklfjkslfjljskdfjkl order by order_total desc)
            """,
            out="""\
                SELECT sum(order_total) OVER w
                FROM orders
                WINDOW
                  w AS (
                    PARTITION BY
                      customer_iddffkjllfjksljdfdklsfjklsfjkljfdklsdsjklfjkslfjljskdfjkl
                    ORDER BY
                      order_total DESC
                  );
            """,
        )


    def test_partition_by_multi_arg_nests(self):
        return DiffTestBlueprint(
            sql="""\
                SELECT last_value(thread.start_ts) OVER (
                  PARTITION BY upid, android_standardize_thread_name(thread.name)
                  ORDER BY thread.start_ts
                  RANGE BETWEEN CURRENT ROW AND cast_int!($sliding_window_dur) FOLLOWING
                ) FROM thread
            """,
            out="""\
                SELECT
                  last_value(thread.start_ts) OVER (
                    PARTITION BY
                      upid,
                      android_standardize_thread_name(thread.name)
                    ORDER BY
                      thread.start_ts
                    RANGE BETWEEN CURRENT ROW AND cast_int!($sliding_window_dur)
                  )
                FROM thread;
            """,
        )

    def test_short_partition_by_stays_inline(self):
        return DiffTestBlueprint(
            sql="SELECT sum(x) OVER (PARTITION BY id ORDER BY ts) FROM t",
            out="SELECT sum(x) OVER (PARTITION BY id ORDER BY ts) FROM t;",
        )


class FilterOverFormat(TestSuite):
    def test_filter_only(self):
        return DiffTestBlueprint(
            sql="select count(*) filter (where x > 0) from t",
            out="SELECT count(*) FILTER (WHERE x > 0) FROM t;",
        )

    def test_filter_with_over(self):
        return DiffTestBlueprint(
            sql="select sum(x) filter (where x > 0) over (order by y) from t",
            out="SELECT sum(x) FILTER (WHERE x > 0) OVER (ORDER BY y) FROM t;",
        )

    def test_filter_with_named_window(self):
        return DiffTestBlueprint(
            sql="select sum(x) filter (where x > 0) over w from t window w as (order by y)",
            out="SELECT sum(x) FILTER (WHERE x > 0) OVER w FROM t WINDOW w AS (ORDER BY y);",
        )


class FrameSpecFormat(TestSuite):
    def test_rows_between(self):
        return DiffTestBlueprint(
            sql="select sum(x) over (order by y rows between 1 preceding and 1 following) from t",
            out="SELECT sum(x) OVER (ORDER BY y ROWS BETWEEN 1 PRECEDING AND 1 FOLLOWING) FROM t;",
        )

    def test_range_unbounded(self):
        return DiffTestBlueprint(
            sql="select sum(x) over (order by y range between unbounded preceding and current row) from t",
            out="""\
                SELECT
                  sum(x) OVER (ORDER BY y RANGE BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW)
                FROM t;
            """,
        )

    def test_groups_with_exclude(self):
        return DiffTestBlueprint(
            sql="select sum(x) over (order by y groups between unbounded preceding and unbounded following exclude ties) from t",
            out="""\
                SELECT
                  sum(x) OVER (
                    ORDER BY
                      y
                    GROUPS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING EXCLUDE TIES
                  )
                FROM t;
            """,
        )

    def test_rows_single_bound(self):
        return DiffTestBlueprint(
            sql="select sum(x) over (order by y rows 2 preceding) from t",
            out="SELECT sum(x) OVER (ORDER BY y ROWS BETWEEN 2 PRECEDING AND CURRENT ROW) FROM t;",
        )
