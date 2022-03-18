# Copyright 2022 Sean Gillespie.
#
# Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
# http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
# <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
# option. This file may not be copied, modified, or distributed
# except according to those terms.
from typing import Any

from gdb.printing import register_pretty_printer, RegexpCollectionPrettyPrinter
import gdb

"""
Python extensions to GDB to assist in debugging a4.
"""


class ValuePrettyPrinter:
    VALUE_MATED = -32768 // 2 + 1
    VALUE_MATE = 32767 // 2
    MATE_DISTANCE_MAX = 50

    val: gdb.Value

    def __init__(self, val: gdb.Value):
        self.val = val

    def to_string(self):
        val = self.val.cast(
            gdb.lookup_type("i16")
        )  # sneakily reads the value from the debuggee process
        if val > self.VALUE_MATE:
            return f"#{self.VALUE_MATE + self.MATE_DISTANCE_MAX - val}"
        elif val < self.VALUE_MATED:
            return f"#-{val - self.VALUE_MATED + self.MATE_DISTANCE_MAX}"
        return f"{val}"


def build_pretty_printers():
    pp = RegexpCollectionPrettyPrinter("a4")
    # haven't found a way to print these two yet without segfaulting gdb
    # pp.add_printer("move", "^a4::core::move::Move$", MovePrettyPrinter)
    # pp.add_printer("position", "^\*mut a4::position::Position$", PositionPrettyPrinter)
    pp.add_printer("value", "^a4::eval::value::Value$", ValuePrettyPrinter)
    return pp


register_pretty_printer(gdb.current_objfile(), build_pretty_printers())
