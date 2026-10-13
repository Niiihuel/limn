# Tokens

The corner radii and animation durations of the whole library, set once for
the process. Color is not a token: it travels with each component as a
palette, see [Scheme](scheme.md).

## Import

```rust
use limn::{set_tokens, tokens, Motion, Radius, Tokens};
```

## Usage

Call `set_tokens` with the shape and pace of the application. Elements read
the tokens when they are built, so an application that rebuilds its tree
every frame changes on the next one.

```rust
set_tokens(Tokens {
    radius: Radius::DEFAULT.scaled(1.5),
    motion: Motion::DEFAULT,
});
```

`Tokens::DEFAULT` is `Radius::DEFAULT` with `Motion::DEFAULT`.

## Radius

```rust
set_tokens(Tokens { radius: Radius::SQUARE, ..Tokens::DEFAULT });
```

A `Radius` has four fields in logical pixels: `control` (4), `surface` (6),
`card` (8) and `panel` (12). `Radius::SQUARE` is zero everywhere, and
`scaled(factor)` multiplies all four. A radius that is negative or not finite
falls back to its default when the tokens are set.

## Motion

```rust
set_tokens(Tokens { motion: Motion::DEFAULT.scaled(0.5), ..Tokens::DEFAULT });
```

A `Motion` has four durations in milliseconds: `hover_ms` (120), `enter_ms`
(140), `exit_ms` (110) and `move_ms` (170). `scaled(2.0)` is half speed and
`scaled(0.5)` is double. A field set to zero makes that kind of change
immediate.

The tokens set the pace. To switch movement off altogether, call
`UiTree::set_reduced_motion(true)`.

## Reading the tokens

```rust
let radius = tokens().radius;
Element::panel(content).radius(radius.card)
```

`tokens()` returns the tokens in force, for an element of your own that
should round like the library's. A component that lets its caller choose a
radius takes a `RadiusRole` (`Control`, `Surface`, `Card`, `Panel` or
`Square`), and `role.px()` reads the radius in force for it.

## See also

[Tokens](../tokens.md) lists every value, what each one applies to and what
the tokens do not govern.
