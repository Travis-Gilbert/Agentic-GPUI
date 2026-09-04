# License ledger

Upstream is [longbridge/gpui-component](https://github.com/longbridge/gpui-component),
Apache-2.0, and every file here stays under `LICENSE-APACHE`. This ledger names
each change this fork carries beyond upstream, so a reader can tell fork work
from upstream work without a diff, and can see where each change came from.

Nothing in this fork is derived from AGPL-licensed sources. Where a change was
prompted by the behaviour of another product, the behaviour was read and the
code written here from scratch; no source was copied, adapted, or translated.

| Change | Where | Test | Origin |
|---|---|---|---|
| `TableState::fill_empty_rows(bool)`, default `true`. Upstream pads the space below the last row of a striped table with empty rows; `false` keeps the banding and leaves that space blank, so a reader counting rows counts only rows that carry data. | `crates/ui/src/table/state.rs` | `crates/ui/tests/table_records_seams.rs::fill_empty_rows_off_draws_exactly_the_rows_that_carry_data`, and `::a_striped_table_still_pads_below_its_last_row_by_default` for the unchanged default | Written for this fork. |
| `TableState::column_dividers(bool)`, default `false`. Draws the `table_row_border` hairline down the right edge of every cell, for a table read as a grid of fields rather than as a list of rows. It sits inside the column's own width, so turning it on moves nothing. | `crates/ui/src/table/state.rs` | `crates/ui/tests/table_records_seams.rs::column_dividers_take_a_pixel_from_every_cell` | Written for this fork. |
| An unsorted column's sort chevrons rest hidden and appear when the pointer is anywhere over that column's header. A sorted column's arrow is unchanged: it reports state, so it is always readable. Carried by a per-header hover group (`col_header_group`) rather than by the icon's own hover, which is the only thing the icon could see before. | `crates/ui/src/table/state.rs` | `crates/ui/tests/table_records_seams.rs::an_unsorted_column_offers_its_chevrons_only_under_the_pointer`, and `::a_sorted_column_keeps_its_arrow_with_the_pointer_away` | Written for this fork. |
| A right-click inside a selectable cell no longer stops propagation. `TableEvent::RightClickedCell` still fires, and the event now reaches the row under the cell and any context menu wrapped around the table, so a table with `cell_selectable(true)` still opens its row menu. The row's mark joins the cell's instead of clearing it. | `crates/ui/src/table/state.rs` | `crates/ui/tests/table_records_seams.rs::a_right_click_in_a_selectable_cell_still_opens_the_rows_menu` | Written for this fork. |

## Considered and not built

- `TableState::row_height(Pixels)`. Not needed: `Size::Size(px)` already sets an
  arbitrary table row height and the default `Size` resolves to 32px
  (`crates/ui/src/sizing.rs`), so a consumer that wants a specific row height
  has one already.

## Upstream oddities seen and left alone

- `render_sort_icon`'s hover branch calls `.opacity(7.)` where the range is
  0–1 (`crates/ui/src/table/state.rs`). Untouched: it predates this fork and
  fixing it is not one of these changes.
