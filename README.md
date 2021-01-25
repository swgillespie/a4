# Gambit - A Chess Engine

`gambit` is a (work-in-progress) chess engine. It is the fourth incarnation of the `apollo` family of chess engines:

1. [swgillespie/apollo](https://github.com/swgillespie/apollo) contains versions one and two of the `apollo` engine,
   both written in Rust. Version 1 never played chess, but implemented the rules correctly. Version 2 plays chess at
   a beginner level but is plagued by poor search and evaluation. After abandoning version 3, I returned to version 2
   and taught it to play from an opening book, which allowed it to occasionally beat strong beginner players.
2. [swgillespie/apollo3](https://github.com/swgillespie/apollo3) contains version three of the `apollo` engine, this
   time written in C++. Version 3 played chess at a beginner level but was riddled with bugs in its search code.

As the fourth incarnation of `apollo`, `gambit` aims to synthesize all of my learnings from its previous lives and come
away with a stronger chess engine. `gambit` cribbed a lot of code from `apollo` v2.

I've been working on the `apollo` family of chess engines on and off for four years. When I first learned to program the
first large program I attempted to write was a chess engine, but at the time I wasn't a skilled enough programmer to
pull it off. Chess programming is near and dear to my heart and I have a ton of fun hacking on this project.

Some technical details and decisions about `gambit`, as informed by its previous lives:

1. `gambit` is a **copy-make** engine. There are two approaches to applying a move to a chess position; either you copy
   the game state and destructively modify it to reflect the move, or you incrementally update the game state and *unmake*
   any changes you made to undo the move. If you think about it, in theory copy-make is probably slower than make-unmake,
   but `apollo3` was a make-unmake and it was slower than `apollo2`. I implemented make-unmake in `gambit`, measured it,
   and found that Rust's `clone` implementation beats it no matter how much I optimize `unmake`. Furthermore, the `unmake`
   function is hard to write and hard to get correct, while `clone` is obvious and impossible to mess up.
2. `gambit` is a **UCI** engine. UCI is the standard protocol for chess GUIs to interact with engines. It's what Stockfish
   uses, and if it's good enough for Stockfish, it's good enough for me. There's also a bunch of nice tooling for engines
   that speak UCI such as integration into chess GUIs (so you can actually play the bot) to scripting. `apollo` v2 had a
   program that hosted two copies of itself and made them play eachother, for testing changes that could influence the
   strength of the engine.
3. `gambit` is **parallel**. Chess is a famously hard problem to parallelize. The strategy that `gambit` aims to use
   is dubbed **Lazy-SMP**; the goal is to have a single shared hashtable data structure that caches information about
   specific positions (the *transposition table*) and have a number of threads search using it. It's not the most
   theoretically beautiful algorithm, but it does produce effective speedups on a SMP machine. apollo v2 attempted to
   parallelize its search using `rayon` and `crossbeam`, but this proved difficult to get correct and `rayon`'s use
   of small tasks is not particularly well-suited to the problem of searching chess positions.

Gambit's current state is that it fully implements the game of chess (with lots of tests). Evaluation and search is
currently in progress. Code for dealing with UCI is likely to be lifted directly from `apollo` v2.
