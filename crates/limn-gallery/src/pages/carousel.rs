use super::{caption, ground, section, Cx, Section};
use limn::{Avatar, Carousel, Component, ComponentPalette, Element, Icon, Link, MediaTile, Skeleton};

const ALBUMS: [(&str, &str); 12] = [
    ("Kind of Blue", "Miles Davis"),
    ("Blue Train", "John Coltrane"),
    ("Mingus Ah Um", "Charles Mingus"),
    ("Time Out", "The Dave Brubeck Quartet"),
    ("Moanin'", "Art Blakey"),
    ("Somethin' Else", "Cannonball Adderley"),
    ("Maiden Voyage", "Herbie Hancock"),
    ("Speak No Evil", "Wayne Shorter"),
    ("Saxophone Colossus", "Sonny Rollins"),
    ("Song for My Father", "Horace Silver"),
    ("The Shape of Jazz to Come", "Ornette Coleman"),
    ("Getz/Gilberto", "Stan Getz, João Gilberto"),
];

const TILE: f32 = 144.0;

pub fn view(cx: &Cx) -> Vec<Section> {
    // The edges fade into the page the shelves stand on.
    let palette = ComponentPalette { surface: ground(cx), ..cx.palette };
    // The gallery ships no pictures: a cover here is a glyph on a fill.
    let cover = |icon| Element::icon(icon).large_icon(40.0).foreground(palette.text).background(palette.raised);
    let albums = |prefix: &str, count: usize| -> Vec<Element> {
        (0..count)
            .map(|i| {
                let (title, artist) = ALBUMS[i % ALBUMS.len()];
                let icon = [Icon::Library, Icon::Playlist, Icon::Heart][i % 3];
                MediaTile::new(format!("{prefix}:{i}"), title, palette).artwork(cover(icon)).subtitle(artist).width(TILE).render()
            })
            .collect()
    };
    let artists = ["Nina Simone", "Billie Holiday", "Ella Fitzgerald", "Sarah Vaughan", "Abbey Lincoln", "Etta James", "Dinah Washington", "Carmen McRae"];
    let artists: Vec<Element> = artists
        .iter()
        .enumerate()
        .map(|(i, name)| {
            MediaTile::new(format!("carousel-artist:{i}"), *name, palette).artwork(cover(Icon::Microphone)).subtitle("Artist").round(true).width(TILE).render()
        })
        .collect();
    let coming: Vec<Element> = (0..8)
        .map(|_| {
            Element::column(vec![
                Skeleton::block("carousel-coming", palette, TILE, TILE).render(),
                Skeleton::line("carousel-coming", palette, TILE * 0.7).render(),
                Skeleton::line("carousel-coming", palette, TILE * 0.45).render(),
            ])
            .gap(8.0)
        })
        .collect();
    let all = |id: &str| Link::new(id, "Show all", palette).render();
    vec![
        section(cx, "Carousel", vec![
            Carousel::new("carousel-albums", albums("carousel-album", 12), palette)
                .title("Jazz classics")
                .trailing(all("carousel-albums-all"))
                .render(),
            caption(cx, "Drag it anywhere, on a tile too, and let go with speed: it glides on and slows to rest. A trackpad's sideways scroll and Shift with the wheel pan it; the wheel scrolls the page. A tile that is pressed without moving is clicked."),
        ]),
        section(cx, "Eyebrow and title", vec![
            Carousel::new("carousel-mixes", albums("carousel-mix", 10), palette)
                .eyebrow("Made for Ana")
                .title("Your top mixes")
                .trailing(all("carousel-mixes-all"))
                .render(),
            caption(cx, ".eyebrow(text) over .title(text): what the row is to the reader, over its name."),
        ]),
        section(cx, "Beside a picture", vec![
            Carousel::new("carousel-artists", artists, palette)
                .leading(Avatar::new("Nina Simone", palette).render())
                .eyebrow("More like")
                .title("Nina Simone")
                .render(),
            caption(cx, ".leading(element): a picture held round in 48 × 48 beside the heading, as an artist's."),
        ]),
        section(cx, "A long shelf", vec![
            Carousel::new("carousel-long", albums("carousel-long-album", 60), palette)
                .title("Sixty albums")
                .gap(16.0)
                .inset(16.0)
                .render(),
            caption(cx, ".gap(16.0).inset(16.0): the first tile lines up with a 16 px gutter and the row still scrolls under it, to the edges, where it fades over what is hidden."),
        ]),
        section(cx, "While it loads", vec![
            Carousel::new("carousel-loading", coming, palette).title("Recently played").render(),
            caption(cx, "Placeholders are items like any other: Skeleton blocks while the covers are on their way."),
        ]),
        section(cx, "Long title", vec![
            Carousel::new("carousel-long-title", albums("carousel-long-title-album", 6), palette)
                .title("The albums everyone in the house keeps putting on when friends come over")
                .trailing(all("carousel-long-title-all"))
                .render(),
            caption(cx, "A title that does not fit is cut with an ellipsis, and the trailing link keeps its place."),
        ]),
    ]
}
