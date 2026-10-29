# Reusable toast notification system for Rust + Slint

This note explains how to port the toast notification system into another Rust +
Slint project without introducing a heavy crate. Toasts stack at the top right,
at most three are active at once, eviction is animated, each toast disappears
automatically after four seconds, and enter/exit transitions animate height,
position and opacity.

## Quick start

1. Copy `ui/toast/toast.slint`, `toast/mod.rs` and `toast/queue.rs` into the
   target project and adapt relative paths.
2. Add `ToastOverlay { available-width: root.width; spacing: ...; }` to the host
   Slint component.
3. Declare `mod toast;`, invoke `slint::include_modules!();` and call
   `Toaster::install(&ui)`.
4. Call `toaster.show("...", ToastLevel::Success)` from UI-thread code.
5. Run `cargo fmt --check`, `cargo check --features gui` and
   `cargo test --features gui`.

## 1. Files to copy

Copy these files into the target project, then adapt relative paths if needed:

- `ui/toast/toast.slint`
- the toast Rust code, placed as a submodule of the GUI module:
  - `toast/mod.rs`
  - `toast/queue.rs`
- `assets/icons/ui/close.svg`, or change the default `close-icon` (see section 6)

The Slint file owns all toast markup. The Rust code must live beside the
`slint::include_modules!()` call, because that macro generates the Slint types
(`MainWindow`, `ToastState`, ...) in the module where it is invoked. The toast
glue cannot live in a different crate than the one that compiles the `.slint`
files.

## 2. Slint integration

The host window imports the overlay, re-exports the globals that Rust needs, and
instantiates the overlay. It declares no toast property or callback of its own.

```slint
import { ToastOverlay } from "toast/toast.slint";

export { ToastTiming, ToastState } from "toast/toast.slint";

export component MainWindow inherits Window {
    // ... ordinary application UI ...

    // Declared after the application content so that toasts are drawn above it.
    ToastOverlay {
        available-width: root.width;
        spacing: Theme.spacing;
    }
}
```

Notes:

- Without the `ToastOverlay { ... }` instance nothing is displayed: importing the
  component does not create it.
- If the target project has no `Theme` global, pass a concrete spacing such as
  `8px`. The overlay has defaults (`available-width: 860px`, `spacing: 8px`),
  but a host should pass its real width.
- Slint generates `ToastItem` because it participates in the public API of
  `ToastState`. The `export { ToastState }` line is enough in the Slint version
  tested here. If `ToastItem` is not generated in another version, add it to the
  `export { ... }` list.
- `mod.rs` refers to the window type as `MainWindow`. In another project, rename
  it there (see section 9).

`toast.slint` provides:

- `ToastItem`: `{ text: string, level: int, id: int, closing: bool }`
- `ToastTiming`: global with `enter: 250ms` and `exit: 200ms`
- `ToastState`: global with `in property <[ToastItem]> items` and
  `callback close-requested(int)`
- `Toast`: the visual component, instantiated by the overlay
- `ToastOverlay`: the positioned stack container

The Rust side owns the model. `ToastOverlay` renders one `Toast` per entry of
`ToastState.items`, in model order, and calls `ToastState.close-requested(id)`
when the close badge of a toast is clicked. The host must not read or write
`ToastState.items`.

## 3. Rust integration

The toast module uses Slint types generated in the GUI module, so it is declared
as a submodule of that module:

```rust
#![cfg(feature = "gui")]

mod toast;
use toast::{ToastLevel, Toaster};

use anyhow::Result;
use slint::ComponentHandle;

slint::include_modules!();

pub fn run() -> Result<()> {
    let ui = MainWindow::new()
        .map_err(|error| anyhow::anyhow!("window creation failed: {error}"))?;

    let toaster = Toaster::install(&ui);

    ui.on_browse_source(move || {
        // ... application work ...
        toaster.show("Source loaded.", ToastLevel::Success);
    });

    ui.run()
        .map_err(|error| anyhow::anyhow!("UI execution failed: {error}"))?;
    Ok(())
}
```

`Toaster` is cheap to clone (an `Rc` and a weak window handle), so clone it into
each closure or pass `&Toaster` to helper functions:

```rust
fn crop_document(toaster: &Toaster) {
    if input_is_missing() {
        toaster.show("Choose a source file first.", ToastLevel::Error);
        return;
    }

    // ... document processing ...

    if has_warnings {
        toaster.show("Completed with warnings.", ToastLevel::Warning);
    } else {
        toaster.show("Completed.", ToastLevel::Success);
    }
}
```

`Toaster::install(ui)`:

- creates the private `Rc<RefCell<ToastQueue>>`
- connects `ToastState::on_close_requested`
- initializes `ToastState.items` with an empty `VecModel`
- returns a `Toaster`

`Toaster::show(text, level)` accepts anything that implements `Into<String>`,
queues the toast, updates the model and schedules the automatic dismissal.
`ToastLevel` maps to the Slint integer used by `ToastItem.level`:
`Success = 0`, `Warning = 1`, `Error = 2`.

Things to know:

- **UI thread only.** `Toaster` contains an `Rc`, so it is not `Send`. Call
  `show` from the UI thread. For work running on another thread, send the result
  back to the UI thread (for example with `slint::invoke_from_event_loop`) and
  show the toast from there. The `Toaster` itself cannot be moved into that
  closure, so keep it in UI-thread state.
- **Window dropped.** `show` does nothing if the window no longer exists
  (the weak handle cannot be upgraded).
- **Close handling.** Because `install` connects the close callback, the host
  needs no close callback and no model management of its own.

## 4. Module layout and build

```text
gui.rs                      // include_modules!(), mod toast;, application handlers
gui/toast/mod.rs            // Toaster, ToastLevel, Slint glue, timers, model sync
gui/toast/queue.rs          // pure queue logic and its unit tests
ui/toast/toast.slint        // all toast Slint markup
ui/main_window.slint        // imports ToastOverlay, re-exports the globals
```

`queue.rs` has no Slint dependency. It contains:

- `ToastNotice`: input of `push_toast` (`text`, `level`, `closing`). Callers
  create it with `closing: false`.
- `ToastEntry`: the stored entry (`id`, `text`, `level`, `closing`)
- `ToastQueue`: the stack (oldest first), ID generation, eviction and two-phase
  removal
- the constants `MAX_VISIBLE_TOASTS` (3) and `TOAST_LIFETIME` (4 s)
- unit tests

The feature and build setup below is illustrative. Compare it with the real
`Cargo.toml` and `build.rs` of the target project instead of copying it blindly.

```toml
[features]
gui = ["dep:slint", "dep:slint-build"]

[dependencies]
slint = { version = "...", optional = true }

[build-dependencies]
slint-build = { version = "...", optional = true }
```

```rust
// build.rs
#[cfg(feature = "gui")]
{
    if let Err(error) = slint_build::compile("ui/main_window.slint") {
        panic!("Slint compilation failed: {error}");
    }
}
```

Imports inside `.slint` files are resolved relative to the importing file, so
the build entry point does not change when the toast file lives in a subfolder.

## 5. Behavior contract

Preserve these invariants when adapting or extending the system:

- The queue stores toasts oldest first. `sync` builds the Slint model in
  **reverse order (newest first)** and inserts new rows at index 0, so
  `ToastOverlay` iterates `ToastState.items` as is. Never reverse indexes in the
  `.slint` file: a delegate must stay attached to the same toast.
- Toasts are identified by ID, never by row or position.
- At most three **active** toasts exist at any time. When a fourth one is pushed,
  the oldest active toast is marked `closing`. It stays in the model during its
  exit animation, so the model can briefly hold four rows.
- Closing is always two-phase: first `closing = true` (the Slint exit animation
  runs), then the entry is removed from the queue and the model after
  `ToastTiming.exit`. Manual dismissal, auto-dismissal and eviction all use this
  path.
- The removal delay is read from the Slint global `ToastTiming.exit`. Keep the
  exit duration defined there and nowhere else, so the Rust timer and the Slint
  animation cannot drift apart.
- Auto-dismiss starts when a toast is shown and fires after four seconds. It does
  nothing if the toast is already closing or removed.
- Model updates are incremental: rows that disappeared are removed, new rows are
  inserted, and changed rows are updated in place with `set_row_data`. The
  existing `VecModel` is never replaced, so delegates keep their identity and do
  not replay their enter animation. The model must be the `VecModel` created by
  `install`; if something else replaces `ToastState.items`, `sync` falls back to
  installing a fresh model and every visible toast replays its enter animation.

Toast colors are semantic and intentionally vivid rather than theme-derived:
success and error use white text, warning uses dark text.

## 6. Icon

`Toast` exposes `in property <image> close-icon`. Its default value is
`@image-url("../../../../assets/icons/ui/close.svg")`, resolved relative to
`toast.slint`. When `toast.slint` is placed in `ui/toast/`, this relative path
resolves to `<project-root>/assets/icons/ui/close.svg`.

- If the file moves, adjust the default path. A missing image is a compile error.
- `ToastOverlay` does not forward `close-icon` to the toasts it creates, so a host
  cannot override the icon from outside. To change it, edit the default in
  `toast.slint`, or add an `in property <image> close-icon` to `ToastOverlay` and
  pass it to each `Toast`.

The badge is a 24x24 px circle containing a 14x14 px icon.

## 7. Tuning

| What | Where |
| --- | --- |
| Maximum active toasts (3) | `MAX_VISIBLE_TOASTS` in `queue.rs` |
| Toast lifetime (4 s) | `TOAST_LIFETIME` in `queue.rs` |
| Enter/exit animation duration | `ToastTiming` in `toast.slint` |
| Colors per level | `Toast` in `toast.slint` |
| Width (max 360 px), position (top right), spacing | `ToastOverlay` in `toast.slint` and the properties passed by the host |

Both toast-count and lifetime values are module-private in `queue.rs`: `MAX_VISIBLE_TOASTS`
is a private `const`, and `TOAST_LIFETIME` is `pub(crate) const`. Edit them directly in
`queue.rs` after copying it. `TOAST_LIFETIME` is also read by `mod.rs` to schedule
automatic dismissal.

## 8. Verification

After adapting the component, run the host's normal checks, for example:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check -p host-crate
cargo check -p host-crate --features gui
```

The unit tests cover only the queue. The animations need a manual check:

1. Trigger four or more toasts quickly: the oldest one must collapse and fade,
   and the others must not flicker.
2. Close one toast with its badge: the others must not replay their enter
   animation.
3. Let a toast expire (four seconds).
4. Shrink the window to its minimum width and check the overlay width.

## 9. Limitations

This is not a standalone crate. Porting is intentionally manual: copy, adapt
paths and names, then compile.

The Rust API is concrete to the host window type. `mod.rs` imports
`super::{MainWindow, ToastItem, ToastState, ToastTiming}`, so a host whose window
has another name must edit that import and the `MainWindow` references.
Genericity over `ComponentHandle` is possible only if the host keeps the same
generated globals and property names.

## Sources in Trimbox

- `crates/cli/ui/toast/toast.slint`
- `crates/cli/src/gui/toast/mod.rs`
- `crates/cli/src/gui/toast/queue.rs`
- `crates/cli/src/gui.rs`
- `crates/cli/ui/main_window.slint`
