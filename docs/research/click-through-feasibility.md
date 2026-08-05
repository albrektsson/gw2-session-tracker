# Research: is mouse click-through achievable for the main window?

Part of #7. Answers #19.

## Bottom line

**Yes, feasible — at the ImGui level, cheaply.** Build the main window with
Dear ImGui's `ImGuiWindowFlags_NoMouseInputs` (exposed in the vendored Rust
binding as `Window::mouse_inputs(false)` or `.no_inputs()`). Nexus's own
input-routing code gates game-vs-UI mouse dispatch on ImGui's aggregate
`io.WantCaptureMouse`, and a window with that flag stops counting toward
it — so clicks over the window fall through to the game exactly the way a
Blish HUD "clickthrough" toggle behaves. No Nexus-specific API and no raw
Win32 window-style hack (`WS_EX_TRANSPARENT`/`WS_EX_LAYERED`) are needed or
even usable here — see caveats below for why the Win32 route is a dead end
for this addon.

Caveat: this is an all-or-nothing, whole-window toggle. `NoMouseInputs`
disables hover/click capture for every widget in the window for that
frame — there's no partial-widget or edge-only passthrough. That's fine for
a "clickthrough" checkbox (the Blish parity ask), but it means while
click-through is on, none of the window's own buttons/drag-resize/etc. are
interactive either — matches how the feature reads in Blish HUD (a mode you
toggle off to interact with the window again).

## 1. Does Raidcore Nexus itself support click-through for addon windows?

No dedicated addon-facing API for it, and no Nexus-side window-style
tricks — because there is no addon-specific window to apply them to (see
§3). What Nexus *does* provide is the mechanism that makes the ImGui flag
effective:

- Nexus subclasses **GW2's own window**, not a Nexus- or addon-owned one.
  `src/Hooks/Hooks.cpp`: `s_Context.WindowHandle = swapChainDesc.OutputWindow;`
  — the HWND comes from the game's own DXGI swap chain description, captured
  by hooking `Present`.
  (Source: `RaidcoreGG/Nexus`, `src/Hooks/Hooks.cpp`, via `gh search code`
  against repo HEAD `e14ebecfc645e0c9185cd070e18f30d6ff52c50a`.)
- `src/Runtime/Runtime.cpp`: `SetWindowLongPtr(ctx.WindowHandle, GWLP_WNDPROC,
  (LONG_PTR)Hooks::Target::WndProc);` — Nexus subclasses that single HWND's
  WndProc globally, once, for the whole process.
- `src/UI/UiContext.cpp:274`: `ImGui_ImplWin32_Init(Runtime::Get().WindowHandle);`
  — ImGui itself is initialized directly against that same shared HWND.
- `src/UI/UiInput.cpp` (`CUiInput::WndProc`, lines 40–152) is where the
  actual per-message routing decision happens:
  ```cpp
  case WM_LBUTTONDOWN:
  {
      if (io.WantCaptureMouse && !Inputs::IsCursorHidden())
      {
          io.MouseDown[0] = true;
          return 0;   // consumed by ImGui, not forwarded to the game
      }
      else //if (!io.WantCaptureMouse)
      {
          ImGui::ClearActiveID();
      }
      break;   // NOT consumed -> falls through to the game's own WndProc
  }
  ```
  (`src/UI/UiInput.cpp:73-95`, same repo/ref.) When `io.WantCaptureMouse` is
  false, Nexus does not consume the click — it `break`s out and the message
  continues down the window's original WndProc chain to the game. This is
  the exact mechanism that makes an ImGui-level "don't capture mouse" flag
  equivalent to click-through at the OS input level, with zero Nexus-side
  involvement needed.
- The public addon-facing C API (`src/Host/Addons/API/ApiV6.h`, matching
  the `nexus-rs` `v6.rs` bindings this addon is pinned to) has no
  mouse/input/hittest/passthrough entries — confirmed by grepping the header
  content directly (only hit: the unrelated `InputBinds` vtable for
  keybind registration).
- Searched Nexus's issues and PRs for `"click through"`, `"clickthrough"`,
  `"mouse passthrough"` — zero results. No maintainer-side feature request
  or discussion exists for this.
- The one `Passthrough` concept that *does* exist in Nexus
  (`src/Inputs/InputBinds/IbApi.h`, `IbMapping.h`) is unrelated: it's a
  per-keybind flag controlling whether a **keyboard** bind also forwards to
  the game after an addon handles it. Not mouse, not windows.

## 2. Does Dear ImGui expose a window flag for mouse pass-through?

Yes — this is the actual mechanism in play, and it's a stock Dear ImGui
feature, not something Nexus or `nexus-rs` had to add.

- **ImGui version vendored via `nexus-rs`**: 1.80 (via the `arcdps-imgui-sys`
  0.8.0 crate, which is `nexus-rs`'s ImGui dependency — see
  `Cargo.lock`, `[[package]] name = "arcdps-imgui"` pulled in by `nexus`
  0.12.0 at commit `51bff5ab38ad11332316448b452ece1b773018cd`, the revision
  this repo's `Cargo.lock` pins).
  `~/.cargo/registry/src/index.crates.io-*/arcdps-imgui-sys-0.8.0/third-party/imgui/imgui.h:61`:
  `#define IMGUI_VERSION "1.80"`.
- The flag, straight from upstream Dear ImGui's own header (the primary
  source of truth for what it does):
  `.../arcdps-imgui-sys-0.8.0/third-party/imgui/imgui.h:881`:
  ```c
  ImGuiWindowFlags_NoMouseInputs = 1 << 9, // Disable catching mouse, hovering test with pass through.
  ```
  and the composite `ImGuiWindowFlags_NoInputs` (line 894) which additionally
  drops nav/keyboard focus.
- **The Rust binding exposes it directly**, no `unsafe`/raw `sys::` call
  needed (unlike `igSetWindowFontScale`, which this addon already calls raw
  in `crates/addon/src/ui/main_window.rs:64-66`):
  `~/.cargo/registry/src/index.crates.io-*/arcdps-imgui-0.8.0/src/window/mod.rs`:
  - Line 72: `const NO_MOUSE_INPUTS = sys::ImGuiWindowFlags_NoMouseInputs;`
  - Lines 351-356: builder method
    ```rust
    /// Enables/disables catching mouse input.
    /// Enabled by default.
    /// Note: Hovering test will pass through when disabled
    pub fn mouse_inputs(mut self, value: bool) -> Self {
        self.flags.set(WindowFlags::NO_MOUSE_INPUTS, !value);
        self
    }
    ```
  - Lines 472-481: `.no_inputs()` shorthand (mouse + nav + nav-focus).
- **How it connects to actual OS-level pass-through**: per upstream ImGui's
  own header comment (`imgui.h:1812`), `io.WantCaptureMouse` is "Set when
  Dear ImGui will use mouse inputs, in this case do not dispatch them to
  your main game/application" — and a window built with `NoMouseInputs`
  is excluded from the hover test that would otherwise set it (`imgui.h:1192`:
  "windows with the `ImGuiWindowFlags_NoInputs` flag are ignored by
  `IsWindowHovered()` calls"). That's the io flag Nexus's `UiInput.cpp`
  reads (§1) to decide whether to forward the click to the game. So: set
  the flag on the window → ImGui stops reporting it hovered → Nexus's
  `WantCaptureMouse` check fails → the click physically reaches GW2's own
  WndProc, i.e. true click-through, not a paint trick.

**Where to wire it in this addon**: `crates/addon/src/ui/main_window.rs:56-60`
currently builds the window as:
```rust
nexus::imgui::Window::new("Session Tracker")
    .bg_alpha(state.background_opacity)
    .no_decoration()
    .always_auto_resize(true)
    .build(ui, || { ... })
```
A click-through toggle would add `.mouse_inputs(!state.click_through)` (or
equivalent) to that chain, driven by a new `AppState` field/setting — no
`unsafe` or raw `sys::` call required, unlike the font-scale precedent.

## 3. Would raw Win32 window-style manipulation be viable instead?

**No — and it's moot, because there is no addon-level HWND to apply it to.**
This was the open question in the original ticket, and it's answered
definitively by the same Nexus source cited in §1:

- GW2 owns exactly one HWND for its entire render surface. Nexus obtains it
  from the game's own swap chain (`Hooks.cpp`, `swapChainDesc.OutputWindow`)
  rather than creating one.
- Nexus does not create a second HWND for its own UI or for addons. The one
  `CreateWindowExA` call anywhere in the Nexus codebase
  (`src/Hooks/Hooks.cpp:90`) is unrelated to the overlay: it's a short-lived
  **dummy window** created solely to stand up a temporary D3D11 device/swap
  chain so Nexus can read the `Present`/`ResizeBuffers` vtable to hook it —
  it's destroyed immediately after (`DestroyWindow(wnd)` at line 171,
  confirmed by reading the surrounding function).
- `nexus-rs`'s own API surface corroborates this from the addon side:
  `nexus/src/api/wnd_proc.rs` lets an addon register a callback into
  *the* WndProc chain (singular — `register_wnd_proc`) and provides
  `send_wnd_proc_to_game(h_wnd, ...)` to forward a message "to the game,
  bypassing other hooks" — both operate on one shared `HWND`, not an
  addon-owned one. There is no `create_window`/`HWND`-returning addon API
  anywhere in `nexus-rs` (`nexus/src/api/*.rs`, `nexus/src/addon.rs`,
  `nexus/src/lib.rs` — grepped, no hits).

Consequently, `SetWindowLongPtr(hwnd, GWLP_EXSTYLE, ... | WS_EX_TRANSPARENT
| WS_EX_LAYERED)` is only reachable against **GW2's single window handle**.
Applying it there would make the *entire game window* click-through
whenever this addon wants its own overlay to be — including the game world,
other addons' windows, and Nexus's own UI. That's a global, all-or-nothing
side effect completely disproportionate to a per-addon-window toggle, and
not something this addon should do even if it were technically wired up
(e.g. via a raw `windows` crate call, since `nexus-rs` re-exports the
`windows` crate types already for `HWND`/`WPARAM`/etc. in `wnd_proc.rs`).
This path is a dead end for the feature as scoped — the ImGui flag in §2 is
strictly better: scoped to the one window, no unsafe Win32 style state to
manage or restore, and it's exactly the mechanism Nexus's own input router
was built to respect.

## Broader sweep for a missed mechanism

Grepped the full vendored `nexus-rs` source tree (`nexus/src/**/*.rs`,
checkout `51bff5a` = `51bff5ab38ad11332316448b452ece1b773018cd`, the
revision pinned in this repo's `Cargo.lock`) for `mouse|click.?through
|passthrough|hittest|WS_EX|input|hover`, and separately the public Nexus C
API header (`src/Host/Addons/API/ApiV6.h`) for
`mouse|input|hittest|passthrough|click`. No addon-facing "ignore mouse
input" mechanism beyond the ImGui window flag turned up in either — the
`InputBindsApi`/`InputBinds` hits are the unrelated keybind-passthrough
feature (§1), and the `quick_access.rs`/`popups.rs` "hover" hits are
tooltip-texture and hover-flag identifiers, not window input routing.

## Sources

- This repo: `Cargo.lock` (`nexus` 0.12.0 pinned to
  `51bff5ab38ad11332316448b452ece1b773018cd`; `arcdps-imgui` 0.8.0),
  `crates/addon/src/ui/main_window.rs`.
- `nexus-rs` (Zerthox), vendored checkout at
  `~/.cargo/git/checkouts/nexus-rs-*/51bff5a/nexus/src/`: `api/wnd_proc.rs`,
  `api/gui.rs`, `api/hook.rs`, `addon.rs`, and a full-tree grep of `src/**`.
- `arcdps-imgui` 0.8.0 / `arcdps-imgui-sys` 0.8.0 (crates.io), vendored at
  `~/.cargo/registry/src/index.crates.io-*/arcdps-imgui-0.8.0/src/window/mod.rs`
  and `arcdps-imgui-sys-0.8.0/third-party/imgui/imgui.h` (upstream Dear
  ImGui 1.80 header, the primary source for what `NoMouseInputs` and
  `WantCaptureMouse` mean).
- `RaidcoreGG/Nexus` (https://github.com/RaidcoreGG/Nexus), read via
  `gh api`/`gh search code` against `main` at
  `e14ebecfc645e0c9185cd070e18f30d6ff52c50a`: `src/Hooks/Hooks.cpp`,
  `src/Runtime/Runtime.cpp`, `src/Runtime/Runtime.h`, `src/UI/UiContext.cpp`,
  `src/UI/UiInput.cpp`, `src/Inputs/InputBinds/IbApi.h`,
  `src/Inputs/InputBinds/IbMapping.h`,
  `src/Host/Addons/API/ApiV6.h`; issue/PR search for click-through terms
  (zero results).
