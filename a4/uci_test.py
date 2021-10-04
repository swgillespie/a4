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

from chess.engine import INFO_ALL, Limit, UciProtocol
from chess import Board
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


@pytest.mark.asyncio
async def test_analysis(a4_debug: UciProtocol):
    board = Board()
    info = await a4_debug.analyse(board, Limit(time=0.1))
    assert "nodes" in info
    assert "score" in info


@pytest.mark.asyncio
async def test_play(a4_debug: UciProtocol):
    board = Board()
    result = await a4_debug.play(board, Limit(time=0.1), info=INFO_ALL)
    assert result.move is not None
    assert "nodes" in result.info
