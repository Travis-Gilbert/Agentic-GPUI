---
title: Measured layout
description: Place child allocations computed by an external layout owner.
---

# Measured layout

`MeasuredLayout` places explicit parent-local logical-pixel allocations in
back-to-front order. It clips paint and pointer participation at the container
and child boundaries. A zero-area allocation creates no paint or pointer target.
Passive allocations do not claim pointer ownership. Interactive children must
block covered pointer targets at their actual control root; use
`block_mouse_except_scroll()` when ancestor wheel routing must remain available.
This does not distinguish covered sibling scroll owners from ancestors. It owns no resize state and never computes preferred sizes or remote layout.

```rust
use gpui_kit::*;
use gpui_kit::component::{button::Button, measured_layout::MeasuredLayout};

MeasuredLayout::new("owner-layout", size(px(200.), px(100.)))
    .child("existing-command", Bounds::new(point(px(40.), px(10.)), size(px(120.), px(80.))),
        Button::new("command").label("Existing command").w(px(120.)).h(px(80.)))
```

The actual child control must consume its allocation too. Sizing a placement
wrapper does not resize a child's intrinsic height or hit region. Use stable
owner identities for both the allocation and control, and bind measured data to
the owning frame/viewport revision. Painting must not call the remote evaluator.
External geometry grants no custom paint, popup, focus or splitter behavior.

`Button::on_bounds` and `Checkbox::on_bounds` observe the resolved native control
root, including border and padding, without joining label content. Multiple
observers compose. The Base `BoundsObserver` delegates an element's actual layout
ID and render phases; it does not add another allocation or hit region. Consumers
may use that observation for semantic diagnostics without moving activation to a
separate wrapper.
