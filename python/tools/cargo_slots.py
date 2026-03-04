# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

"""Shared cargo target-directory slot pool.

Prevents multiple concurrent cargo invocations from contending on the same
target-directory lock by giving each caller its own slot.  Slots are reused
across builds so the incremental cache is preserved.

Usage::

    from python.tools.cargo_slots import acquire_slot

    with acquire_slot() as target_dir:
        subprocess.run(cmd, env={**os.environ, "CARGO_TARGET_DIR": target_dir})
"""

from __future__ import annotations

import fcntl
import os
from collections.abc import Generator
from contextlib import contextmanager

ROOT_DIR: str = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

_SLOTS_BASE: str = os.path.join(ROOT_DIR, "target", "cargo-slots")
_NUM_SLOTS: int = 8


@contextmanager
def acquire_slot() -> Generator[str, None, None]:
    """Yield a free ``CARGO_TARGET_DIR`` path, blocking until one is available.

    Does a non-blocking linear scan first; if all slots are busy, blocks on
    slot 0 (the first to become free wins).
    """
    os.makedirs(_SLOTS_BASE, exist_ok=True)

    open_fds = []
    try:
        # Non-blocking pass: grab the first free slot.
        for i in range(_NUM_SLOTS):
            slot_dir = os.path.join(_SLOTS_BASE, f"slot-{i}")
            os.makedirs(slot_dir, exist_ok=True)
            fd = open(os.path.join(_SLOTS_BASE, f"slot-{i}.lock"), "w")
            open_fds.append(fd)
            try:
                fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
                try:
                    yield slot_dir
                    return
                finally:
                    fcntl.flock(fd, fcntl.LOCK_UN)
            except OSError:
                pass  # slot busy — keep fd open, try next

        # All slots busy: block on slot 0.
        fcntl.flock(open_fds[0], fcntl.LOCK_EX)
        try:
            yield os.path.join(_SLOTS_BASE, "slot-0")
        finally:
            fcntl.flock(open_fds[0], fcntl.LOCK_UN)

    finally:
        for fd in open_fds:
            try:
                fd.close()
            except OSError:
                pass
