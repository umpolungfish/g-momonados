//! Read one glyph word per line on stdin, print `word<TAB>verdict<TAB>register`.
//! Exists so the Rust kernel can be diffed against the Python one word by word
//! instead of only by a failure count.
use std::io::{self, BufRead};

use imasm_core::imasm16_3::{parse_glyph_word, run_word_register, tri_ancestral_verdict};

fn main() {
    for line in io::stdin().lock().lines() {
        let w = line.unwrap();
        let w = w.trim();
        if w.is_empty() { continue; }
        let steps = parse_glyph_word(w);
        let v = tri_ancestral_verdict(&steps).0;
        let r = run_word_register(&steps);
        println!("{w}\t{v}\t{r}");
    }
}
