# Session Tracker

A Raidcore Nexus addon for Guild Wars 2 that tracks GW2 stats as both a
Session value and a Lifetime value, mirroring the Blish HUD module
`ecksofa.sessiontracker` (`Taschenbuch/BlishHud-SessionTracker`).

## Language

**Stat**:
A single trackable quantity — an id, a display name, an icon, and a Stat Source. The full set is the Stat Catalog.
_Avoid_: Metric, field

**Stat Source**:
Where a Stat's Lifetime Value comes from: a GW2 achievement id, a wallet currency id, an account/character field (e.g. WvW Rank, KDR), or an item id (summed across storage). Session Timer, Distance Traveled, and Combat Time are sourced from MumbleLink instead of the GW2 API.

**Lifetime Value**:
A Stat's current value as reported by its Stat Source. For achievement- and Deaths-backed stats this only ever increases (see Regression Guard); for currency- and item-backed stats it is a live balance that can also decrease as the player spends, salvages, or deposits.
_Avoid_: Total, all-time value

**Session Value**:
The delta between a Stat's current Lifetime Value and its value when the session started, except for Ratio Stats (e.g. KDR) which are computed from underlying session deltas rather than a lifetime-to-lifetime diff. Can be negative for currency/item stats if spending outpaces gain during the session.
_Avoid_: Delta, gain

**Ratio Stat**:
A Stat whose value is one Stat divided by another (e.g. KDR = kills / deaths) rather than read straight from a Stat Source. Falls back to the raw numerator when the denominator is zero (`ratio_with_fallback`) instead of dividing by zero — applies to both its Lifetime Value and its Session Value.
_Avoid_: Computed stat, derived stat

**Session**:
The tracking window a Session Value is measured against. Ends and restarts on reset, triggered manually only — automatic triggers (e.g. on map change) were considered and rejected as unnecessary.
_Avoid_: Run, tracking period

**Regression Guard**:
The rule that a Lifetime Value drop is ignored (treated as a transient GW2 API glitch) for Stat Sources that should only ever increase — currently Achievement- and Deaths-backed stats (`is_regression_guarded`). Not applied to currency/item stats or ratios, which can legitimately drop.
_Avoid_: Guard, clamp

**Category**:
A Stat's browsing grouping in the Select Stats picker — purely a UI concern, not a property of the Stat Source. A Stat can belong to more than one Category (e.g. a currency is tagged `Currency` and also `Wvw` if WvW-specific).
_Avoid_: Tag, group

**Supercategory**:
A display-only grouping of Categories in the picker (General, Competitive, PvE, Material Storage). Not attached to individual Stats.
_Avoid_: Section, tab group

**Stat List**:
One of four independent, ordered selections of Stats a player has chosen to display — Global, WvW, PvP, or PvE (`StatListKind`). Each has its own selection and order, persisted separately. Assignment to a list is entirely manual; nothing filters which Stats are "allowed" in which list.
_Avoid_: Selection, layout

**Map Group**:
The coarse-grained classification of the player's current map, derived from MumbleLink's map type id (`map_group_for`): Wvw, Pvp, or Pve (open world, fractals, raids, strikes, and other instances all fold into Pve). `None` for character creation/tutorial/unrecognized ids. Determines which Stat List (beyond Global) renders in the main window.
_Avoid_: Mode, map type

**Main Window**:
The always-on HUD showing one row per Stat in the active Stat Lists (Global's Stats first, then the Map Group's list, skipping duplicates).
_Avoid_: HUD, overlay

**Stat Catalog**:
The complete set of Stats the addon knows about, spanning WvW, PvP, general account currencies/achievements, and items (including Material Storage). Breadth target is full parity with BlishHud-SessionTracker.
_Avoid_: Stat list (see Stat List, which is a different concept — a player's selection, not the full catalog)
