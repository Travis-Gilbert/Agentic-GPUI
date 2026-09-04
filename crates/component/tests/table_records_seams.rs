//! Seams a record-grid consumer needs from `TableState`, each pinned by the
//! behaviour it changes rather than by the field that carries it.
//!
//! A record grid is read as a grid of fields, not as a list of rows: it counts
//! its rows, it wants a hairline between columns, it offers a sort rather than
//! announcing one on every column, and it puts a menu on the row while still
//! selecting cells. The four tests below are those four promises.

use std::{cell::RefCell, rc::Rc};

use gpui::{
    App, AppContext as _, Context, Div, Entity, InteractiveElement as _, IntoElement, Modifiers,
    MouseButton, MouseDownEvent, Pixels, Point, Render, Stateful, Styled as _, Subscription,
    TestAppContext, VisualTestContext, Window, div, px,
};
use gpui_component::{
    Sizable as _, Size,
    menu::PopupMenu,
    table::{Column, ColumnSort, DataTable, TableDelegate, TableEvent, TableState},
};

#[derive(Default)]
struct SeamLog {
    /// Every row index the table asked the delegate to render, filler rows
    /// included: a filler row is drawn through `render_tr` like any other.
    rendered_rows: Vec<usize>,
    /// Every row index the table asked the delegate to build a menu for.
    menu_rows: Vec<usize>,
    /// Every cell a right-click reported.
    right_clicked_cells: Vec<(usize, usize)>,
}

struct SeamDelegate {
    rows: usize,
    sort: Option<ColumnSort>,
    log: Rc<RefCell<SeamLog>>,
}

impl TableDelegate for SeamDelegate {
    fn columns_count(&self, _: &App) -> usize {
        2
    }

    fn rows_count(&self, _: &App) -> usize {
        self.rows
    }

    fn column(&self, col_ix: usize, _: &App) -> Column {
        let name = format!("column-{col_ix}");
        let column = Column::new(name.clone(), name).width(px(160.));
        match self.sort {
            Some(sort) => column.sort(sort),
            None => column,
        }
    }

    fn render_th(
        &mut self,
        col_ix: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        div()
            .size_full()
            .debug_selector(move || format!("th-{col_ix}"))
    }

    fn render_tr(
        &mut self,
        row_ix: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> Stateful<Div> {
        self.log.borrow_mut().rendered_rows.push(row_ix);
        div().id(("row", row_ix))
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        div()
            .size_full()
            .debug_selector(move || format!("td-{row_ix}-{col_ix}"))
    }

    fn context_menu(
        &mut self,
        row_ix: usize,
        menu: PopupMenu,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> PopupMenu {
        self.log.borrow_mut().menu_rows.push(row_ix);
        menu
    }
}

struct SeamRoot {
    table: Entity<TableState<SeamDelegate>>,
    stripe: bool,
    _events: Subscription,
}

impl Render for SeamRoot {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        DataTable::new(&self.table)
            .stripe(self.stripe)
            .with_size(Size::Medium)
    }
}

struct Seam {
    log: Rc<RefCell<SeamLog>>,
}

/// Open a window on one table and draw it twice.
///
/// The second draw is not a formality: the height the table pads against is
/// read from the scroll handle's bounds, which the first layout is what
/// produces.
fn open(
    cx: &mut TestAppContext,
    rows: usize,
    sort: Option<ColumnSort>,
    stripe: bool,
    build: impl FnOnce(TableState<SeamDelegate>) -> TableState<SeamDelegate> + 'static,
) -> (Seam, &mut VisualTestContext) {
    cx.update(gpui_component::init);

    let log = Rc::new(RefCell::new(SeamLog::default()));
    let delegate_log = log.clone();
    let event_log = log.clone();
    let (_root, cx) = cx.add_window_view(move |window, cx| {
        let table = cx.new(|cx| {
            build(TableState::new(
                SeamDelegate {
                    rows,
                    sort,
                    log: delegate_log,
                },
                window,
                cx,
            ))
        });
        let events = cx.subscribe(&table, move |_: &mut SeamRoot, _, event: &TableEvent, _| {
            if let TableEvent::RightClickedCell(row_ix, col_ix) = event {
                event_log
                    .borrow_mut()
                    .right_clicked_cells
                    .push((*row_ix, *col_ix));
            }
        });
        SeamRoot {
            table,
            stripe,
            _events: events,
        }
    });

    cx.run_until_parked();
    draw(cx);
    draw(cx);
    (Seam { log }, cx)
}

fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| {
        _ = window.draw(cx);
    });
}

fn center(cx: &mut VisualTestContext, selector: &'static str) -> Point<Pixels> {
    let bounds = cx
        .debug_bounds(selector)
        .unwrap_or_else(|| panic!("{selector} was not drawn"));
    bounds.center()
}

#[gpui::test]
fn a_striped_table_still_pads_below_its_last_row_by_default(cx: &mut TestAppContext) {
    let (seam, _) = open(cx, 2, None, true, |table| table);

    let last = seam.log.borrow().rendered_rows.iter().copied().max();
    assert!(
        last.is_some_and(|last| last > 1),
        "the seam is opt-in: upstream's padding is what a table draws until it says otherwise, \
         and this one drew up to {last:?}"
    );
}

#[gpui::test]
fn fill_empty_rows_off_draws_exactly_the_rows_that_carry_data(cx: &mut TestAppContext) {
    let (seam, _) = open(cx, 2, None, true, |table| table.fill_empty_rows(false));

    let mut rendered = seam.log.borrow().rendered_rows.clone();
    rendered.sort_unstable();
    rendered.dedup();
    assert_eq!(
        rendered,
        vec![0, 1],
        "a striped table told not to pad draws its two rows and no others"
    );
}

#[gpui::test]
fn an_unsorted_column_offers_its_chevrons_only_under_the_pointer(cx: &mut TestAppContext) {
    let (_seam, cx) = open(cx, 2, Some(ColumnSort::Default), false, |table| table);

    assert!(
        cx.debug_bounds("table-sort-icon:0").is_none(),
        "an unsorted column's chevrons stay out of the row of column names at rest"
    );

    let header = center(cx, "th-0");
    cx.simulate_mouse_move(header, None, Modifiers::default());
    draw(cx);

    assert!(
        cx.debug_bounds("table-sort-icon:0").is_some(),
        "the offer arrives with the pointer on the header"
    );
}

#[gpui::test]
fn a_sorted_column_keeps_its_arrow_with_the_pointer_away(cx: &mut TestAppContext) {
    let (_seam, cx) = open(cx, 2, Some(ColumnSort::Ascending), false, |table| table);

    assert!(
        cx.debug_bounds("table-sort-icon:0").is_some(),
        "a sorted column reports the table's state whether or not it is pointed at"
    );
}

/// The drawn width of the first cell's content, with dividers on or off.
///
/// The window is opened and read inside this function so the visual context it
/// borrows is released before the next one is opened.
fn first_cell_width(cx: &mut TestAppContext, column_dividers: bool) -> Pixels {
    let (_seam, cx) = open(cx, 2, None, false, move |table| {
        table.column_dividers(column_dividers)
    });
    cx.debug_bounds("td-0-0")
        .expect("the first cell was not drawn")
        .size
        .width
}

#[gpui::test]
fn column_dividers_take_a_pixel_from_every_cell(cx: &mut TestAppContext) {
    let undivided = first_cell_width(cx, false);
    let divided = first_cell_width(cx, true);

    assert_eq!(
        divided,
        undivided - px(1.),
        "the divider is a hairline inside the column's own width, not width added to it"
    );
}

#[gpui::test]
fn a_right_click_in_a_selectable_cell_still_opens_the_rows_menu(cx: &mut TestAppContext) {
    let (seam, cx) = open(cx, 3, None, false, |table| {
        table.cell_selectable(true).row_header(false)
    });

    let cell = center(cx, "td-1-0");
    cx.simulate_event(MouseDownEvent {
        button: MouseButton::Right,
        position: cell,
        modifiers: Modifiers::default(),
        click_count: 1,
        first_mouse: false,
    });
    draw(cx);

    let log = seam.log.borrow();
    assert_eq!(
        log.right_clicked_cells,
        vec![(1, 0)],
        "the cell still reports itself"
    );
    assert_eq!(
        log.menu_rows,
        vec![1],
        "and the click reaches the row, so the row's menu is the one that opens"
    );
}
