# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

"""SQLite code extraction utilities.

Provides tools for working with SQLite's grammar and source code,
including running Lemon, diffing upstream grammar, and text transforms.
"""

from .transforms import (
    Transform,
    Pipeline,
    ReplaceText,
    TruncateAt,
    RemoveRegex,
    ReplaceRegex,
    RemoveSection,
    SymbolRename,
    SymbolRenameExact,
    RemoveFunctionCalls,
    StripBlessingComment,
)
from .tools import (
    ToolRunner,
    create_symbol_rename_pipeline,
    create_parser_symbol_rename_pipeline,
    create_keywordhash_rename_pipeline,
)
from .grammar_build import (
    build_synq_grammar,
    split_extension_grammar,
    parse_actions_content,
    parse_extension_keywords,
)

__all__ = [
    # transforms.py
    "Transform",
    "Pipeline",
    "ReplaceText",
    "TruncateAt",
    "RemoveRegex",
    "ReplaceRegex",
    "RemoveSection",
    "SymbolRename",
    "SymbolRenameExact",
    "RemoveFunctionCalls",
    "StripBlessingComment",
    # tools.py
    "ToolRunner",
    "create_symbol_rename_pipeline",
    "create_parser_symbol_rename_pipeline",
    "create_keywordhash_rename_pipeline",
    # grammar_build.py
    "build_synq_grammar",
    "split_extension_grammar",
    "parse_actions_content",
    "parse_extension_keywords",
]
