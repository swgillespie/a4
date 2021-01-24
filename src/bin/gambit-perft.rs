use structopt::StructOpt;

use gambit::movegen;
use gambit::Position;

#[derive(Debug, StructOpt)]
struct Options {
    /// The depth to search to.
    #[structopt(short, long)]
    depth: u32,

    /// FEN representation of the position to analyze.
    #[structopt(name = "FEN")]
    fen: String,

    /// If set, use the position's legality test to test for legal moves instead of looking for check. Probably faster,
    /// useful for finding bugs in the legality tester.
    #[structopt(long)]
    use_legality_test: bool,
}

pub fn perft(pos: &Position, depth: u32, use_legality_test: bool) -> u64 {
    if depth == 0 {
        return 1;
    }

    let mut moves = Vec::new();
    movegen::generate_moves(pos.side_to_move(), pos, &mut moves);
    return moves
        .iter()
        .map(|&mov| {
            if use_legality_test {
                if pos.is_legal_given_pseudolegal(mov) {
                    let mut new_pos = pos.clone();
                    //let side_to_move = pos.side_to_move();
                    new_pos.make_move(mov);
                    /*
                    if new_pos.is_check(side_to_move) {
                        println!("{}", pos);
                        println!("move {} is not legal, but legality check said yes", mov);
                        panic!();
                    }
                    */
                    perft(&new_pos, depth - 1, use_legality_test)
                } else {
                    /*
                    let mut new_pos = pos.clone();
                    let side_to_move = pos.side_to_move();
                    new_pos.apply_move(mov);
                    if !new_pos.is_check(side_to_move) {
                        println!("{}", pos);
                        println!("move {} is legal, but legality check said no", mov);
                        panic!();
                    } else {
                        0
                    }
                    */
                    0
                }
            } else {
                let mut new_pos = pos.clone();
                let side_to_move = pos.side_to_move();
                new_pos.make_move(mov);
                if !new_pos.is_check(side_to_move) {
                    perft(&new_pos, depth - 1, use_legality_test)
                } else {
                    0
                }
            }
        })
        .sum();
}

fn main() {
    let ops = Options::from_args();
    let pos = Position::from_fen(ops.fen).unwrap();
    let count = perft(&pos, ops.depth, ops.use_legality_test);
    println!("{}", count);
}
