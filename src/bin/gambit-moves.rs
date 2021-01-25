use structopt::StructOpt;

use gambit::movegen;
use gambit::Position;

#[derive(Debug, StructOpt)]
struct Options {
    /// FEN representation of the position to analyze.
    #[structopt(name = "FEN")]
    fen: String,

    /// If set, use the position's legality test to test for legal moves instead of looking for check. Probably faster,
    /// useful for finding bugs in the legality tester.
    #[structopt(long)]
    use_legality_test: bool,
}

fn main() {
    let ops = Options::from_args();
    let pos = Position::from_fen(ops.fen).unwrap();
    let mut moves = Vec::new();
    movegen::generate_moves(pos.side_to_move(), &pos, &mut moves);
    moves.retain(|&m| pos.is_legal_given_pseudolegal(m));
    for mov in moves {
        println!("{}", mov.as_uci());
    }
}
