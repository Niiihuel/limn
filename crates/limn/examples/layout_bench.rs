//! What a long list that is not virtualized costs a frame: building its view,
//! and laying it out. The numbers a change to the layout is judged by; see
//! `agents/workflow.md` for how to run it so two runs can be compared.
//!
//! ```sh
//! cargo build --release -p limn --example layout_bench
//! taskset -c 6 target/release/examples/layout_bench          # 100, 1,000 and 4,000 rows
//! taskset -c 6 target/release/examples/layout_bench 8000     # the rows you name
//! ```
//!
//! Text is measured from the default metrics, with no font, so the time is
//! the tree's own.

use std::hint::black_box;
use std::time::{Duration, Instant};

use limn::{Button, Component, ComponentPalette, Element, Size, UiTree};

/// Frames timed for each row count; the best one is reported, since anything
/// slower is the machine's noise.
const FRAMES: usize = 30;

fn view(rows: usize, palette: ComponentPalette) -> Element {
    let rows = (0..rows)
        .map(|row| {
            Element::row(vec![
                Element::text(format!("Row {row}")),
                Element::spacer().flex(1.0),
                Button::new(format!("open-{row}"), "Open", palette).render(),
            ])
        })
        .collect();
    Element::scroll("rows", Element::column(rows))
}

fn main() {
    let named: Vec<usize> = std::env::args().skip(1).filter_map(|rows| rows.parse().ok()).collect();
    let counts = if named.is_empty() { vec![100, 1000, 4000] } else { named };
    let palette = ComponentPalette::neutral_dark();
    let viewport = Size::new(1280.0, 800.0);

    println!("{:>6}  {:>9}  {:>11}  {:>8}", "rows", "view (ms)", "layout (ms)", "commands");
    for rows in counts {
        let mut tree = UiTree::new(view(rows, palette));
        // Entrances over: every frame timed is a frame at rest.
        tree.set_reduced_motion(true);
        let mut commands = tree.layout(viewport).commands.len();
        let (mut best_view, mut best_layout) = (Duration::MAX, Duration::MAX);
        for _ in 0..FRAMES {
            let start = Instant::now();
            let root = black_box(view(rows, palette));
            let built = Instant::now();
            // Replacing the root frees the last one: part of a frame's cost.
            tree.set_root(root);
            commands = black_box(tree.layout(viewport)).commands.len();
            best_view = best_view.min(built - start);
            best_layout = best_layout.min(built.elapsed());
        }
        println!(
            "{rows:>6}  {:>9.3}  {:>11.3}  {commands:>8}",
            best_view.as_secs_f64() * 1e3,
            best_layout.as_secs_f64() * 1e3,
        );
    }
}
