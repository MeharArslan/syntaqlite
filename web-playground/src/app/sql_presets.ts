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
    description: "Simple projection with filter and ordering.",
    sql: `SELECT a, b
FROM t
WHERE c = 1
ORDER BY a;`,
  },
  {
    id: "sqlite-string-functions",
    label: "String Functions",
    description: "Common SQLite string function usage.",
    sql: `SELECT
  id,
  lower(trim(name)) AS name_norm,
  substr(email, 1, 12) AS email_prefix,
  coalesce(phone, 'n/a') AS phone
FROM users
WHERE length(name) > 2;`,
  },
  {
    id: "sqlite-date-functions",
    label: "Date Functions",
    description: "Date/time helpers and arithmetic.",
    sql: `SELECT
  event_id,
  datetime(created_at, 'unixepoch') AS created_at_utc,
  julianday('now') - julianday(datetime(created_at, 'unixepoch')) AS age_days
FROM events
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
  ) AS customer_rank
FROM orders;`,
  },
  {
    id: "sqlite-json-functions",
    label: "JSON Functions",
    description: "Extracting values from JSON payloads.",
    sql: `SELECT
  id,
  json_extract(payload, '$.user.id') AS user_id,
  json_extract(payload, '$.event.type') AS event_type
FROM audit_log
WHERE json_extract(payload, '$.event.type') = 'login';`,
  },
];

const PERFETTO_PRESETS: SqlPreset[] = [
  {
    id: "perfetto-include-module",
    label: "Include Module",
    description: "Perfetto extension statement for module imports.",
    sql: `INCLUDE PERFETTO MODULE android.startup;`,
  },
  {
    id: "perfetto-create-function",
    label: "Create Perfetto Function",
    description: "Perfetto scalar function declaration.",
    sql: `CREATE PERFETTO FUNCTION top_slice_count(cpu INT)
RETURNS INT
AS
SELECT count(*)
FROM slice
WHERE cpu = $cpu;`,
  },
  {
    id: "perfetto-create-table",
    label: "Create Perfetto Table",
    description: "Perfetto table declaration with SELECT body.",
    sql: `CREATE PERFETTO TABLE hot_slices
USING SPAN_JOIN
AS
SELECT ts, dur, name
FROM slice
WHERE dur > 1000000;`,
  },
  {
    id: "perfetto-create-index",
    label: "Create Perfetto Index",
    description: "Perfetto index statement on a logical table.",
    sql: `CREATE PERFETTO INDEX idx_hot_slices
ON hot_slices(ts, dur);`,
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
