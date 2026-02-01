# tjam visualizer widget — super simple integration

This is a **vibecoded adaptation** of the original project:  
https://github.com/alemidev/scope-tui

Goal: same scope vibe (oscillo / vectorscope / spectroscope) but as a **ratatui widget** that I can place anywhere in my TUI.

---

## What it is

- `VisualizerWidget` (stateful widget)
- `VisualizerState` (stores mode + config + cached datasets)
- **TAB** switches modes:
    - oscilloscope → vectorscope → spectroscope → oscilloscope

The widget needs audio frames as:

```rust
Option<&Matrix<f64>>
```

---

## Files / module

Keep UI stuff in a folder module (avoid `src/ui.rs`):

```
src/ui/mod.rs
src/ui/visualizer_widget/...
```

`src/ui/mod.rs`:

```rust
pub mod visualizer_widget;
```

---

## 1) Add state to your App

```rust
use crate::ui::visualizer_widget::VisualizerState;

struct App {
    viz: VisualizerState,
}

impl App {
    fn new() -> Self {
        Self { viz: VisualizerState::new() }
    }
}
```

---

## 2) Render it where you want

Inside `terminal.draw`:

```rust
use crate::ui::visualizer_widget::VisualizerWidget;

terminal.draw(|f| {
    let area = f.area();

    // you decide the area (layout / split / centered / whatever)
    let viz_area = area;

    let widget = VisualizerWidget::new(audio_frame.as_ref());
    f.render_stateful_widget(widget, viz_area, &mut app.viz);
})?;
```

---

## 3) Forward events to the widget state

In your event loop:

```rust
while event::poll(Duration::from_millis(0))? {
    let ev = event::read()?;

    // optional: ignore key releases
    if let Event::Key(k) = &ev {
        if k.kind != KeyEventKind::Press {
            continue;
        }
    }

    // widget handles TAB + per-mode keys
    if app.viz.handle_event(ev) {
        break; // widget requested quit
    }
}
```

---

## Feeding audio frames

The widget expects:

```rust
pub type Matrix<T> = Vec<Vec<T>>;
```

Example stereo:

- `data[0]` = Left
- `data[1]` = Right

You can feed it from:

- rodio tap capture
- test buffers
- anything that produces `Matrix<f64>`

---

## Common gotchas

- If you get Rust module ambiguity (E0761): don’t have both `src/ui.rs` and `src/ui/mod.rs`. Keep the folder module (`src/ui/mod.rs`).
- If you hit borrow checker issues in `VisualizerState::update()`: don’t call `current_display()` then `current_display_mut()` in the same method. Just `match self.mode` and call the concrete fields.

---

## Done

That’s it.  
Widget in, feed frames, forward events, TAB switches modes.
