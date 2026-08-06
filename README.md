# Sokoban

A crate pushing puzzle game built with the [Nightshade](https://github.com/matthewjberger/nightshade) engine.

Push crates onto goals across multi storey boards, and do it with ice, pits,
one way arrows, belts, fragile and brittle floors, water, incinerators, spikes,
breakable boulders, glass, mirrors that bend a beam, gems that fill sockets and
turn on coloured light, prisms and splitters that tint and divide it, locks and
shutters that answer to it, watchers that kill anything standing beside them,
elevators between storeys, and a party of characters who each do one thing the
others cannot.

Every board is data. `levels/campaign.json` is the campaign, `levels/gallery.json`
is the gallery of worked examples, and both are read by the same rules the game
plays by, so a board that ships is a board the solver has finished.

## Quickstart

```bash
# native
just run

# wasm (webgpu)
just run-wasm

# steam deck
just build-steamdeck
just deploy-steamdeck
```

> All chromium-based browsers like Brave, Vivaldi, Chrome, etc support WebGPU.
> Firefox also [supports WebGPU](https://mozillagfx.wordpress.com/2025/07/15/shipping-webgpu-on-windows-in-firefox-141/) now starting with version `141`.

## Checks

The shipped boards answer to a set of readings taken off the data rather than a
test suite, and they run through the same binary the game does:

```bash
just gate      # lint, then every check below that has to pass
just analyze   # solves every campaign board and compares it to its recorded par
just lessons   # plays every gallery demonstration through the rules
just demos     # solves every gallery board and writes the worked example back
just random 3  # generates boards from every preset and every hazard setting
just story     # reads the campaign in order and says what each board is about
```

## Prerequisites

* [just](https://github.com/casey/just)
* [trunk](https://trunkrs.dev/) (for web builds)
* [cross](https://github.com/cross-rs/cross) (for Steam Deck builds)

> Run `just` with no arguments to list all commands

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
