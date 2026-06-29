use arbiter::count_probes;

#[test]
fn count_probes_no_match_returns_zero() {
    assert_eq!(count_probes("ship it"), 0);
}

#[test]
fn count_probes_case_insensitive_returns_hits() {
    assert!(count_probes("EXPLAIN this please") >= 1);
}

#[test]
fn count_probes_representative_phrases_each_detected() {
    let samples = [
        "why did you choose this",
        "can you explain the approach",
        "what happens if it fails",
        "are you sure this is correct",
        "please double check the logic",
        "that's wrong, revert it",
        "what are the trade-offs",
        "did you consider the edge cases",
        "should we do it instead",
        "i don't understand this part",
    ];
    for sample in samples {
        assert!(count_probes(sample) >= 1, "no probe detected in: {sample}");
    }
}
