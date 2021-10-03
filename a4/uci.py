# Copyright 2021 Sean Gillespie.
#
# Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
# http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
# <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
# option. This file may not be copied, modified, or distributed
# except according to those terms.

from pathlib import Path
from chess.engine import UciProtocol, popen_uci


async def popen(path: Path) -> UciProtocol:
    _, engine = await popen_uci(str(path))
    return engine


async def popen_release() -> UciProtocol:
    return await popen(Path("target") / Path("release") / Path("a4"))


async def popen_debug() -> UciProtocol:
    return await popen(Path("target") / Path("debug") / Path("a4"))
