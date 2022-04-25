# Copyright 2022 Sean Gillespie.
#
# This file is part of a4's bookgen.
# a4's bookgen is free software: you can redistribute it and/or modify it under
# the terms of the GNU Affero General Public License as published by the Free
# Software Foundation, either version 2 of the License, or (at your option) any
# later version.
#
# a4's bookgen is distributed in the hope that it will be useful, but WITHOUT
# ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
# FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License along
# with a4's bookgen. If not, see <https://www.gnu.org/licenses/>.

from typing import List
import json
import requests
import backoff

BASE_URL = "https://explorer.lichess.ovh/masters"


@backoff.on_exception(backoff.expo, requests.exceptions.RequestException)
def process_prefix(moves: List[str]):
    params = {"play": ",".join(moves)}
    resp = requests.get(BASE_URL, params=params)
    print(f"{resp.url} -> {resp.status_code}")
    resp.raise_for_status()
    body = resp.json()
    doc = {}
    doc["total"] = body["white"] + body["draws"] + body["black"]
    if doc["total"] < 50000:
        return None

    doc_moves = []
    for mov in body["moves"]:
        entry = {}
        entry["count"] = mov["white"] + mov["draws"] + mov["black"]
        entry["move"] = mov["uci"]
        entry["children"] = process_prefix(moves + [mov["uci"]])
        doc_moves.append(entry)
    for mov in doc_moves:
        mov["probability"] = mov["count"] / doc["total"]
    doc["moves"] = doc_moves
    return doc


def main():
    document = process_prefix([])
    with open("src/book.json", "w") as book:
        json.dump(document, book, indent=2)


if __name__ == "__main__":
    main()
