# Copyright 2021 Sean Gillespie.
#
# Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
# http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
# <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
# option. This file may not be copied, modified, or distributed
# except according to those terms.

"""
Integration tests for the UCI protocol implementation by a4.
"""

from chess.engine import UciProtocol
from a4.uci import popen_debug
import pytest


@pytest.fixture
async def a4_debug():
    engine = await popen_debug()
    try:
        yield engine
    finally:
        await engine.quit()


@pytest.mark.asyncio
async def test_open(a4_debug: UciProtocol):
    assert a4_debug.id["name"].startswith("a4")
    assert "author" in a4_debug.id


@pytest.mark.asyncio
async def test_ping(a4_debug: UciProtocol):
    await a4_debug.ping()


@pytest.mark.asyncio
async def test_debug(a4_debug: UciProtocol):
    a4_debug.debug(on=True)
    a4_debug.debug(on=False)
