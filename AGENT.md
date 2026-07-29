# AGENT.md — Session Tracker: Project Vision

## What this is

Session Tracker is a Raidcore Nexus addon for Guild Wars 2 that shows a
live window of the player's GW2 stats — both **session** counts (how much
a stat has increased since a tracking session started) and **lifetime**
totals (the account's all-time value) — side by side, for whichever stats
the player chooses to track.

It is a native-addon counterpart to the Blish HUD module
`ecksofa.sessiontracker` (maintained as `Taschenbuch/BlishHud-SessionTracker`),
and mirrors as much of that module's functionality as possible: same
idea, same underlying data source (the official GW2 API), running as a
Nexus addon instead of inside Blish HUD. Matching BlishHud-SessionTracker's
functionality is the actual target for this addon, not a stretch goal
beyond some smaller MVP.

## The core experience

- A **stats window** lists every stat the player has chosen to track, each
  shown as a row with an icon, a name, a session value, and a lifetime
  value.
- A **stat picker** lets the player search/browse the full catalog of
  trackable stats and choose which ones appear in the stats window, in
  the order they want. Nothing is hardcoded to a fixed list — the catalog
  is broad, and display is entirely user-configurable, mirroring
  BlishHud-SessionTracker's "select stats" panel.
- **Session** values reset to zero at the start of a tracking session and
  count up from there; **lifetime** values are always the player's true
  all-time total as reported by the GW2 API.
- The addon needs a GW2 API key (entered once, stored locally) with
  enough scopes to read the stat categories below.

## Stat catalog (MVP target breadth)

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

Every stat has an id, a display name, an icon, and one of four sources: a
GW2 achievement id, a wallet currency id, an account/character field, or
an item id.

## How it works, conceptually

- Stats are **not** derived from real-time combat parsing (no ArcDPS
  dependency) — they come from polling the official GW2 API
  (achievements, wallet, account, characters, PvP stats) with the
  player's API key, the same mechanism BlishHud-SessionTracker uses. Some
  values lag behind real-time play by the API's own propagation delay,
  same as the reference module.
- **Lifetime** for most stats is simply the current value the API
  reports. Achievement- and deaths-backed stats only ever go up (a lower
  value from a later poll is treated as a transient API glitch and
  ignored), but currency-backed stats are a live wallet balance, not a
  strict all-time total: spending gold, crafting with mystic coins, buying
  with badges of honor, etc. all make the value go *down*, and a session
  that includes a spend will show a negative session delta for that stat.
- **Item-based stats have no achievement or currency to read**, so their
  lifetime value is computed by summing how many of that item id the
  player currently holds: across every character's bags, the account
  bank, shared inventory slots, and material storage (material storage is
  a separate API resource with no bag/slot structure of its own). Like
  currency-backed stats, this count can go *down* as well as up — the
  player can spend, salvage, or deposit the item — so it behaves as a live
  possession count, not a strict all-time total.
- **Session** for a stat is the delta between its current lifetime value
  and its value when the session started — except where a raw
  diff-of-values would be meaningless (a ratio like KDR), which is instead
  computed from its underlying session deltas (session kills ÷ session
  deaths, not lifetime-KDR-now minus lifetime-KDR-then). The same delta
  math applies to currency- and item-based stats; it just isn't guaranteed
  to be a pure "amount gained," since the player may also spend some
  mid-session — a session that spends more than it earns shows as a
  negative value.
- A session can be reset (manually, and/or automatically on a trigger like
  re-entering WvW or a game restart) so the player can measure "how much
  did I get done tonight" or "since I logged in today" however they like.

## What "done" looks like

A player can:
1. Enter their API key once.
2. Open a settings/stat-picker panel, search the full catalog (WvW, PvP,
   currencies, general account, items), and select exactly the stats they
   care about, in whatever order they want.
3. Open the stats window and see each selected stat's session and
   lifetime value update automatically as they play.
4. Reset their session whenever they want a fresh baseline.

That's the MVP: functional parity with what BlishHud-SessionTracker
offers today, delivered as a native Nexus addon.

## Conventions

- Default to zero inline comments. Don't add a comment just because a line
  does something non-trivial, or to explain a design/naming choice that's
  already clear from the code itself. Only write one when a future reader
  could not otherwise infer a genuinely non-obvious constraint or invariant
  — and even then, keep it to a single short line.
- Comments that do get written must describe the current state only. Don't
  narrate history — what an approach used to be, what was tried before, why
  a past attempt failed. That belongs in commit messages, not the file.
- Commit messages are a single line — a summary title, no body.
- Release notes are generated from the commit log by git-cliff
  (`cliff.toml`, run from `.github/workflows/release.yml`) rather than
  from PRs, since most commits land directly on `main`. Prefixing a
  subject with a type — `feat:`, `fix:`, `docs:`, `refactor:`, `perf:`,
  `chore:`/`ci:` — groups it under that heading in the next release's
  notes instead of the catch-all "Other" section; see git-cliff's
  [conventional_commits strategy](https://git-cliff.org/docs/configuration/git/#conventional_commits).
  Not required — an unprefixed commit still shows up, just ungrouped.
