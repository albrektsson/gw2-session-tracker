# Overlay customization + per-hour rate/history: implementation spec

Full functional parity with `ecksofa.sessiontracker`'s overlay-customization settings
(row format, sizing/position, colors, window behavior, gold format) plus its per-hour
rate and session history-log feature. Resolved via the wayfinder map
[Overlay customization + per-hour/history stats: parity with ecksofa.sessiontracker](https://github.com/albrektsson/gw2-session-tracker/issues/7);
each section below traces back to the closed ticket that resolved it.

Reference screenshots: [settings panel](https://pkgs.blishhud.com/metadata/img/profile/311874811-60c2b005-ecda-4e72-bd09-a1c0cde7bdb4.png),
[main window + hover tooltip](https://pkgs.blishhud.com/metadata/img/profile/311875024-293e9477-fe91-4920-bc66-125254e205fc.png).

New `CONTEXT.md` vocabulary this effort introduces: **Session Rate**, **History Snapshot**
(both already recorded in `CONTEXT.md`).

## Crate architecture ([#18](https://github.com/albrektsson/gw2-session-tracker/issues/18))

- `session_rate(id: &str) -> f64` — a method on `SessionTracker` (`crates/core/src/session.rs`),
  alongside `session_value`/`lifetime_value`. Derives from `elapsed()` + the existing
  session-value delta; no new state.
- History-snapshot storage is a new core type, `SessionHistory`, composed as a field on
  `SessionTracker` (not flattened into its scalar fields). Lives in `core`
  (`session.rs` or a sibling `history.rs`); cleared on `reset()`.
- Nothing moves to `net` or `addon` — `addon` only ever reads `SessionTracker`'s output to
  render, same relationship it already has to `session_value`. Follows
  `docs/adr/0001-category-stays-in-core.md`'s precedent.

## Per-hour rate ([#8](https://github.com/albrektsson/gw2-session-tracker/issues/8))

`session_rate = session_value / elapsed_hours`, computed identically everywhere it's
shown (main window row, hover tooltip) — a whole-session average, not a rolling window.
Not defined for Ratio Stats or for Session Timer/Combat Time (see the applicability
table under Row format below).

## History log ([#9](https://github.com/albrektsson/gw2-session-tracker/issues/9), [#21](https://github.com/albrektsson/gw2-session-tracker/issues/21))

- **Persistence**: in-memory, session-scoped. Clears on Reset Session like the rest of
  `SessionTracker`. No on-disk format.
- **Interval**: every 5 minutes, riding the addon's existing 60-second poll cadence
  (`crates/addon/src/lib.rs:115`) as every 5th tick — no new timer infra.
- **Retention**: unbounded for the session (~144 entries/stat over 12 hours at 5-minute
  granularity — not worth a cap/eviction policy).
- **Scope**: full catalog (all 806 `StatDef`s), not filtered to configured Stat Lists —
  mirrors `SessionTracker.lifetime`'s existing catalog-wide shape (fed by
  `build_snapshot`). Worst case ~930KB for a 12-hour session.
- Each entry is a **History Snapshot**: a Stat's Session Value (and Session Rate where
  applicable) at a point in time. Distinguished from `api.rs`'s unrelated `ApiSnapshot`.

## Row format ([#10](https://github.com/albrektsson/gw2-session-tracker/issues/10), [#11](https://github.com/albrektsson/gw2-session-tracker/issues/11))

Replaces today's hardcoded `icon, session, lifetime` row shape with a fully composable
ordered field list + separator.

**Field set**: Icon, Name, Session value, Lifetime value, Session Rate. No separate
spacer/blank field. Label modes (icon-only/text-only/icon+text) are eliminated as a
distinct concept — they fall out for free from including/excluding Icon/Name in the
ordered list.

**Separator**: one global choice, applied uniformly between every field in the row (not
configurable per adjacent-field-pair). Preset list (`|`, `/`, `-`, space) plus a
free-text override.

**Scope**: global, not configurable per Stat List — a display preference, not a
per-context one.

**Per-stat field applicability** — not every field applies to every stat:

| Stat | Icon | Name | Session | Lifetime | Rate |
|---|---|---|---|---|---|
| `session_timer` | Y | Y | Y | - | - |
| `combat_time` | Y | Y | Y | - | - |
| `distance_traveled` | Y | Y | Y | - | Y |
| `kdr` / `pvp_kdr` | Y | Y | Y | Y | - |
| everything else | Y | Y | Y | Y | Y |

When a field doesn't apply to a given stat, it's **omitted from that row entirely** —
value and its neighboring separator both drop — rather than shown as a placeholder or
dash. Rows are allowed to have fewer segments than others.

## Window sizing/layout ([#12](https://github.com/albrektsson/gw2-session-tracker/issues/12))

Adopted ecksofa's 5 knobs 1:1 (font/icon size excluded — already covered by
`text_scale`):

| Field | Type | Range | Default | Notes |
|---|---|---|---|---|
| `fixed_window_height` | bool | — | `false` | |
| `window_height` | f32 | 50.0–800.0 | 200.0 | only applied/editable when `fixed_window_height` is true |
| `window_right_margin` | f32 | 0.0–50.0 | 0.0 | extra space after each row's rightmost content, independent of `padding` |
| `padding` | f32 | 0.0–30.0 | 8.0 | maps to ImGui's `WindowPadding` style var (8.0 matches ImGui's own default) |
| `fix_label_width` | bool | — | `false` | |
| `label_width` | f32 | 20.0–300.0 | 80.0 | reserves a fixed-width column for the Name field so values align; no-op when Name isn't in the active field list |

`always_auto_resize(true)` stays unconditional. When `fixed_window_height` is true,
additionally apply `.size_constraints([0.0, window_height], [f32::MAX, window_height])`
(clamps height, width keeps auto-sizing) plus `.scroll_bar(true)` (restores the native
scrollbar that `.no_decoration()`'s `NO_SCROLLBAR` would otherwise suppress). No
`hide_scrollbar`/`scrollbar_fix` knobs — those work around BlishHud's custom-drawn
chrome, a bug class that doesn't apply to `arcdps-imgui`'s native scrollbar.

## Color config ([#13](https://github.com/albrektsson/gw2-session-tracker/issues/13))

`text_color` is removed outright (no back-compat alias) and replaced:

| Field | Type | Default | Notes |
|---|---|---|---|
| `label_color` | `[f32; 4]` | `[1.0, 0.85, 0.3, 1.0]` | Name field text |
| `value_color` | `[f32; 4]` | `[1.0, 0.85, 0.3, 1.0]` | Session/Lifetime/Rate value text |
| `background_color` | `[f32; 3]` (RGB; alpha stays `background_opacity`) | `[0.0, 0.0, 0.0]` | combined with `background_opacity` at render time |
| `bold_text` | `bool` (unchanged) | `false` | single global toggle, applies to both label and value draws |
| `icon_color` | unchanged | — | out of scope for this ticket |

Uses this addon's existing `ColorEdit` widget convention, not ecksofa's named-preset
dropdown. Rendering: replace `.bg_alpha(state.background_opacity)` with
`ui.push_style_color(StyleColor::WindowBg, [background_color[0..3], background_opacity])`.

## Window anchor/position + drag toggle ([#14](https://github.com/albrektsson/gw2-session-tracker/issues/14), [#15](https://github.com/albrektsson/gw2-session-tracker/issues/15))

| Field | Type | Default | Notes |
|---|---|---|---|
| `window_anchor` | enum `TopLeft \| TopRight \| BottomLeft \| BottomRight` | `TopLeft` | 4 corners only |
| `window_offset` | `[f32; 2]` | `[20.0, 20.0]` | pixel offset from the anchored corner; not a settings-panel slider — set implicitly by dragging |
| `window_drag_enabled` | bool | `false` | locked by default |

**Anchor semantics**: stays pinned through resizes. Every frame (when locked), the
window's top-left is computed from `(window_anchor, window_offset, current auto-resized
size)` and forced via `SetNextWindowPos(_, Condition::Always)` — e.g. a `BottomRight`
anchor keeps that corner fixed as rows are added. Not left to ImGui's ini-based
position memory; `window_anchor`/`window_offset` are explicit persisted `AppState`
fields.

**Drag interaction**:
- **Locked** (`window_drag_enabled == false`, default): position forced every frame,
  window immovable.
- **Unlocked**: `SetNextWindowPos` is skipped for that frame so ImGui's native drag
  takes over; after the window is built, its actual position is read back
  (`ui.window_pos()`), converted to an anchor-relative offset, and written into
  `window_offset`. Re-locking resumes from exactly where the drag left off.

**Anchor changes**: switching `window_anchor` recomputes `window_offset` to preserve
the window's current absolute screen position relative to the new corner, rather than
resetting to the default offset.

## Quick-access menu icon ([#15](https://github.com/albrektsson/gw2-session-tracker/issues/15))

- `menu_icon_enabled: bool`, default `true` — registers/deregisters via
  `nexus::api::quick_access::add_quick_access`/`remove_quick_access` as the flag changes.
- **Texture**: `images/session_tracker.png`, loaded via
  `nexus::texture::get_texture_or_create_from_file` (same pattern as
  `crates/addon/src/ui/stat_icon.rs`). Same texture for normal/hover states.
- **Click behavior**: left-click triggers the existing `SESSION_TRACKER_TOGGLE_MAIN`
  keybind (`crates/addon/src/lib.rs`, default `ALT+SHIFT+W`) via `add_quick_access`'s
  `keybind_identifier` param — toggles main window visibility.
- No right-click context menu (settings stays reachable via
  `SESSION_TRACKER_TOGGLE_SETTINGS`, default `ALT+SHIFT+E`).

## Click-through ([#19](https://github.com/albrektsson/gw2-session-tracker/issues/19) research, [#20](https://github.com/albrektsson/gw2-session-tracker/issues/20) design)

**Feasibility** (research): achievable via Dear ImGui's `NoMouseInputs` flag —
`Window::mouse_inputs(false)` / `.no_inputs()` on the vendored Rust binding, no
`unsafe`/raw `sys::` call, no Nexus API, no Win32 hack. Wiring point:
`crates/addon/src/ui/main_window.rs:56-60`. Produces real OS-level pass-through because
Nexus's input router (`UiInput.cpp::WndProc`) gates every mouse message on ImGui's
aggregate `io.WantCaptureMouse`; excluding the window from ImGui's hover test lets
clicks fall through to GW2. All-or-nothing per-window toggle (matches Blish HUD).
Full writeup: `docs/research/click-through-feasibility.md`.

**Config**:
- `click_through_enabled: bool`, default `false`.
- **Mutually exclusive with `window_drag_enabled`**, enforced in the settings UI
  (grey out one while the other is enabled) rather than picking a render-time
  precedence rule — a click-through window can't receive mouse events, so it can't
  also be dragged.
- Placement: grouped immediately adjacent to `window_drag_enabled` in Window Behavior
  (see Settings organization below) so the greyed-out state reads clearly.
- Scope: applies only to the main stat-list window; the settings window always stays
  interactive.

## Gold/coin format pattern ([#16](https://github.com/albrektsson/gw2-session-tracker/issues/16))

`format_coin` becomes pattern-driven.

- `coin_format: String`, default `"{g}g {s}s {c}c"` (renders identically to today's
  fixed output).
- **Tokens**: bracket-delimited `{g}`, `{s}`, `{c}`; everything else in the pattern is
  copied through literally. `{g}` is thousands-separated via `format_thousands`
  (unchanged); `{s}`/`{c}` are unpadded (matches today exactly — `"0g 0s 33c"`, not
  `"0g 00s 33c"`). No zero-padded token variant. No `{sign}` token — the negative-value
  `-` prefix stays automatic outside the pattern.
- **Validation**: malformed = unbalanced braces or a token name other than `g`/`s`/`c`.
  On Save, a malformed pattern is rejected (`PollStatus::Error`, not persisted),
  leaving the previously-saved valid pattern in effect — matches the API key field's
  validate-on-save convention.

## Hide-zero-value-stats ([#17](https://github.com/albrektsson/gw2-session-tracker/issues/17))

- `hide_zero_stats: bool`, default `false`, one global toggle across all Stat Lists.
- **Zero condition**: AND — hidden only when **both** Session Value and Lifetime Value
  are zero. Stats with no Lifetime Value (`session_timer`, `combat_time`,
  `distance_traveled`) fall back to Session-alone.
- **Layout**: a hidden stat is fully removed from layout — no reserved blank line,
  remaining rows pack upward. Orthogonal to `fixed_window_height`/`fix_label_width`.

## Hover tooltip layout ([#22](https://github.com/albrektsson/gw2-session-tracker/issues/22))

Replaces the current one-line `ui.tooltip_text(stat.display_name)` stub in
`crates/addon/src/ui/main_window.rs`. Prototyped 3 variants against 3 cases (normal
stat, MumbleLink timer, Ratio Stat) — layout mockup:
`docs/prototypes/hover-tooltip-layout.html`.

- **Structure**: icon+name header, then a "Lifetime" section and a "Session" section
  (Session Value, Session Rate, Duration), each field on its own line, using this
  repo's own vocabulary (not Blish's "Total"/"SESSION" wording). Inapplicable rows
  omitted entirely: MumbleLink timers get no Lifetime section; Ratio Stats get no
  Session Rate line.
- **No description line** — `StatDef` has no description field; left as fog (see Open
  questions below) rather than decided against.
- **History table**: capped to the 10 most recent History Snapshots, no scroll —
  oldest entries fall off the bottom. Keeps tooltip height fixed regardless of session
  length; unbounded retention stays in `SessionHistory` itself.

## Settings window organization ([#23](https://github.com/albrektsson/gw2-session-tracker/issues/23))

The ~8 new config groups above consolidate into 3 new themed tabs rather than a
tab-per-group flat list. Final tab bar, in order:

**General, Select Stats, Arrange Stats, Appearance, Window Behavior, Formatting**

- **General** (shrinks): API key entry, poll status, Reset Session only.
  `background_opacity`/`text_scale` move to Appearance.
- **Appearance** (new) — split into 3 `CollapsingHeader` sections (largest tab; the
  other two new tabs stay flat, no sub-headers):
  - *Text & Color*: `text_scale`, `bold_text`, `label_color`, `value_color`,
    `background_color`, `background_opacity`
  - *Row Format*: the composable field list + separator
  - *Window Sizing*: the 5 sizing knobs
- **Window Behavior** (new, flat): anchor + offset, `window_drag_enabled`,
  `click_through_enabled` (adjacent to drag), `menu_icon_enabled`
- **Formatting** (new, flat): `coin_format`, `hide_zero_stats`

**File structure**: one file per new tab — `appearance_tab.rs`, `window_behavior_tab.rs`,
`formatting_tab.rs` — each exporting `render_*_tab(ui, app)`, matching the existing
`arrange_stats_tab.rs`/`select_stats_tab.rs` convention. `settings_window.rs` stays a
thin wiring point (tab_bar + General inline, as today).

## Open questions (not blocking implementation)

- Whether the hover tooltip should show a per-stat description line. `StatDef` has no
  description field today; would need a copy source (author it, or pull from the GW2
  API's own achievement/item descriptions) before it's specifiable.

## Out of scope

- **Automatic session reset trigger** (e.g. Blish HUD's "on module start"). `CONTEXT.md`'s
  Session definition already explicitly rejects automatic triggers. Manual reset only.
