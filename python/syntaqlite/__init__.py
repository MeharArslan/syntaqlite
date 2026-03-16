"""syntaqlite — SQLite SQL tools."""

import os
import stat
import subprocess
import sys

__version__ = "0.0.36"


def get_binary_path():
    """Return the path to the bundled syntaqlite binary."""
    binary = os.path.join(os.path.dirname(__file__), "bin", "syntaqlite")
    if sys.platform == "win32":
        binary += ".exe"

    # Ensure binary is executable on Unix (wheel extraction strips permissions)
    if sys.platform != "win32":
        current_mode = os.stat(binary).st_mode
        if not (current_mode & stat.S_IXUSR):
            os.chmod(
                binary,
                current_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH,
            )

    return binary


def main():
    """Execute the bundled binary."""
    binary = get_binary_path()

    if sys.platform == "win32":
        sys.exit(subprocess.call([binary] + sys.argv[1:]))
    else:
        os.execvp(binary, [binary] + sys.argv[1:])


