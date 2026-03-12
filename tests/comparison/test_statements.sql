-- Each statement is separated by a blank line for easy splitting.
-- These test obscure SQLite-specific syntax that formatters must handle.

-- T01: Multi ON CONFLICT UPSERT + RETURNING
INSERT INTO inventory (sku, warehouse, qty, price)
VALUES ('ABC-123', 'WH-EAST', 50, 19.99)
ON CONFLICT (sku, warehouse) DO UPDATE SET qty = inventory.qty + excluded.qty
ON CONFLICT (sku) WHERE warehouse IS NULL DO NOTHING
RETURNING sku, qty AS new_qty, TYPEOF(price) AS price_type;

-- T02: Recursive CTE + MATERIALIZED / NOT MATERIALIZED
WITH RECURSIVE cnt(x) AS MATERIALIZED (
    VALUES(1) UNION ALL SELECT x+1 FROM cnt WHERE x < 100
),
running AS NOT MATERIALIZED (
    SELECT x, SUM(x) OVER (ORDER BY x ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) AS s
    FROM cnt
)
SELECT * FROM running LIMIT 10 OFFSET 5;

-- T03: CREATE TABLE STRICT + WITHOUT ROWID + generated columns
CREATE TABLE IF NOT EXISTS measurements (
  id INTEGER PRIMARY KEY,
  sensor_id TEXT NOT NULL REFERENCES sensors(id) ON DELETE CASCADE ON UPDATE SET NULL DEFERRABLE INITIALLY DEFERRED,
  raw_value REAL NOT NULL CHECK(raw_value BETWEEN -1000.0 AND 1000.0),
  unit TEXT NOT NULL DEFAULT 'celsius' COLLATE NOCASE,
  calibrated REAL GENERATED ALWAYS AS (raw_value * 1.02 + 0.5) STORED,
  recorded_at TEXT NOT NULL DEFAULT (STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')),
  UNIQUE (sensor_id, recorded_at) ON CONFLICT REPLACE
) STRICT, WITHOUT ROWID;

-- T04: UPDATE FROM + INDEXED BY
UPDATE orders INDEXED BY idx_orders_status
SET total = (SELECT SUM(oi.qty * oi.price) FROM order_items oi WHERE oi.order_id = orders.id),
    updated_at = DATETIME('now')
FROM customers c
WHERE orders.customer_id = c.id AND c.active = 1;

-- T05: CREATE TRIGGER + RAISE + WHEN + FOR EACH ROW
CREATE TRIGGER IF NOT EXISTS enforce_audit
  AFTER UPDATE OF salary, department ON employees
  FOR EACH ROW
  WHEN NEW.salary > OLD.salary * 1.5
BEGIN
  INSERT INTO audit_log (tbl, row_id, old_val)
    VALUES ('employees', OLD.id, JSON_OBJECT('salary', OLD.salary));
  SELECT CASE WHEN NEW.salary > OLD.salary * 2.0
    THEN RAISE(ABORT, 'Salary increase exceeds 100%') ELSE 1 END;
END;

-- T06: FILTER clause + IIF + NULLS LAST
SELECT d.name,
  COUNT(*) FILTER (WHERE e.active = 1) AS active_count,
  IIF(COUNT(*) > 50, 'large', 'small') AS dept_size
FROM departments d LEFT JOIN employees e ON e.dept_id = d.id
GROUP BY d.id
ORDER BY active_count DESC NULLS LAST;

-- T07: ATTACH DATABASE
ATTACH DATABASE ':memory:' AS scratch;

-- T08: INSERT OR REPLACE
INSERT OR REPLACE INTO kv (key, value) VALUES ('foo', 'bar');

-- T09: CREATE VIRTUAL TABLE (FTS5)
CREATE VIRTUAL TABLE IF NOT EXISTS docs_fts USING fts5(title, body, content=docs, content_rowid=id);

-- T10: PRAGMA
PRAGMA table_info('measurements');

-- T11: EXPLAIN QUERY PLAN
EXPLAIN QUERY PLAN SELECT * FROM users WHERE email LIKE '%@example.com';

-- T12: ALTER TABLE DROP COLUMN
ALTER TABLE measurements DROP COLUMN unit;

-- T13: ALTER TABLE RENAME COLUMN
ALTER TABLE measurements RENAME COLUMN raw_value TO raw_reading;

-- T14: REINDEX
REINDEX idx_measurements_sensor;

-- T15: Window frame RANGE BETWEEN
SELECT id, value,
  AVG(value) OVER (ORDER BY id RANGE BETWEEN 10 PRECEDING AND 10 FOLLOWING) AS moving_avg
FROM data;

-- T16: CREATE INDEX with WHERE (partial index)
CREATE INDEX IF NOT EXISTS idx_active_users ON users(email, name) WHERE active = 1 AND deleted_at IS NULL;

-- T17: REPLACE statement
REPLACE INTO settings (key, value) VALUES ('theme', 'dark');

-- T18: Nested window functions + EXCLUDE
SELECT id,
  SUM(value) OVER w AS running,
  LAG(value, 2) OVER w AS lagged
FROM data
WINDOW w AS (ORDER BY id ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW EXCLUDE CURRENT ROW);

-- T19: GLOB and LIKE with ESCAPE
SELECT * FROM files WHERE path GLOB 'src/**/*.rs' AND name LIKE '%test\_util%' ESCAPE '\';

-- T20: INSERT with multiple VALUES + ON CONFLICT DO UPDATE SET with excluded
INSERT INTO counters (name, count) VALUES ('a', 1), ('b', 2), ('c', 3)
ON CONFLICT (name) DO UPDATE SET count = counters.count + excluded.count;

-- T21: Complex subquery expressions
SELECT (SELECT COUNT(*) FROM users) AS user_count,
       EXISTS (SELECT 1 FROM orders WHERE total > 1000) AS has_big_orders,
       CAST(ROUND(AVG(price), 2) AS TEXT) AS avg_price,
       NULLIF(status, 'unknown') AS clean_status,
       COALESCE(nickname, username, 'anonymous') AS display_name
FROM products;

-- T22: ANALYZE
ANALYZE;

-- T23: SAVEPOINT / RELEASE / ROLLBACK TO
SAVEPOINT my_savepoint;

-- T24: DROP TABLE IF EXISTS
DROP TABLE IF EXISTS temp_results;

-- T25: CREATE TABLE AS SELECT
CREATE TABLE report_snapshot AS
SELECT date('now') AS snapshot_date, category, SUM(amount) AS total
FROM transactions
GROUP BY category;

-- T26: DETACH DATABASE
DETACH DATABASE scratch;

-- T27: UPSERT with complex expressions in DO UPDATE
INSERT INTO metrics (name, ts, value)
VALUES ('cpu', STRFTIME('%s', 'now'), 0.75)
ON CONFLICT (name, ts) DO UPDATE SET
  value = (metrics.value * 0.9 + excluded.value * 0.1),
  updated_count = metrics.updated_count + 1
RETURNING name, value, updated_count;

-- T28: WITH (non-recursive) + DELETE ... RETURNING
WITH stale AS (
  SELECT id FROM sessions WHERE last_active < DATETIME('now', '-30 days')
)
DELETE FROM sessions WHERE id IN (SELECT id FROM stale)
RETURNING id, user_id;

-- T29: UPDATE ... RETURNING
UPDATE users SET last_login = DATETIME('now') WHERE id = 42 RETURNING id, name, last_login;

-- T30: RIGHT JOIN + IS DISTINCT FROM
SELECT u.name, o.total
FROM orders o RIGHT JOIN users u ON u.id = o.customer_id
WHERE o.total IS DISTINCT FROM NULL;

-- T31: FULL OUTER JOIN
SELECT u.name, o.total
FROM users u FULL OUTER JOIN orders o ON u.id = o.customer_id
ORDER BY u.name NULLS FIRST;

-- T32: JSON -> and ->> operators
SELECT id,
  data -> '$.name' AS name_json,
  data ->> '$.email' AS email_text,
  data -> '$.tags' ->> '$[0]' AS first_tag
FROM docs_json;

-- T33: Numeric literals with underscores
SELECT 1_000_000 AS million, 0xFF_FF AS hex_val, 3.141_592_653 AS pi_approx, 1_0e1_0 AS sci;

-- T34: Multiple WINDOW definitions + nth_value + ntile
SELECT id, value,
  row_number() OVER w_ord AS rn,
  ntile(4) OVER w_ord AS quartile,
  first_value(value) OVER w_frame AS frame_first,
  nth_value(value, 3) OVER w_frame AS third_val,
  cume_dist() OVER w_ord AS cd,
  percent_rank() OVER w_ord AS pr
FROM data
WINDOW w_ord AS (ORDER BY id),
       w_frame AS (ORDER BY id ROWS BETWEEN 2 PRECEDING AND 2 FOLLOWING);

-- T35: HAVING without GROUP BY (3.39+)
SELECT count(*) AS n FROM users HAVING count(*) > 0;

-- T36: IS NOT DISTINCT FROM in complex expression
SELECT *
FROM orders o
WHERE o.status IS NOT DISTINCT FROM 'shipped'
  AND o.total IS DISTINCT FROM 0.0
  AND o.customer_id IS NOT DISTINCT FROM (SELECT id FROM customers LIMIT 1);

-- T37: Blob literals + CAST chains
SELECT X'DEADBEEF' AS raw_blob,
  CAST(X'48454C4C4F' AS TEXT) AS hello,
  CAST(CAST(X'FF' AS INTEGER) AS TEXT) AS chain,
  HEX(UNHEX('48454C4C4F')) AS roundtrip;

-- T38: GENERATED ALWAYS AS (VIRTUAL vs STORED) + complex expressions
CREATE TABLE generated_demo (
  a INTEGER NOT NULL,
  b INTEGER NOT NULL,
  sum_ab INTEGER GENERATED ALWAYS AS (a + b) VIRTUAL,
  prod_ab INTEGER GENERATED ALWAYS AS (a * b) STORED,
  label TEXT GENERATED ALWAYS AS (CAST(a AS TEXT) || 'x' || CAST(b AS TEXT)) VIRTUAL,
  ratio REAL GENERATED ALWAYS AS (CASE WHEN b != 0 THEN CAST(a AS REAL) / b ELSE NULL END) STORED
);

-- T39: Deeply nested CTE + compound SELECT (UNION / INTERSECT / EXCEPT)
WITH
  a AS (SELECT id, name FROM users WHERE active = 1),
  b AS (SELECT customer_id AS id FROM orders),
  c AS (SELECT id FROM a INTERSECT SELECT id FROM b),
  d AS (SELECT id FROM a EXCEPT SELECT id FROM c)
SELECT id, name FROM users WHERE id IN (SELECT id FROM d)
UNION ALL
SELECT id, name FROM users WHERE id IN (SELECT id FROM c)
ORDER BY name;

-- T40: Window GROUPS frame + EXCLUDE TIES
SELECT id, value,
  SUM(value) OVER (ORDER BY id GROUPS BETWEEN 1 PRECEDING AND 1 FOLLOWING EXCLUDE TIES) AS grp_sum,
  AVG(value) OVER (ORDER BY id GROUPS CURRENT ROW EXCLUDE NO OTHERS) AS grp_avg
FROM data;
