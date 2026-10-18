// Cross-check only: expected values come from the exact-rational Python oracle.
use std::io::{self, BufRead};
fn main() {
    let mut count = 0;
    let mut disagreements = 0;
    for line in io::stdin().lock().lines() {
        let line = line.unwrap();
        let mut fields = line.splitn(3, '\t');
        let _group = fields.next().unwrap();
        let expected = fields.next().unwrap();
        let token = fields.next().unwrap();
        let value: f64 = token.parse().unwrap();
        let actual = if value.is_infinite() { "OVERFLOW".to_owned() }
                     else { format!("{:016X}", value.to_bits()) };
        count += 1;
        if actual != expected {
            disagreements += 1;
            println!("DISAGREEMENT\t{expected}\t{actual}\t{token}");
        }
    }
    println!("TASK096_RUST_CROSSCHECK count={count} disagreements={disagreements}");
    if disagreements != 0 { std::process::exit(1); }
}