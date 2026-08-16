# Session Tracker

[![CI](https://github.com/albrektsson/gw2-session-tracker/actions/workflows/ci.yml/badge.svg)](https://github.com/albrektsson/gw2-session-tracker/actions/workflows/ci.yml)
[![CodeQL](https://github.com/albrektsson/gw2-session-tracker/actions/workflows/github-code-scanning/codeql/badge.svg)](https://github.com/albrektsson/gw2-session-tracker/actions/workflows/github-code-scanning/codeql)
[![Dependabot Updates](https://github.com/albrektsson/gw2-session-tracker/actions/workflows/dependabot/dependabot-updates/badge.svg)](https://github.com/albrektsson/gw2-session-tracker/actions/workflows/dependabot/dependabot-updates)
![GitHub all releases](https://img.shields.io/github/downloads/albrektsson/gw2-session-tracker/total)

A [Raidcore Nexus](https://raidcore.gg/Nexus) addon for Guild Wars 2 that
shows a window of GW2 stats, each tracked as both a **session** value (how
much it's increased since a tracking session started) and a **lifetime**
value (the account's all-time total). It's a native-addon counterpart to
the Blish HUD module `ecksofa.sessiontracker`
([`Taschenbuch/BlishHud-SessionTracker`](https://github.com/Taschenbuch/BlishHud-SessionTracker)),
aiming to mirror as much of its functionality as possible. See
[`CONTEXT.md`](./CONTEXT.md) for the project's glossary of terms.

## Product vision

- A **stats window** lists every stat the player has chosen to track, each
  shown as a row with an icon, a name, a session value, and a lifetime
  value.
- A **stat picker** lets the player search/browse the full catalog of
  trackable stats and choose which ones appear in the stats window, in
  the order they want. Nothing is hardcoded to a fixed list — the catalog
  is broad, and display is entirely user-configurable, mirroring
  BlishHud-SessionTracker's "select stats" panel.
- The addon needs a GW2 API key (entered once, stored locally) with
  enough scopes to read the stat categories below.

The trackable catalog spans every category BlishHud-SessionTracker covers:

- **WvW**: kills, deaths, KDR, WvW rank, camps/towers/keeps/castles/
  objectives captured & defended, dolyaks killed/escorted, supply spent on
  repairs, WvW-specific currencies (badges of honor, skirmish claim
  tickets, WvW tickets, etc.)
- **PvP**: matches won/lost, rank, PvP-specific currencies
- **General account**: currencies (gold, gems, karma, laurels, etc.),
  achievement points, unlocked achievements, and other account-wide
  achievement- or wallet-backed stats the API exposes
- **Items**: specific items obtained through play (e.g. Heavy Loot Bags
  from WvW, Memory of Battle) that have no achievement or currency behind
  them, tracked by counting how many the player currently holds

**Done** means functional parity with what BlishHud-SessionTracker offers
today: enter an API key once, pick exactly the stats you care about from
the full catalog, watch each one's session and lifetime value update live,
and reset your session whenever you want a fresh baseline.

## Screenshots

| Main window | Select Stats |
|---|---|
| ![Main window](./images/session_tracker.png) | ![Select Stats tab](./images/stat_selector.png) |

| Arrange Stats | General settings |
|---|---|
| ![Arrange Stats tab](./images/stat_sorter.png) | ![General settings tab](./images/settings.png) |

## Structure

A 3-crate Cargo workspace:

- **`crates/core`** (`session_tracker_core`) — pure logic: GW2 API
  response parsing, the stat catalog, session/lifetime delta math, and
  config file persistence. No Nexus or Windows dependency, so it builds
  and tests on any platform.
- **`crates/net`** (`session_tracker_net`) — the GW2 API HTTP client
  (via `ureq`) and the background polling thread/shared app state. Still
  fully cross-platform.
- **`crates/addon`** (`session_tracker`) — the actual Nexus addon
  (`cdylib`). Depends on [`nexus-rs`](https://github.com/Zerthox/nexus-rs)
  for the ImGui windows, keybinds, and addon entry point. This is the
  only crate that's Windows-only and needs cross-compilation.

## Building

Native logic (`core`/`net`) builds and tests on any platform with a
standard Rust toolchain:

```sh
cargo test -p session_tracker_core -p session_tracker_net
```

The addon itself (`session_tracker`) is Windows-only and needs the
`x86_64-pc-windows-gnu` target plus a MinGW-w64 cross-compiler, since it
depends on `nexus-rs` (which pulls in the `windows` crate and a vendored
C++ ImGui backend):

```sh
rustup target add x86_64-pc-windows-gnu
# install a mingw-w64 cross toolchain, e.g.:
#   brew install mingw-w64          (Homebrew)
#   dnf install mingw64-gcc mingw64-gcc-c++   (Fedora)

cargo build --release --target x86_64-pc-windows-gnu -p session_tracker
```

This produces `target/x86_64-pc-windows-gnu/release/session_tracker.dll`.
`.cargo/config.toml` already sets up the linker and static-linking flags
needed to avoid runtime MinGW DLL dependencies — no extra flags needed.
Copy the built DLL into your GW2 install's Nexus `addons` folder to load
it in-game.

## Releasing

Releases are tag-triggered: pushing a `v*` tag runs
[`release.yml`](./.github/workflows/release.yml), which builds
`session_tracker.dll` in release mode for `x86_64-pc-windows-msvc` and
publishes a GitHub release for that tag with the DLL attached and notes
auto-generated from the commits since the previous tag. There's no manual
step on GitHub — the tag push is the entire release.

All three crates share one version via `[workspace.package]` in the root
`Cargo.toml` (`version.workspace = true` in each crate), so there's a
single place to bump. That version is what Nexus displays next to the
addon's name in-game.

To cut a release, from a clean working tree on `main`:

```sh
./scripts/release.sh X.Y.Z
```

This bumps the workspace version, refreshes `Cargo.lock`, commits, pushes,
tags, and pushes the tag — the `Release` workflow picks it up from there.
Watch it run in the Actions tab; the GitHub release appears once it
finishes.

## License

MIT — see [`LICENSE`](./LICENSE).

Some stat icons (`crates/addon/assets/icons/`) are adapted from
[Taschenbuch/BlishHud-SessionTracker](https://github.com/Taschenbuch/BlishHud-SessionTracker),
licensed under MIT.

The Session Timer icon (`stopwatch.png`) is the
["Stopwatch" icon](https://game-icons.net/1x1/skoll/stopwatch.html) by Skoll
from [game-icons.net](https://game-icons.net), licensed under
[CC BY 3.0](https://creativecommons.org/licenses/by/3.0/).

The Distance Traveled icon (`run.png`) is the
["Run" icon](https://game-icons.net/1x1/lorc/run.html) by Lorc from
[game-icons.net](https://game-icons.net), licensed under
[CC BY 3.0](https://creativecommons.org/licenses/by/3.0/).

The Combat Time icon (`crossed-swords.png`) is the
["Crossed Swords" icon](https://game-icons.net/1x1/lorc/crossed-swords.html)
by Lorc from [game-icons.net](https://game-icons.net), licensed under
[CC BY 3.0](https://creativecommons.org/licenses/by/3.0/).

## Contributing

Contributions are welcome — please open a pull request, or open an issue
first if you'd like to discuss a change before writing it.
