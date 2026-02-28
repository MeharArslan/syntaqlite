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
};

export function getSqlPresetLibrary(dialectId: string): SqlPresetLibrary {
  return LIBRARIES[dialectId] ?? LIBRARIES.sqlite;
}
