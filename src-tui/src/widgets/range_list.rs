use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Text},
    widgets::{Block, StatefulWidget, Widget},
};

/// A single item in a [`RangeList`].
///
/// Mirrors `ratatui::widgets::ListItem` but with public fields so our custom
/// widget can access them for rendering.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RangeListItem<'a> {
    content: Text<'a>,
    style: Style,
}

impl<'a> RangeListItem<'a> {
    pub fn new<T: Into<Text<'a>>>(content: T) -> Self {
        Self {
            content: content.into(),
            style: Style::default(),
        }
    }

    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn style<S: Into<Style>>(mut self, style: S) -> Self {
        self.style = style.into();
        self
    }

    pub fn height(&self) -> usize {
        self.content.height()
    }

    pub fn width(&self) -> usize {
        self.content.width()
    }
}

impl<'a, T> From<T> for RangeListItem<'a>
where
    T: Into<Text<'a>>,
{
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

/// State for the [`RangeList`] widget.
///
/// Follows the same API as `ratatui::widgets::ListState` with deferred clamping:
/// navigation methods may set `selected` to out-of-bounds values (e.g. `usize::MAX`),
/// which are clamped to the actual item count at render time.
///
/// Extends `ListState` with visual selection support: an anchor-based range selection
/// that can be toggled on/off independently of cursor movement.
#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub struct RangeListState {
    offset: usize,
    selected: Option<usize>,
    /// When `Some`, visual selection is active with this anchor index.
    /// The selection range spans from `min(anchor, selected)` to `max(anchor, selected)`.
    selection_anchor: Option<usize>,
}

impl RangeListState {
    /// Sets the index of the first item to be displayed.
    ///
    /// Fluent setter — consumes and returns `self`.
    #[must_use = "method moves the value of self and returns the modified value"]
    pub const fn with_offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    /// Sets the index of the selected item.
    ///
    /// Fluent setter — consumes and returns `self`.
    #[must_use = "method moves the value of self and returns the modified value"]
    pub const fn with_selected(mut self, selected: Option<usize>) -> Self {
        self.selected = selected;
        self
    }

    /// Index of the first item to be displayed.
    pub const fn offset(&self) -> usize {
        self.offset
    }

    /// Mutable reference to the index of the first item to be displayed.
    pub fn offset_mut(&mut self) -> &mut usize {
        &mut self.offset
    }

    /// Index of the selected item, or `None` if nothing is selected.
    pub const fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Mutable reference to the selected index.
    pub fn selected_mut(&mut self) -> &mut Option<usize> {
        &mut self.selected
    }

    /// Sets the selected item index.
    ///
    /// Setting `None` also resets the scroll offset to `0`.
    pub fn select(&mut self, index: Option<usize>) {
        self.selected = index;
        if index.is_none() {
            self.offset = 0;
        }
    }

    /// Selects the next item, or the first item if nothing is selected.
    ///
    /// Uses deferred clamping: the value is corrected at render time.
    pub fn select_next(&mut self) {
        let next = self.selected.map_or(0, |i| i.saturating_add(1));
        self.select(Some(next));
    }

    /// Selects the previous item, or the last item if nothing is selected.
    ///
    /// Uses deferred clamping: `usize::MAX` is corrected at render time.
    pub fn select_previous(&mut self) {
        let previous = self.selected.map_or(usize::MAX, |i| i.saturating_sub(1));
        self.select(Some(previous));
    }

    /// Selects the first item.
    pub fn select_first(&mut self) {
        self.select(Some(0));
    }

    /// Selects the last item.
    ///
    /// Uses deferred clamping: `usize::MAX` is corrected at render time.
    pub fn select_last(&mut self) {
        self.select(Some(usize::MAX));
    }

    /// Scrolls the selection down by `amount`.
    pub fn scroll_down_by(&mut self, amount: u16) {
        let selected = self.selected.unwrap_or_default();
        self.select(Some(selected.saturating_add(amount as usize)));
    }

    /// Scrolls the selection up by `amount`.
    pub fn scroll_up_by(&mut self, amount: u16) {
        let selected = self.selected.unwrap_or_default();
        self.select(Some(selected.saturating_sub(amount as usize)));
    }

    // --- Visual selection extension ---

    /// Toggles visual selection.
    ///
    /// If selection is inactive, activates it with the anchor set to the current
    /// `selected` index. If selection is already active, deactivates it.
    pub fn toggle_selection(&mut self) {
        if self.selection_anchor.is_some() {
            self.selection_anchor = None;
        } else {
            self.selection_anchor = self.selected;
        }
    }

    /// Cancels visual selection, clearing the anchor.
    pub fn cancel_selection(&mut self) {
        self.selection_anchor = None;
    }

    /// Returns the inclusive selection range `(start, end)` if visual selection is active.
    ///
    /// The range is ordered so `start <= end`.
    /// Returns `None` if visual selection is not active or no item is selected.
    pub fn selection_range(&self) -> Option<(usize, usize)> {
        let anchor = self.selection_anchor?;
        let selected = self.selected?;
        Some((anchor.min(selected), anchor.max(selected)))
    }

    /// Whether visual selection is currently active.
    pub const fn is_selection_active(&self) -> bool {
        self.selection_anchor.is_some()
    }
}

/// A scrollable list widget with managed scroll state and visual selection support.
///
/// Similar to `ratatui::widgets::List` but paired with [`RangeListState`] which
/// provides cursor navigation, scroll offset management, and visual range selection.
///
/// Uses [`RangeListItem`] instead of `ratatui::widgets::ListItem` so item content
/// and style are accessible for custom rendering.
///
/// # Usage
///
/// ```ignore
/// let items: Vec<RangeListItem> = data.iter().map(render_item).collect();
/// let widget = RangeList::new(items)
///     .block(block)
///     .highlight_style(Style::default().bg(Color::DarkGray))
///     .selection_style(Style::default().bg(Color::Rgb(40, 40, 60)));
/// frame.render_stateful_widget(widget, area, &mut state);
/// ```
pub struct RangeList<'a> {
    items: Vec<RangeListItem<'a>>,
    block: Option<Block<'a>>,
    style: Style,
    highlight_style: Style,
    selection_style: Style,
    highlight_symbol: Option<&'a str>,
    scroll_padding: usize,
}

impl<'a> RangeList<'a> {
    /// Creates a new `RangeList` from an iterator of items.
    pub fn new<T>(items: T) -> Self
    where
        T: IntoIterator,
        T::Item: Into<RangeListItem<'a>>,
    {
        Self {
            items: items.into_iter().map(Into::into).collect(),
            block: None,
            style: Style::default(),
            highlight_style: Style::default(),
            selection_style: Style::default(),
            highlight_symbol: None,
            scroll_padding: 0,
        }
    }

    /// Wraps the list with a [`Block`].
    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    /// Sets the base style for the entire widget area.
    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Sets the style applied to the selected (cursor) item.
    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn highlight_style(mut self, style: Style) -> Self {
        self.highlight_style = style;
        self
    }

    /// Sets the style applied to items within the visual selection range.
    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn selection_style(mut self, style: Style) -> Self {
        self.selection_style = style;
        self
    }

    /// Sets the symbol displayed before the selected item.
    #[must_use = "method moves the value of self and returns the modified value"]
    pub fn highlight_symbol(mut self, symbol: &'a str) -> Self {
        self.highlight_symbol = Some(symbol);
        self
    }

    /// Sets the number of items to keep visible above and below the selected item.
    #[must_use = "method moves the value of self and returns the modified value"]
    pub const fn scroll_padding(mut self, padding: usize) -> Self {
        self.scroll_padding = padding;
        self
    }

    /// Returns the number of items in the list.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns `true` if the list is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Calculates the visible item range given the current scroll state.
    ///
    /// Uses the same algorithm as ratatui's `List` — accounts for multi-line items
    /// and `scroll_padding`.
    fn get_items_bounds(
        &self,
        selected: Option<usize>,
        offset: usize,
        max_height: usize,
    ) -> (usize, usize) {
        let offset = offset.min(self.items.len().saturating_sub(1));

        let mut first_visible_index = offset;
        let mut last_visible_index = offset;

        let mut height_from_offset = 0;

        for item in self.items.iter().skip(offset) {
            if height_from_offset + item.height() > max_height {
                break;
            }
            height_from_offset += item.height();
            last_visible_index += 1;
        }

        let index_to_display = self
            .apply_scroll_padding_to_selected_index(
                selected,
                max_height,
                first_visible_index,
                last_visible_index,
            )
            .unwrap_or(offset);

        while index_to_display >= last_visible_index {
            height_from_offset =
                height_from_offset.saturating_add(self.items[last_visible_index].height());
            last_visible_index += 1;

            while height_from_offset > max_height {
                height_from_offset =
                    height_from_offset.saturating_sub(self.items[first_visible_index].height());
                first_visible_index += 1;
            }
        }

        while index_to_display < first_visible_index {
            first_visible_index -= 1;
            height_from_offset =
                height_from_offset.saturating_add(self.items[first_visible_index].height());

            while height_from_offset > max_height {
                last_visible_index -= 1;
                height_from_offset =
                    height_from_offset.saturating_sub(self.items[last_visible_index].height());
            }
        }

        (first_visible_index, last_visible_index)
    }

    fn apply_scroll_padding_to_selected_index(
        &self,
        selected: Option<usize>,
        max_height: usize,
        first_visible_index: usize,
        last_visible_index: usize,
    ) -> Option<usize> {
        let last_valid_index = self.items.len().saturating_sub(1);
        let selected = selected?.min(last_valid_index);

        let mut scroll_padding = self.scroll_padding;
        while scroll_padding > 0 {
            let mut height_around_selected = 0;
            for index in selected.saturating_sub(scroll_padding)
                ..=selected
                    .saturating_add(scroll_padding)
                    .min(last_valid_index)
            {
                height_around_selected += self.items[index].height();
            }
            if height_around_selected <= max_height {
                break;
            }
            scroll_padding -= 1;
        }

        Some(
            if (selected + scroll_padding).min(last_valid_index) >= last_visible_index {
                selected + scroll_padding
            } else if selected.saturating_sub(scroll_padding) < first_visible_index {
                selected.saturating_sub(scroll_padding)
            } else {
                selected
            }
            .min(last_valid_index),
        )
    }
}

impl<'a> StatefulWidget for RangeList<'a> {
    type State = RangeListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        buf.set_style(area, self.style);

        self.block.as_ref().render_ref(area, buf);
        let list_area = self.block.as_ref().inner_if_some(area);

        if list_area.is_empty() {
            return;
        }

        if self.items.is_empty() {
            state.select(None);
            return;
        }

        // Clamp deferred `selected` values to the actual item range.
        if state.selected.is_some_and(|s| s >= self.items.len()) {
            state.select(Some(self.items.len().saturating_sub(1)));
        }

        // Also clamp the selection anchor.
        if let Some(anchor) = state.selection_anchor {
            if anchor >= self.items.len() {
                state.selection_anchor = Some(self.items.len().saturating_sub(1));
            }
        }

        let list_height = list_area.height as usize;

        let (first_visible_index, last_visible_index) =
            self.get_items_bounds(state.selected, state.offset, list_height);

        state.offset = first_visible_index;

        // Resolve highlight symbol widths.
        let highlight_symbol = self.highlight_symbol.unwrap_or("");
        let highlight_symbol_width = Line::from(highlight_symbol).width();
        let blank_symbol = " ".repeat(highlight_symbol_width);

        let selection_spacing = state.selected.is_some() && highlight_symbol_width > 0;
        let selection_range = state.selection_range();

        let mut current_height: u16 = 0;
        for (i, item) in self
            .items
            .iter()
            .enumerate()
            .skip(state.offset)
            .take(last_visible_index - first_visible_index)
        {
            let x = list_area.left();
            let y = list_area.top() + current_height;
            current_height += item.height() as u16;

            let row_area = Rect {
                x,
                y,
                width: list_area.width,
                height: item.height() as u16,
            };

            let item_style = self.style.patch(item.style);
            buf.set_style(row_area, item_style);

            let is_selected = state.selected == Some(i);
            let in_selection = selection_range.is_some_and(|(lo, hi)| i >= lo && i <= hi);

            let highlight_symbol_width = highlight_symbol_width as u16;
            let item_area = if selection_spacing {
                Rect {
                    x: row_area.x + highlight_symbol_width,
                    width: row_area.width.saturating_sub(highlight_symbol_width),
                    ..row_area
                }
            } else {
                row_area
            };

            buf.set_style(item_area, item.content.style);
            for (line, row) in item.content.iter().zip(item_area.rows()) {
                line.render(row, buf);
            }

            // Write highlight symbols for each line of the item.
            if selection_spacing {
                for j in 0..item.content.height() {
                    let symbol = if is_selected && j == 0 {
                        highlight_symbol
                    } else {
                        &blank_symbol
                    };
                    buf.set_stringn(
                        x,
                        y + j as u16,
                        symbol,
                        list_area.width as usize,
                        item_style,
                    );
                }
            }

            // Apply selection style first (wider range), then highlight on top (cursor).
            if in_selection {
                buf.set_style(row_area, self.selection_style);
            }
            if is_selected {
                buf.set_style(row_area, self.highlight_style);
            }
        }
    }
}

/// Extension trait adding `inner_if_some` and `render_ref` for `Option<&Block>`.
trait OptionBlockExt {
    fn inner_if_some(self, area: Rect) -> Rect;
    fn render_ref(self, area: Rect, buf: &mut Buffer);
}

impl OptionBlockExt for Option<&Block<'_>> {
    fn inner_if_some(self, area: Rect) -> Rect {
        match self {
            Some(block) => block.inner(area),
            None => area,
        }
    }

    fn render_ref(self, area: Rect, buf: &mut Buffer) {
        if let Some(block) = self {
            block.clone().render(area, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{
        buffer::Buffer,
        layout::Rect,
        style::{Color, Style, Stylize},
        widgets::StatefulWidget,
    };

    use super::*;

    fn stateful_widget(
        widget: RangeList<'_>,
        state: &mut RangeListState,
        width: u16,
        height: u16,
    ) -> Buffer {
        let mut buffer = Buffer::empty(Rect::new(0, 0, width, height));
        StatefulWidget::render(widget, buffer.area, &mut buffer, state);
        buffer
    }

    // ---- RangeListState navigation tests ----

    #[test]
    fn default_state() {
        let state = RangeListState::default();
        assert_eq!(state.selected(), None);
        assert_eq!(state.offset(), 0);
        assert!(!state.is_selection_active());
    }

    #[test]
    fn select_and_reset() {
        let mut state = RangeListState::default();
        state.select(Some(2));
        assert_eq!(state.selected(), Some(2));
        assert_eq!(state.offset(), 0);

        state.select(None);
        assert_eq!(state.selected(), None);
        assert_eq!(state.offset(), 0);
    }

    #[test]
    fn state_navigation() {
        let mut state = RangeListState::default();
        state.select_first();
        assert_eq!(state.selected(), Some(0));

        state.select_previous();
        assert_eq!(state.selected(), Some(0));

        state.select_next();
        assert_eq!(state.selected(), Some(1));

        state.select_previous();
        assert_eq!(state.selected(), Some(0));

        state.select_last();
        assert_eq!(state.selected(), Some(usize::MAX));

        state.select_next();
        assert_eq!(state.selected(), Some(usize::MAX));

        state.select_previous();
        assert_eq!(state.selected(), Some(usize::MAX - 1));
    }

    #[test]
    fn scroll_by() {
        let mut state = RangeListState::default();
        state.select(Some(2));
        state.scroll_down_by(4);
        assert_eq!(state.selected(), Some(6));

        state.scroll_up_by(3);
        assert_eq!(state.selected(), Some(3));

        state.scroll_up_by(100);
        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn fluent_setters() {
        let state = RangeListState::default()
            .with_offset(5)
            .with_selected(Some(3));
        assert_eq!(state.offset(), 5);
        assert_eq!(state.selected(), Some(3));
    }

    #[test]
    fn mutable_accessors() {
        let mut state = RangeListState::default();
        *state.offset_mut() = 10;
        *state.selected_mut() = Some(5);
        assert_eq!(state.offset(), 10);
        assert_eq!(state.selected(), Some(5));
    }

    // ---- Visual selection tests ----

    #[test]
    fn toggle_selection() {
        let mut state = RangeListState::default();
        state.select(Some(3));

        state.toggle_selection();
        assert!(state.is_selection_active());
        assert_eq!(state.selection_range(), Some((3, 3)));

        // Move cursor, range expands.
        state.select_next(); // now at 4
        assert_eq!(state.selection_range(), Some((3, 4)));

        // Move cursor further.
        state.select_next(); // now at 5
        assert_eq!(state.selection_range(), Some((3, 5)));

        // Toggle off.
        state.toggle_selection();
        assert!(!state.is_selection_active());
        assert_eq!(state.selection_range(), None);
    }

    #[test]
    fn selection_range_backwards() {
        let mut state = RangeListState::default();
        state.select(Some(5));
        state.toggle_selection();
        state.select_previous(); // 4
        state.select_previous(); // 3
        assert_eq!(state.selection_range(), Some((3, 5)));
    }

    #[test]
    fn cancel_selection() {
        let mut state = RangeListState::default();
        state.select(Some(3));
        state.toggle_selection();
        assert!(state.is_selection_active());
        state.cancel_selection();
        assert!(!state.is_selection_active());
    }

    #[test]
    fn selection_range_with_nothing_selected() {
        let mut state = RangeListState::default();
        // Anchor exists but nothing is selected.
        state.selection_anchor = Some(3);
        assert_eq!(state.selection_range(), None);
    }

    // ---- Render tests ----

    #[test]
    fn empty_list() {
        let mut state = RangeListState::default();
        state.select_first();
        let list = RangeList::new(Vec::<RangeListItem>::new());
        let buffer = stateful_widget(list, &mut state, 10, 3);
        assert_eq!(state.selected(), None);
        assert_eq!(
            buffer,
            Buffer::with_lines(["          ", "          ", "          "])
        );
    }

    #[test]
    fn single_item() {
        let mut state = RangeListState::default().with_selected(Some(0));
        let list = RangeList::new(["Item 0"]).highlight_symbol(">> ");
        let buffer = stateful_widget(list, &mut state, 10, 3);
        let expected = Buffer::with_lines([">> Item 0 ", "          ", "          "]);
        assert_eq!(buffer, expected);
        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn clamp_selected_to_last() {
        let mut state = RangeListState::default();
        state.select_last(); // usize::MAX
        let list = RangeList::new(["A", "B", "C"]);
        let _buffer = stateful_widget(list, &mut state, 5, 3);
        assert_eq!(state.selected(), Some(2));
    }

    #[test]
    fn multiple_items_with_highlight() {
        let mut state = RangeListState::default().with_selected(Some(1));
        let list = RangeList::new(["Item 0", "Item 1", "Item 2"])
            .highlight_symbol(">>")
            .highlight_style(Style::default().fg(Color::Yellow));
        let buffer = stateful_widget(list, &mut state, 10, 5);

        // Verify items are rendered in correct positions.
        let expected = Buffer::with_lines([
            "  Item 0  ",
            ">>Item 1  ",
            "  Item 2  ",
            "          ",
            "          ",
        ]);
        // Check text content matches.
        for y in 0..5 {
            for x in 0..10 {
                assert_eq!(
                    buffer[(x, y)].symbol(),
                    expected[(x, y)].symbol(),
                    "symbol mismatch at ({x}, {y})"
                );
            }
        }
        // The selected row (y=1) should have Yellow fg from highlight_style.
        for x in 0..10 {
            assert_eq!(
                buffer[(x, 1)].fg,
                Color::Yellow,
                "expected Yellow fg at ({x}, 1)"
            );
        }
    }

    #[test]
    fn scroll_selected_into_view() {
        let mut state = RangeListState::default()
            .with_selected(Some(4))
            .with_offset(0);
        let list = RangeList::new(["A", "B", "C", "D", "E"]);
        let _buffer = stateful_widget(list, &mut state, 5, 3);
        // E (index 4) should be visible, so offset should shift.
        assert_eq!(state.selected(), Some(4));
        assert_eq!(state.offset(), 2);
    }

    #[test]
    fn selection_style_applied() {
        let mut state = RangeListState::default().with_selected(Some(2));
        state.selection_anchor = Some(0);
        // Selection range: 0..=2

        let list = RangeList::new(["A", "B", "C", "D"])
            .selection_style(Style::default().bg(Color::Blue))
            .highlight_style(Style::default().bg(Color::Red));

        let buffer = stateful_widget(list, &mut state, 5, 4);

        // Items 0, 1 should have selection_style (blue bg).
        // Item 2 should have highlight_style (red bg) since it's selected (overrides).
        // Item 3 should have no special style.
        for y in 0..2 {
            for x in 0..5 {
                let cell = &buffer[(x, y)];
                assert_eq!(
                    cell.bg,
                    Color::Blue,
                    "expected Blue bg at ({x}, {y}), got {:?}",
                    cell.bg
                );
            }
        }
        for x in 0..5 {
            let cell = &buffer[(x, 2)];
            assert_eq!(
                cell.bg,
                Color::Red,
                "expected Red bg at ({x}, 2), got {:?}",
                cell.bg
            );
        }
        for x in 0..5 {
            let cell = &buffer[(x, 3)];
            assert_eq!(
                cell.bg,
                Color::Reset,
                "expected Reset bg at ({x}, 3), got {:?}",
                cell.bg
            );
        }
    }

    #[test]
    fn multi_line_items() {
        let mut state = RangeListState::default().with_selected(Some(1));
        let list = RangeList::new(["Item 0\nLine 2", "Item 1", "Item 2"]);
        let buffer = stateful_widget(list, &mut state, 10, 5);
        let expected = Buffer::with_lines([
            "Item 0    ",
            "Line 2    ",
            "Item 1    ",
            "Item 2    ",
            "          ",
        ]);
        assert_eq!(buffer, expected);
    }

    #[test]
    fn offset_renders_shifted() {
        let mut state = RangeListState::default().with_offset(2);
        let list = RangeList::new(["A", "B", "C", "D", "E"]);
        let buffer = stateful_widget(list, &mut state, 5, 3);
        let expected = Buffer::with_lines(["C    ", "D    ", "E    "]);
        assert_eq!(buffer, expected);
    }

    #[test]
    fn scroll_padding() {
        let mut state = RangeListState::default()
            .with_selected(Some(2))
            .with_offset(2);
        let list = RangeList::new(["Item 0", "Item 1", "Item 2", "Item 3", "Item 4", "Item 5"])
            .scroll_padding(1)
            .highlight_symbol(">> ");
        let buffer = stateful_widget(list, &mut state, 10, 4);
        assert_eq!(state.selected(), Some(2));
        // With padding=1, item 1 should be visible (one before selected).
        assert!(state.offset() <= 1);
        let expected = Buffer::with_lines(["   Item 1 ", ">> Item 2 ", "   Item 3 ", "   Item 4 "]);
        assert_eq!(buffer, expected);
    }

    #[test]
    fn selection_anchor_clamped_at_render() {
        let mut state = RangeListState::default().with_selected(Some(1));
        state.selection_anchor = Some(100); // way out of bounds
        let list = RangeList::new(["A", "B", "C"]);
        let _buffer = stateful_widget(list, &mut state, 5, 3);
        // Anchor should be clamped to 2 (last valid index).
        assert_eq!(state.selection_anchor, Some(2));
    }

    #[test]
    fn with_block() {
        use ratatui::widgets::Borders;

        let mut state = RangeListState::default().with_selected(Some(0));
        let block = Block::default().borders(Borders::ALL).title("List");
        let list = RangeList::new(["Item 0", "Item 1"]).block(block);
        let buffer = stateful_widget(list, &mut state, 12, 4);
        let expected = Buffer::with_lines([
            "┌List──────┐",
            "│Item 0    │",
            "│Item 1    │",
            "└──────────┘",
        ]);
        assert_eq!(buffer, expected);
    }

    #[test]
    fn base_style_applied() {
        let mut state = RangeListState::default();
        let list = RangeList::new(["A", "B"]).style(Style::default().fg(Color::Red));
        let buffer = stateful_widget(list, &mut state, 5, 3);
        let expected = Buffer::with_lines(["A    ".red(), "B    ".red(), "     ".red()]);
        assert_eq!(buffer, expected);
    }
}
