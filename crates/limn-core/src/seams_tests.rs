//! The seams of the engine: what it opens for `limn-components` and for no
//! host, hidden from its reference.

/// How many there are. A seam is a debt (R23): each marks a component that
/// only limn could build, since a host cannot rely on what is hidden. When
/// one closes, lower this; it never goes up.
const SEAMS: usize = 70;

#[test]
fn seams_do_not_grow() {
    // Put together here so that this file holds none.
    let mark = concat!("#[doc(", "hidden)]");
    let source = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
    let found: usize = std::fs::read_dir(source)
        .expect("the engine's source")
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|extension| extension == "rs"))
        .map(|entry| std::fs::read_to_string(entry.path()).expect("a source file").matches(mark).count())
        .sum();
    assert!(found <= SEAMS, "{found} seams where {SEAMS} are allowed: a component built on a new seam is one a host could not build (R23)");
    assert_eq!(found, SEAMS, "seams went down: lower SEAMS so they stay down");
}
