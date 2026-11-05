//! Prints the teacher table (`content/chem/evidence.csv`, I-058): one row per
//! codex reaction with its equation, band, what the player sees, evidence and
//! source. Run through `scripts/evidence-csv.sh`.

fn main() {
    print!("{}", realgame_chem::evidence::csv_for_content());
}
