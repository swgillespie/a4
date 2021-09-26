use structopt::StructOpt;

use a4::movegen;
use a4::Position;

#[derive(Debug, StructOpt)]
struct Options {
    /// FEN representation of the position to analyze.
    #[structopt(name = "FEN")]
    fen: String,
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
