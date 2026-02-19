// select_columns: parse SQL and resolve output columns, expanding * using
// schema knowledge from CREATE TABLE / CREATE VIEW / CTEs.
//
// This is an example / test of the syntaqlite C API ergonomics.

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>
#include <unordered_map>
#include <vector>

#include "syntaqlite/parser.h"
#include "syntaqlite/sqlite.h"
#include "syntaqlite/sqlite_node.h"
#include "syntaqlite/sqlite_tokens.h"

// ── Helpers ─────────────────────────────────────────────────────────────

// Extract a std::string from a source span. Returns "" if absent.
static std::string span_str(SyntaqliteParser* p, SyntaqliteSourceSpan span) {
  uint32_t len;
  const char* text = syntaqlite_span_text(p, span, &len);
  if (!text)
    return {};
  return {text, len};
}

// Case-insensitive string for table name lookups.
static std::string lower(std::string s) {
  for (auto& c : s)
    c = (char)std::tolower((unsigned char)c);
  return s;
}

// ── Schema ──────────────────────────────────────────────────────────────

// A resolved output column: name + optional source table.
struct Column {
  std::string name;
  std::string source_table;  // which table/view/cte it came from (for *)
};

// Maps lowercase table/view/cte name → list of columns.
using Schema = std::unordered_map<std::string, std::vector<Column>>;

// ── Forward declarations ────────────────────────────────────────────────

static std::vector<Column> resolve_select_columns(
    SyntaqliteParser* p,
    const SyntaqliteSelectStmt* select,
    const Schema& schema);

static std::vector<Column> resolve_stmt_columns(SyntaqliteParser* p,
                                                uint32_t stmt_id,
                                                const Schema& schema);

// ── FROM clause: collect table sources ──────────────────────────────────

struct TableSource {
  std::string name;             // the table/view/cte/subquery name
  std::string alias;            // alias if present, else same as name
  std::vector<Column> columns;  // resolved columns for this source
};

static void collect_from_sources(SyntaqliteParser* p,
                                 uint32_t from_id,
                                 const Schema& schema,
                                 std::vector<TableSource>& out) {
  if (!syntaqlite_node_is_present(from_id))
    return;

  const auto* node =
      (const SyntaqliteTableSource*)syntaqlite_parser_node(p, from_id);

  switch ((SyntaqliteNodeTag)node->tag) {
    case SYNTAQLITE_NODE_TABLE_REF: {
      const auto& ref = node->table_ref;
      std::string name = span_str(p, ref.table_name);
      std::string alias =
          syntaqlite_span_is_present(ref.alias) ? span_str(p, ref.alias) : name;
      std::vector<Column> cols;
      auto it = schema.find(lower(name));
      if (it != schema.end()) {
        cols = it->second;
        // Update source_table to the alias
        for (auto& c : cols)
          c.source_table = alias;
      }
      out.push_back({name, alias, std::move(cols)});
      break;
    }

    case SYNTAQLITE_NODE_SUBQUERY_TABLE_SOURCE: {
      const auto& sub = node->subquery_table_source;
      std::string alias = span_str(p, sub.alias);
      auto cols = resolve_stmt_columns(p, sub.select, schema);
      for (auto& c : cols)
        c.source_table = alias;
      out.push_back({"(subquery)", alias, std::move(cols)});
      break;
    }

    case SYNTAQLITE_NODE_JOIN_CLAUSE: {
      const auto& join = node->join_clause;
      collect_from_sources(p, join.left, schema, out);
      collect_from_sources(p, join.right, schema, out);
      break;
    }

    case SYNTAQLITE_NODE_JOIN_PREFIX: {
      const auto& jp = node->join_prefix;
      collect_from_sources(p, jp.source, schema, out);
      break;
    }

    default:
      break;
  }
}

// ── Expression → column name ────────────────────────────────────────────

// Try to infer a column name from an expression (for unnamed result columns).
static std::string expr_name(SyntaqliteParser* p, uint32_t expr_id) {
  if (!syntaqlite_node_is_present(expr_id))
    return "?";

  const auto* expr = (const SyntaqliteExpr*)syntaqlite_parser_node(p, expr_id);

  switch ((SyntaqliteNodeTag)expr->tag) {
    case SYNTAQLITE_NODE_COLUMN_REF:
      return span_str(p, expr->column_ref.column);

    case SYNTAQLITE_NODE_LITERAL:
      return span_str(p, expr->literal.source);

    case SYNTAQLITE_NODE_FUNCTION_CALL: {
      auto name = span_str(p, expr->function_call.func_name);
      return name + "(...)";
    }

    case SYNTAQLITE_NODE_AGGREGATE_FUNCTION_CALL: {
      auto name = span_str(p, expr->aggregate_function_call.func_name);
      return name + "(...)";
    }

    case SYNTAQLITE_NODE_CAST_EXPR:
      return "CAST(" + expr_name(p, expr->cast_expr.expr) + ")";

    case SYNTAQLITE_NODE_SUBQUERY_EXPR:
      return "(subquery)";

    default: {
      // Fall back to source text span from the parser source.
      // For binary/unary exprs, just describe the type.
      if (expr->tag == SYNTAQLITE_NODE_BINARY_EXPR)
        return "(" + expr_name(p, expr->binary_expr.left) + " op " +
               expr_name(p, expr->binary_expr.right) + ")";
      if (expr->tag == SYNTAQLITE_NODE_UNARY_EXPR)
        return "op(" + expr_name(p, expr->unary_expr.operand) + ")";
      return "<expr>";
    }
  }
}

// ── Resolve SELECT columns ──────────────────────────────────────────────

static std::vector<Column> resolve_select_columns(
    SyntaqliteParser* p,
    const SyntaqliteSelectStmt* select,
    const Schema& schema) {
  std::vector<Column> result;

  // Gather FROM sources for * expansion.
  std::vector<TableSource> sources;
  collect_from_sources(p, select->from_clause, schema, sources);

  if (!syntaqlite_node_is_present(select->columns))
    return result;

  const void* col_list = syntaqlite_parser_node(p, select->columns);
  uint32_t count = syntaqlite_list_count(col_list);

  for (uint32_t i = 0; i < count; i++) {
    const auto* rc =
        (const SyntaqliteResultColumn*)syntaqlite_list_child(p, col_list, i);

    if (rc->flags.bits.star) {
      // table.* or * — alias field holds the table qualifier if present.
      std::string qualifier;
      if (syntaqlite_span_is_present(rc->alias)) {
        qualifier = span_str(p, rc->alias);
      }

      if (qualifier.empty()) {
        // SELECT * — expand all tables in FROM order.
        bool expanded = false;
        for (const auto& src : sources) {
          if (!src.columns.empty()) {
            for (const auto& c : src.columns)
              result.push_back(c);
            expanded = true;
          }
        }
        if (!expanded) {
          result.push_back({"*", ""});
        }
      } else {
        // SELECT t.* — find matching source.
        bool found = false;
        for (const auto& src : sources) {
          if (strcasecmp(src.alias.c_str(), qualifier.c_str()) == 0 ||
              strcasecmp(src.name.c_str(), qualifier.c_str()) == 0) {
            if (!src.columns.empty()) {
              for (const auto& c : src.columns)
                result.push_back(c);
            } else {
              result.push_back({qualifier + ".*", qualifier});
            }
            found = true;
            break;
          }
        }
        if (!found) {
          result.push_back({qualifier + ".*", qualifier});
        }
      }
      continue;
    }

    // Regular column — use alias if present, otherwise infer from expr.
    std::string name;
    if (syntaqlite_span_is_present(rc->alias)) {
      name = span_str(p, rc->alias);
    } else {
      name = expr_name(p, rc->expr);
    }
    result.push_back({name, ""});
  }

  return result;
}

// ── Resolve any statement's columns ─────────────────────────────────────

static std::vector<Column> resolve_stmt_columns(SyntaqliteParser* p,
                                                uint32_t stmt_id,
                                                const Schema& schema) {
  if (!syntaqlite_node_is_present(stmt_id))
    return {};

  const auto* stmt = (const SyntaqliteStmt*)syntaqlite_parser_node(p, stmt_id);

  switch ((SyntaqliteNodeTag)stmt->tag) {
    case SYNTAQLITE_NODE_SELECT_STMT:
      return resolve_select_columns(p, &stmt->select_stmt, schema);

    case SYNTAQLITE_NODE_WITH_CLAUSE: {
      const auto& with = stmt->with_clause;
      // Build a local schema with CTE definitions added.
      Schema local = schema;

      if (syntaqlite_node_is_present(with.ctes)) {
        const void* cte_list = syntaqlite_parser_node(p, with.ctes);
        uint32_t cte_count = syntaqlite_list_count(cte_list);

        for (uint32_t i = 0; i < cte_count; i++) {
          const auto* cte =
              (const SyntaqliteCteDefinition*)syntaqlite_list_child(p, cte_list,
                                                                    i);

          std::string cte_name = lower(span_str(p, cte->cte_name));

          // If CTE has explicit column names, use those.
          if (syntaqlite_node_is_present(cte->columns)) {
            const void* name_list = syntaqlite_parser_node(p, cte->columns);
            uint32_t nc = syntaqlite_list_count(name_list);
            std::vector<Column> cols;
            for (uint32_t j = 0; j < nc; j++) {
              const auto* col_ref =
                  (const SyntaqliteColumnRef*)syntaqlite_list_child(
                      p, name_list, j);
              cols.push_back({span_str(p, col_ref->column), cte_name});
            }
            local[cte_name] = std::move(cols);
          } else {
            // Resolve from the CTE's SELECT body.
            auto cols = resolve_stmt_columns(p, cte->select, local);
            for (auto& c : cols)
              c.source_table = cte_name;
            local[cte_name] = std::move(cols);
          }
        }
      }

      return resolve_stmt_columns(p, with.select, local);
    }

    case SYNTAQLITE_NODE_COMPOUND_SELECT: {
      // UNION etc — columns come from the left side.
      return resolve_stmt_columns(p, stmt->compound_select.left, schema);
    }

    case SYNTAQLITE_NODE_VALUES_CLAUSE:
      return {};  // VALUES doesn't have named columns.

    default:
      return {};
  }
}

// ── Process a top-level statement ───────────────────────────────────────

static int process_statement(SyntaqliteParser* p,
                             uint32_t root_id,
                             Schema& schema) {
  const auto* stmt = (const SyntaqliteStmt*)syntaqlite_parser_node(p, root_id);

  switch ((SyntaqliteNodeTag)stmt->tag) {
    case SYNTAQLITE_NODE_CREATE_TABLE_STMT: {
      const auto& ct = stmt->create_table_stmt;
      std::string table_name = lower(span_str(p, ct.table_name));

      if (syntaqlite_node_is_present(ct.columns)) {
        const void* col_list = syntaqlite_parser_node(p, ct.columns);
        uint32_t count = syntaqlite_list_count(col_list);
        std::vector<Column> cols;

        for (uint32_t i = 0; i < count; i++) {
          const auto* col_def =
              (const SyntaqliteColumnDef*)syntaqlite_list_child(p, col_list, i);
          cols.push_back({span_str(p, col_def->column_name), table_name});
        }
        schema[table_name] = std::move(cols);
        printf("registered table '%s'\n", table_name.c_str());
      } else if (syntaqlite_node_is_present(ct.as_select)) {
        // CREATE TABLE ... AS SELECT
        auto cols = resolve_stmt_columns(p, ct.as_select, schema);
        for (auto& c : cols)
          c.source_table = table_name;
        schema[table_name] = std::move(cols);
        printf("registered table '%s' (from SELECT)\n", table_name.c_str());
      }
      return 0;
    }

    case SYNTAQLITE_NODE_CREATE_VIEW_STMT: {
      const auto& cv = stmt->create_view_stmt;
      std::string view_name = lower(span_str(p, cv.view_name));

      // If view has explicit column names, use those.
      if (syntaqlite_node_is_present(cv.column_names)) {
        const void* name_list = syntaqlite_parser_node(p, cv.column_names);
        uint32_t count = syntaqlite_list_count(name_list);
        std::vector<Column> cols;
        for (uint32_t i = 0; i < count; i++) {
          const auto* col_ref =
              (const SyntaqliteColumnRef*)syntaqlite_list_child(p, name_list,
                                                                i);
          cols.push_back({span_str(p, col_ref->column), view_name});
        }
        schema[view_name] = std::move(cols);
      } else {
        // Resolve from the view's body.
        auto cols = resolve_stmt_columns(p, cv.select, schema);
        for (auto& c : cols)
          c.source_table = view_name;
        schema[view_name] = std::move(cols);
      }
      printf("registered view '%s'\n", view_name.c_str());
      return 0;
    }

    case SYNTAQLITE_NODE_SELECT_STMT:
    case SYNTAQLITE_NODE_WITH_CLAUSE:
    case SYNTAQLITE_NODE_COMPOUND_SELECT: {
      auto cols = resolve_stmt_columns(p, root_id, schema);
      printf("output columns (%zu):\n", cols.size());
      for (size_t i = 0; i < cols.size(); i++) {
        printf("  [%zu] %s", i + 1, cols[i].name.c_str());
        if (!cols[i].source_table.empty()) {
          printf("  (from %s)", cols[i].source_table.c_str());
        }
        printf("\n");
      }
      return 0;
    }

    case SYNTAQLITE_NODE_EXPLAIN_STMT:
      printf("EXPLAIN statement — not a data query\n");
      return 0;

    default:
      fprintf(stderr, "skipping non-query statement (tag=%d)\n", stmt->tag);
      return 0;
  }
}

// ── Main ────────────────────────────────────────────────────────────────

int main(int argc, char** argv) {
  const char* sql = nullptr;

  if (argc >= 2) {
    sql = argv[1];
  } else {
    static char buf[64 * 1024];
    size_t n = fread(buf, 1, sizeof(buf) - 1, stdin);
    buf[n] = '\0';
    sql = buf;
  }

  SyntaqliteParser* p = syntaqlite_create_sqlite_parser(nullptr);
  syntaqlite_parser_reset(p, sql, (uint32_t)strlen(sql));

  Schema schema;
  SyntaqliteParseResult result;
  int stmt_num = 0;

  while ((result = syntaqlite_parser_next(p)).root != SYNTAQLITE_NULL_NODE) {
    if (result.error) {
      fprintf(stderr, "parse error: %s\n",
              result.error_msg ? result.error_msg : "unknown");
      syntaqlite_parser_destroy(p);
      return 1;
    }

    stmt_num++;
    if (stmt_num > 1)
      printf("\n");
    process_statement(p, result.root, schema);
  }

  if (stmt_num == 0) {
    if (result.error) {
      fprintf(stderr, "parse error: %s\n",
              result.error_msg ? result.error_msg : "unknown");
    } else {
      fprintf(stderr, "error: no SQL statement provided\n");
    }
    syntaqlite_parser_destroy(p);
    return 1;
  }

  syntaqlite_parser_destroy(p);
  return 0;
}
