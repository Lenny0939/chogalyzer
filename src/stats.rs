pub mod bigram_stats;
pub mod trigram_stats;

use crate::{Finger, Key, Stats, INCLUDE_THUMB_ALT, INCLUDE_THUMB_ROLL};
use ahash::AHashMap;

#[must_use]
pub fn analyze(
    mut corpus: String,
    layout_letters: [char; 32],
    command: &str,
    magic_rules: &AHashMap<char, char>,
) -> Stats {
    let layout = layout_raw_to_table(&layout_letters);
    let [mut previous_letter, mut skip_previous_letter, mut epic_previous_letter] = ['_'; 3];
    let mut stats: Stats = Stats::default();
    let mut char_freq: AHashMap<char, u32> = AHashMap::default();
    let finger_weights: AHashMap<Finger, i64> = AHashMap::from([
        (Finger::Pinky, 66),
        (Finger::Ring, 28),
        (Finger::Middle, 21),
        (Finger::Index, 18),
        (Finger::Thumb, 50),
    ]);

    for letter in layout_letters {
        let rule: [char; 2] = match magic_rules.get(&letter) {
            Some(other_letter) => [letter, *other_letter],
            None => [letter, letter],
        };
        corpus = corpus.replace(&rule.iter().collect::<String>(), &format!("{letter}*"));
    }

    for letter_u8 in corpus.as_bytes() {
        let letter = *letter_u8 as char;
        let key = &layout[&letter];
        let previous_key = &layout[&previous_letter];
        let skip_previous_key = &layout[&skip_previous_letter];
        let epic_previous_key = &layout[&epic_previous_letter];
        stats.chars += 1;

        *char_freq.entry(letter).or_insert(0) += 1;

        let bigram = bigram_stats::bigram_stats(
            previous_key,
            key,
            command,
            &mut stats,
            &finger_weights,
            false,
        );
        if bigram.0 {
            *stats
                .ngram_table
                .entry([previous_letter, letter, ' '])
                .or_insert(0) += 1;
        }
        let skipgram = bigram_stats::skipgram_stats(
            skip_previous_key,
            key,
            epic_previous_key,
            command,
            &mut stats,
            &finger_weights,
        );
        if skipgram {
            *stats
                .ngram_table
                .entry([skip_previous_letter, '_', letter])
                .or_insert(0) += 1;
        }
        let trigram =
            trigram_stats::trigram_stats(skip_previous_key, previous_key, key, command, stats);
        stats = trigram.0;
        if trigram.1 {
            *stats
                .ngram_table
                .entry([skip_previous_letter, previous_letter, letter])
                .or_insert(0) += 1;
        }
        if !epic_previous_key.hand == key.hand {
            epic_previous_letter = letter;
        }
        skip_previous_letter = previous_letter;
        previous_letter = letter;
    }
    if !(INCLUDE_THUMB_ALT || INCLUDE_THUMB_ROLL) {
        stats.chars -= stats.thumb_stat;
    }
    #[rustfmt::skip]
    let weighting: [u32; 32] = [
        12, 4, 3, 6, 7, 7, 6, 3, 4, 12, 
        3,  1, 0, 0, 6, 6, 0, 0, 1, 3, 
        8,  9, 8, 4, 9, 9, 4, 8, 9, 8, 
                  0,       0,
    ];
    let max_freq = AHashMap::from([
        (Finger::Pinky, 7),
        (Finger::Ring, 12),
        (Finger::Middle, 13),
        (Finger::Index, 13),
        (Finger::Thumb, 25),
    ]);
    let mut columns: AHashMap<(Finger, u8), u32> = AHashMap::new();
    for i in 0..layout_letters.len() {
        if char_freq.contains_key(&layout_letters[i]) {
            stats.heatmap += i64::from(weighting[i] * char_freq[&layout_letters[i]]);
            let key = &layout[&layout_letters[i]];
            *columns.entry((key.finger.clone(), key.hand)).or_insert(0) += char_freq[&layout_letters[i]];
        }
    }
    for (column, freq) in columns {
        let penalty = f32::max(freq as f32 - max_freq[&column.0] as f32 / 100.0 * stats.chars as f32, 0.0) as i64;
        stats.column_pen += penalty;
    };
    let weights = Stats {
        score: 0.0,
        heatmap: -500,
        //heatmap: 0,
        column_pen: -10000,
        fspeed: -200,
        sfb: 0,
        sfr: 0,
        sfs: 0,
        fsb: -500,
        hsb: -100,
        hss: -20,
        fss: -100,
        lsb: -200,
        lss: -40,
        inroll: 100,
        outroll: 40,
        alt: 0,
        inthreeroll: 320,
        outthreeroll: 160,
        weak_red: -2000,
        red: -300,
        thumb_stat: 0,
        chars: 0,
        skipgrams: 0,
        ngram_table: AHashMap::default(),
        bad_bigrams: AHashMap::default(),
    };
    stats.score = score(&stats, &weights);
    stats
}

#[must_use]
pub fn score(stats: &Stats, weighting: &Stats) -> f64 {
    let mut score = 0;
    score += stats.fspeed * weighting.fspeed / 7;
    score += stats.heatmap * weighting.heatmap / 100;
    score += stats.column_pen * weighting.column_pen;
    score += stats.lsb * weighting.lsb;
    score += stats.lss * weighting.lss;
    score += stats.fsb * weighting.fsb;
    score += stats.fss * weighting.fss;
    score += stats.inroll * weighting.inroll;
    score += stats.inthreeroll * weighting.inthreeroll;
    score += stats.outroll * weighting.outroll;
    score += stats.alt * weighting.alt;
    score += stats.outthreeroll * weighting.outthreeroll;
    score += stats.weak_red * weighting.weak_red;
    score += stats.red * weighting.red;
    score as f64
}

pub fn layout_raw_to_table(layout_raw: &[char; 32]) -> AHashMap<char, Key> {
    #[rustfmt::skip]
    return AHashMap::from([
        // LH top row
        ( layout_raw[0], Key { hand: 0, finger: Finger::Pinky, row: 0, lateral: false, },),
        ( layout_raw[1], Key { hand: 0, finger: Finger::Ring, row: 0, lateral: false, },),
        ( layout_raw[2], Key { hand: 0, finger: Finger::Middle, row: 0, lateral: false, },),
        ( layout_raw[3], Key { hand: 0, finger: Finger::Index, row: 0, lateral: false, },),
        ( layout_raw[4], Key { hand: 0, finger: Finger::Index, row: 0, lateral: true, },),
        // RH top row
        ( layout_raw[5], Key { hand: 1, finger: Finger::Index, row: 0, lateral: true, },),
        ( layout_raw[6], Key { hand: 1, finger: Finger::Index, row: 0, lateral: false, },),
        ( layout_raw[7], Key { hand: 1, finger: Finger::Middle, row: 0, lateral: false, },),
        ( layout_raw[8], Key { hand: 1, finger: Finger::Ring, row: 0, lateral: false, },),
        ( layout_raw[9], Key { hand: 1, finger: Finger::Pinky, row: 0, lateral: false, },),
        // LH middle row
        ( layout_raw[10], Key { hand: 0, finger: Finger::Pinky, row: 1, lateral: false, },),
        ( layout_raw[11], Key { hand: 0, finger: Finger::Ring, row: 1, lateral: false, },),
        ( layout_raw[12], Key { hand: 0, finger: Finger::Middle, row: 1, lateral: false, },),
        ( layout_raw[13], Key { hand: 0, finger: Finger::Index, row: 1, lateral: false, },),
        ( layout_raw[14], Key { hand: 0, finger: Finger::Index, row: 1, lateral: true, },),
        // RH middle row
        ( layout_raw[15], Key { hand: 1, finger: Finger::Index, row: 1, lateral: true, },),
        ( layout_raw[16], Key { hand: 1, finger: Finger::Index, row: 1, lateral: false, },),
        ( layout_raw[17], Key { hand: 1, finger: Finger::Middle, row: 1, lateral: false, },),
        ( layout_raw[18], Key { hand: 1, finger: Finger::Ring, row: 1, lateral: false, },),
        ( layout_raw[19], Key { hand: 1, finger: Finger::Pinky, row: 1, lateral: false, },),
        // LH bottom row
        ( layout_raw[20], Key { hand: 0, finger: Finger::Pinky, row: 2, lateral: false, },),
        ( layout_raw[21], Key { hand: 0, finger: Finger::Ring, row: 2, lateral: false, },),
        ( layout_raw[22], Key { hand: 0, finger: Finger::Middle, row: 2, lateral: false, },),
        ( layout_raw[23], Key { hand: 0, finger: Finger::Index, row: 2, lateral: false, },),
        ( layout_raw[24], Key { hand: 0, finger: Finger::Index, row: 2, lateral: true, },),
        // RH bottom row
        ( layout_raw[25], Key { hand: 1, finger: Finger::Index, row: 2, lateral: true, },),
        ( layout_raw[26], Key { hand: 1, finger: Finger::Index, row: 2, lateral: false, },),
        ( layout_raw[27], Key { hand: 1, finger: Finger::Middle, row: 2, lateral: false, },),
        ( layout_raw[28], Key { hand: 1, finger: Finger::Ring, row: 2, lateral: false, },),
        ( layout_raw[29], Key { hand: 1, finger: Finger::Pinky, row: 2, lateral: false, },),
        // Thumb keys
        ( layout_raw[30], Key { hand: 0, finger: Finger::Thumb, row: 3, lateral: false, },),
        ( layout_raw[31], Key { hand: 1, finger: Finger::Thumb, row: 3, lateral: false, },),
    ]);
}
