"""Fail-closed Linux Landlock write isolation for experimental consumer builds.

Read/execute are unrestricted. Mutation is allowed only beneath the explicitly
listed private consumer/support/state/tmp roots. Shared provider trees are not
on that list. This also restricts chmod-capable owners and inherited children.
Requires Landlock ABI >= 3 (Linux >= 6.2); not used by ordinary qualification.
"""
import ctypes
import os
from pathlib import Path


def restrict_writes(writable):
    libc = ctypes.CDLL(None, use_errno=True)
    # x86_64 Linux syscall numbers; this pilot already requires this target.
    create, add, enforce = 444, 445, 446
    abi = libc.syscall(create, 0, 0, 1)
    if abi < 3:
        raise RuntimeError(f"Landlock ABI >= 3 required, got {abi}")
    # WRITE_FILE, REMOVE_DIR/FILE, MAKE_{CHAR,DIR,REG,SOCK,FIFO,BLOCK,SYM},
    # REFER and TRUNCATE. Read/execute are intentionally unhandled.
    rights = (1 << 1) | sum(1 << bit for bit in range(4, 15))

    class Ruleset(ctypes.Structure):
        _fields_ = [("handled_access_fs", ctypes.c_uint64)]

    class PathRule(ctypes.Structure):
        _pack_ = 1
        _fields_ = [("allowed_access", ctypes.c_uint64), ("parent_fd", ctypes.c_int32)]

    def check(value):
        if value < 0:
            error = ctypes.get_errno()
            raise OSError(error, os.strerror(error))
        return value

    attributes = Ruleset(rights)
    ruleset = check(libc.syscall(create, ctypes.byref(attributes), ctypes.sizeof(attributes), 0))
    try:
        for directory in writable:
            fd = os.open(Path(directory).resolve(strict=True), os.O_PATH | os.O_CLOEXEC)
            try:
                allowed = rights if Path(directory).is_dir() else (1 << 1) | (1 << 14)
                rule = PathRule(allowed, fd)
                check(libc.syscall(add, ruleset, 1, ctypes.byref(rule), 0))
            finally:
                os.close(fd)
        check(libc.prctl(38, 1, 0, 0, 0))  # PR_SET_NO_NEW_PRIVS
        check(libc.syscall(enforce, ruleset, 0))
    finally:
        os.close(ruleset)