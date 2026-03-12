CREATE TABLE users(id INTEGER PRIMARY KEY, email TEXT, name TEXT, active INT, last_login TEXT);
CREATE TABLE orders(id INTEGER PRIMARY KEY, customer_id INT REFERENCES users(id), status TEXT, total REAL);
CREATE TABLE products(id INTEGER PRIMARY KEY, name TEXT, price REAL, category_id INT);
CREATE TABLE order_items(id INTEGER PRIMARY KEY, order_id INT, product_id INT, qty INT, price REAL);
-- Realistic SQL workload for benchmarking parse/format throughput.
-- Only standard SQL that all tools should handle — no obscure syntax.

SELECT id, name, email FROM users WHERE active = 1 ORDER BY name;

INSERT INTO users (name, email, active) VALUES ('Alice', 'alice@example.com', 1);

UPDATE users SET active = 0 WHERE last_login < '2024-01-01';

DELETE FROM sessions WHERE expires_at < DATETIME('now');

SELECT u.name, COUNT(o.id) AS order_count, SUM(o.total) AS total_spent
FROM users u
LEFT JOIN orders o ON o.customer_id = u.id
WHERE u.active = 1
GROUP BY u.id, u.name
HAVING COUNT(o.id) > 0
ORDER BY total_spent DESC
LIMIT 20;

CREATE TABLE invoices (
  id INTEGER PRIMARY KEY,
  order_id INTEGER NOT NULL REFERENCES orders(id),
  amount REAL NOT NULL,
  status TEXT NOT NULL DEFAULT 'pending',
  created_at TEXT NOT NULL DEFAULT (DATETIME('now'))
);

CREATE INDEX idx_invoices_order ON invoices(order_id);

SELECT p.name, p.price, c.name AS category
FROM products p
INNER JOIN categories c ON c.id = p.category_id
WHERE p.price BETWEEN 10.0 AND 100.0
  AND p.name LIKE '%widget%'
ORDER BY p.price;

INSERT INTO order_items (order_id, product_id, qty, price)
SELECT 1, id, 1, price FROM products WHERE category_id = 5;

SELECT date(created_at) AS day, COUNT(*) AS cnt
FROM orders
WHERE created_at >= DATE('now', '-30 days')
GROUP BY date(created_at)
ORDER BY day;

WITH monthly AS (
  SELECT STRFTIME('%Y-%m', created_at) AS month, SUM(total) AS revenue
  FROM orders
  GROUP BY STRFTIME('%Y-%m', created_at)
)
SELECT month, revenue FROM monthly ORDER BY month DESC LIMIT 12;

UPDATE orders SET status = 'shipped', updated_at = DATETIME('now')
WHERE status = 'processing' AND created_at < DATETIME('now', '-2 days');

SELECT DISTINCT category FROM products ORDER BY category;

SELECT * FROM users WHERE id IN (SELECT customer_id FROM orders WHERE total > 500);

SELECT name, COALESCE(nickname, name) AS display,
  CASE WHEN active = 1 THEN 'active' ELSE 'inactive' END AS status
FROM users;

CREATE TABLE audit_entries (
  id INTEGER PRIMARY KEY,
  table_name TEXT NOT NULL,
  row_id INTEGER NOT NULL,
  change_type TEXT NOT NULL,
  old_data TEXT,
  new_data TEXT,
  created_at TEXT NOT NULL DEFAULT (DATETIME('now'))
);

SELECT o.id, o.total,
  (SELECT name FROM users WHERE id = o.customer_id) AS customer_name,
  (SELECT COUNT(*) FROM order_items WHERE order_id = o.id) AS item_count
FROM orders o
WHERE o.status = 'completed';

DROP TABLE IF EXISTS temp_report;

CREATE TABLE temp_report AS
SELECT u.name, COUNT(*) AS orders, SUM(o.total) AS spent
FROM users u JOIN orders o ON o.customer_id = u.id
GROUP BY u.id;

ALTER TABLE users ADD COLUMN phone TEXT;

SELECT name, email FROM users
UNION
SELECT name, email FROM archived_users
ORDER BY name;

SELECT * FROM products
WHERE price > (SELECT AVG(price) FROM products)
ORDER BY price DESC
LIMIT 10 OFFSET 20;

SELECT CAST(COUNT(*) AS TEXT) || ' users' AS summary FROM users;

INSERT INTO users (name, email) VALUES ('Bob', 'bob@test.com'), ('Carol', 'carol@test.com'), ('Dave', 'dave@test.com');

SELECT id, name, email FROM users WHERE email LIKE '%@example.com' AND name IS NOT NULL;

SELECT customer_id, MIN(total) AS smallest, MAX(total) AS largest, AVG(total) AS average
FROM orders
GROUP BY customer_id;

SELECT ABS(-5), LENGTH('hello'), UPPER('world'), LOWER('HELLO'), TRIM('  hi  ');

SELECT u.name, o.total
FROM users u, orders o
WHERE u.id = o.customer_id AND o.total > 100
ORDER BY o.total DESC;

UPDATE products SET price = ROUND(price * 1.1, 2) WHERE category_id = 3;
