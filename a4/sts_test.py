# Copyright 2021 Sean Gillespie.
#
# Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
# http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
# <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
# option. This file may not be copied, modified, or distributed
# except according to those terms.


from chess import Move
from a4.sts import STSTest


def test_sts_parse():
    epd = '2r2rk1/pb2q2p/1pn1p2p/5p1Q/3P4/P1NB4/1P3PPP/R4RK1 w - - bm d5; id "Undermine.029"; c0 "d5=10, Qxh6=5, Rac1=5, Rfd1=5";'
    test = STSTest.from_epd(epd)
    assert test.fen == "2r2rk1/pb2q2p/1pn1p2p/5p1Q/3P4/P1NB4/1P3PPP/R4RK1 w - - 0 1"
    assert test.best_move == Move.from_uci("d4d5")
    assert test.id == "Undermine.029"
    assert test.score_map[Move.from_uci("d4d5")] == 10
    assert test.score_map[Move.from_uci("h5h6")] == 5
    assert test.score_map[Move.from_uci("a1c1")] == 5
    assert test.score_map[Move.from_uci("f1d1")] == 5
