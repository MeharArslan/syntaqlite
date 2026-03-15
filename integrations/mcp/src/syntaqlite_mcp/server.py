"""syntaqlite MCP server — exposes format, parse, and validate tools."""

import json
import subprocess

from mcp.server.fastmcp import FastMCP

mcp = FastMCP("syntaqlite")


def _run(args: list[str], sql: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["syntaqlite", *args],
        input=sql,
        capture_output=True,
        text=True,
    )


@mcp.tool()
def format_sql(
    sql: str,
    line_width: int = 80,
    keyword_case: str = "upper",
    semicolons: bool = True,
) -> str:
    """Format a SQL string.

    Args:
        sql: The SQL to format.
        line_width: Maximum line width (default 80).
        keyword_case: Keyword casing — "upper", "lower", or "preserve" (default "upper").
        semicolons: Whether to append trailing semicolons (default True).
    """
    args = [
        "fmt",
        "--line-width",
        str(line_width),
        "--keyword-case",
        keyword_case,
    ]
    args.append(f"--semicolons={'true' if semicolons else 'false'}")
    result = _run(args, sql)
    if result.returncode != 0:
        return f"Error: {result.stderr.strip()}"
    return result.stdout


@mcp.tool()
def parse_sql(sql: str) -> str:
    """Parse a SQL string and return its AST dump.

    Args:
        sql: The SQL to parse.
    """
    result = _run(["parse", "-o", "ast"], sql)
    if result.returncode != 0:
        return f"Error: {result.stderr.strip()}"
    return result.stdout


@mcp.tool()
def validate_sql(sql: str) -> str:
    """Check whether a SQL string is syntactically valid.

    Returns JSON with `valid` (bool) and `errors` (string, empty if valid).

    Args:
        sql: The SQL to validate.
    """
    result = _run(["parse"], sql)
    errors = result.stderr.strip()
    response = {
        "valid": result.returncode == 0 and not errors,
        "errors": errors,
    }
    return json.dumps(response)


if __name__ == "__main__":
    mcp.run()
