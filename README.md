# Based
Based is a toy browser engine I'm building simply to learn more about a few concepts. Namely: I wanted to learn `vello`, a pure rust 2d rendering library I thought was interesting. Additionally just how web browsers work behind the scenes. Based may not be organized in the way that a traditional web browser would be, but it's how I would implement it, which I honestly find more valuable. Finally, as I was implementing text rendering I realized how little I know about fonts, so it will also be a glyph shaping/text layout engine. I've been thinking about writing a JS engine from scratch for the project as well, but that might be too much. For the time being, once I get around to JS support it'll probably use boa.

## Try it out
I mean, if you want to i guess. If you're on nix(os), it's as simple as 
```sh
nix-shell
cargo run
```
which will automatically parse and open a basic frontend with tests/basic.html. If you want to change up the file, you'll have to head over [here](https://github.com/rvvvr/based/blob/master/src/context/mod.rs#L30-L33) and change the url yourself. I'll make it a bit easier to choose a page at some point in the future.

If you're not on nix(os), you can still check out shell.nix to see what dependencies you might need, and install them with your favourite package manager. After that, just throw a `cargo run` into your own console, and witness the glory that is based.

## Where are the tests?
Unit tests are hard to write, so I have a separate branch on my local machine where I'm slowly writing them out when I get the motivation. I'll merge and upstream once the code is mostly covered. Same goes for documentation.
