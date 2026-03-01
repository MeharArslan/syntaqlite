// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

export interface SqlPreset {
  id: string;
  label: string;
  description: string;
  sql: string;
}

export interface SqlPresetLibrary {
  dialectId: string;
  label: string;
  presets: SqlPreset[];
}

const SQLITE_PRESETS: SqlPreset[] = [
  {
    id: "sqlite-basic-select",
    label: "Basic Select",
    description: "Join with aggregate, filter, and ordering.",
    sql: `SELECT o.order_id, c.customer_name, sum(li.quantity * li.unit_price) AS order_total, coalesce(sh.shipping_method_name, 'standard') AS shipping_method
FROM orders o
JOIN order_line_items li ON li.order_id = o.order_id
LEFT JOIN customers c ON c.customer_id = o.customer_id
LEFT JOIN shipping_methods sh ON sh.shipping_method_id = o.shipping_method_id
WHERE o.created_at >= datetime('now', '-30 day') AND o.status = 'shipped'
GROUP BY o.order_id, c.customer_name, sh.shipping_method_name
ORDER BY order_total DESC, o.created_at DESC
LIMIT 25;`,
  },
  {
    id: "sqlite-string-functions",
    label: "String Functions",
    description: "Common SQLite string function usage.",
    sql: `SELECT
  id,
  lower(trim(name)) AS name_norm,
  substr(email, 1, 12) AS email_prefix,
  coalesce(phone, 'n/a') AS phone,
  instr(email, '@') AS email_at
FROM users
WHERE length(name) > 2 AND email LIKE '%@%'
ORDER BY name_norm ASC
LIMIT 50;`,
  },
  {
    id: "sqlite-date-functions",
    label: "Date Functions",
    description: "Date/time helpers and arithmetic.",
    sql: `SELECT
  event_id,
  datetime(created_at, 'unixepoch') AS created_at_utc,
  date(created_at, 'unixepoch') AS created_date,
  julianday('now') - julianday(datetime(created_at, 'unixepoch')) AS age_days
FROM events
WHERE created_at >= strftime('%s', 'now', '-14 day')
ORDER BY created_at DESC
LIMIT 25;`,
  },
  {
    id: "sqlite-window-functions",
    label: "Window Functions",
    description: "Ranking rows within partitions.",
    sql: `SELECT
  customer_id,
  order_id,
  order_total,
  rank() OVER (
    PARTITION BY customer_id
    ORDER BY order_total DESC
  ) AS customer_rank,
  dense_rank() OVER (
    PARTITION BY customer_id
    ORDER BY order_total DESC
  ) AS customer_dense_rank,
  count(*) OVER (PARTITION BY customer_id) AS customer_order_count
FROM orders
WHERE order_total > 0;`,
  },
  {
    id: "sqlite-json-functions",
    label: "JSON Functions",
    description: "Extracting values from JSON payloads.",
    sql: `SELECT
  id,
  json_extract(payload, '$.user.id') AS user_id,
  json_extract(payload, '$.event.type') AS event_type,
  json_extract(payload, '$.event.action') AS event_action
FROM audit_log
WHERE json_extract(payload, '$.event.type') = 'login' AND json_extract(payload, '$.user.id') IS NOT NULL
ORDER BY id DESC
LIMIT 100;`,
  },
];

const PERFETTO_PRESETS: SqlPreset[] = [
  {
    id: "perfetto-include-module",
    label: "Include Module",
    description: "Perfetto extension statement for module imports.",
    sql: `INCLUDE PERFETTO MODULE android.startup;
INCLUDE PERFETTO MODULE linux.cpu.idle;`,
  },
  {
    id: "perfetto-create-function",
    label: "Create Perfetto Function",
    description: "Perfetto scalar function declaration.",
    sql: `CREATE PERFETTO FUNCTION top_slice_count(cpu INT, min_dur INT)
RETURNS INT
AS
SELECT count(*)
FROM slice
WHERE cpu = $cpu AND dur >= $min_dur;`,
  },
  {
    id: "perfetto-create-table",
    label: "Create Perfetto Table",
    description: "Perfetto table declaration with SELECT body.",
    sql: `CREATE PERFETTO TABLE hot_slices
AS
SELECT ts, dur, name, track_id
FROM slice
WHERE dur > 1000000
ORDER BY dur DESC
LIMIT 50;`,
  },
  {
    id: "perfetto-create-index",
    label: "Create Perfetto Index",
    description: "Perfetto index statement on a logical table.",
    sql: `CREATE PERFETTO INDEX idx_hot_slices
ON hot_slices(ts, dur, track_id);`,
  },
];

const PYTHON_PRESETS: SqlPreset[] = [
  {
    id: "py-select",
    label: "SELECT Query",
    description: "Python f-string with a parameterized SELECT.",
    sql: `import sqlite3

def get_active_users(conn, min_age):
    cursor = conn.execute(
        f"SELECT id, name, email FROM users WHERE age >= {min_age} AND active = 1 ORDER BY name"
    )
    return cursor.fetchall()
`,
  },
  {
    id: "py-insert",
    label: "INSERT Statement",
    description: "Python f-string with INSERT and multiple values.",
    sql: `import sqlite3

def add_order(conn, customer_id, total):
    conn.execute(
        f"INSERT INTO orders (customer_id, total, created_at) VALUES ({customer_id}, {total}, datetime('now'))"
    )
    conn.commit()
`,
  },
  {
    id: "py-multi",
    label: "Multiple Queries",
    description: "Multiple SQL strings in one Python file.",
    sql: `import sqlite3

def report(conn, department):
    employees = conn.execute(
        f"SELECT name, salary FROM employees WHERE department = {department} ORDER BY salary DESC"
    ).fetchall()

    stats = conn.execute(
        f"SELECT count(*) AS cnt, avg(salary) AS avg_sal FROM employees WHERE department = {department}"
    ).fetchone()

    return employees, stats
`,
  },
];

const TYPESCRIPT_PRESETS: SqlPreset[] = [
  {
    id: "ts-select",
    label: "SELECT Query",
    description: "Template literal with a parameterized SELECT.",
    sql: "import Database from 'better-sqlite3';\n\nfunction getOrders(db: Database, userId: number) {\n  return db.prepare(\n    `SELECT o.id, o.total, o.created_at\n     FROM orders o\n     WHERE o.customer_id = ${userId}\n     ORDER BY o.created_at DESC\n     LIMIT 25`\n  ).all();\n}\n",
  },
  {
    id: "ts-join",
    label: "JOIN Query",
    description: "Template literal with a multi-table JOIN.",
    sql: "function getOrderDetails(db: any, orderId: number) {\n  return db.prepare(\n    `SELECT o.id, c.name AS customer, li.product, li.quantity * li.price AS line_total\n     FROM orders o\n     JOIN customers c ON c.id = o.customer_id\n     JOIN line_items li ON li.order_id = o.id\n     WHERE o.id = ${orderId}`\n  ).all();\n}\n",
  },
  {
    id: "ts-multi",
    label: "Multiple Queries",
    description: "Multiple SQL template literals in one file.",
    sql: "const listUsers = (db: any, active: boolean) =>\n  db.prepare(\n    `SELECT id, name, email FROM users WHERE active = ${active ? 1 : 0} ORDER BY name`\n  ).all();\n\nconst countByRole = (db: any) =>\n  db.prepare(\n    `SELECT role, count(*) AS cnt FROM users GROUP BY role ORDER BY cnt DESC`\n  ).all();\n",
  },
];

const LIBRARIES: Record<string, SqlPresetLibrary> = {
  sqlite: {
    dialectId: "sqlite",
    label: "SQLite",
    presets: SQLITE_PRESETS,
  },
  perfetto: {
    dialectId: "perfetto",
    label: "PerfettoSQL",
    presets: PERFETTO_PRESETS,
  },
  "sqlite:python": {
    dialectId: "sqlite:python",
    label: "Python",
    presets: PYTHON_PRESETS,
  },
  "sqlite:typescript": {
    dialectId: "sqlite:typescript",
    label: "TypeScript",
    presets: TYPESCRIPT_PRESETS,
  },
  "perfetto:python": {
    dialectId: "perfetto:python",
    label: "Python",
    presets: PYTHON_PRESETS,
  },
  "perfetto:typescript": {
    dialectId: "perfetto:typescript",
    label: "TypeScript",
    presets: TYPESCRIPT_PRESETS,
  },
};

export function getSqlPresetLibrary(dialectId: string): SqlPresetLibrary {
  return LIBRARIES[dialectId] ?? LIBRARIES.sqlite;
}
