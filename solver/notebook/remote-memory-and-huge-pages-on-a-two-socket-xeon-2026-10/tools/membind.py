#!/usr/bin/env python3
"""Runs a command with its memory bound to one NUMA node, as `numactl --membind=NODE` would: sets the thread's memory
policy to MPOL_BIND on NODE (set_mempolicy(2), which execve keeps and every thread the command starts inherits) and
execs the command.  A policy without MPOL_F_MOF also keeps the kernel's automatic NUMA balancing from migrating the
command's pages, so they stay on NODE wherever its threads run.  Usage: membind.py NODE COMMAND [ARGS...]"""
import ctypes
import os
import sys

SYS_SET_MEMPOLICY = 238  # x86_64
MPOL_BIND = 2

node = int(sys.argv[1])
libc = ctypes.CDLL(None, use_errno=True)
mask = ctypes.c_ulong(1 << node)
if libc.syscall(SYS_SET_MEMPOLICY, MPOL_BIND, ctypes.byref(mask), ctypes.c_ulong(65)) != 0:
    sys.exit(f"set_mempolicy: {os.strerror(ctypes.get_errno())}")
os.execvp(sys.argv[2], sys.argv[2:])
