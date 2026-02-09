// crates/collector/src/handler_list.rs
//! Handler Card List Widget — Uses workflow_v2 type directly

use iced::widget::{column, container, row, scrollable, text, Space};
use iced::{Alignment, Element, Length, Padding, Theme};

use crate::handler_card::{compact_handler_card, empty_handler_card, handler_card};
use crate::styles;
use crate::workflow::{Handler, Sequence};

/// Handler List Widget — Title Header + Scrollable Card List
pub fn handler_list<'a, Message: 'a + Clone>(
    handlers: &'a [Handler],
    title: &'a str,
) -> Element<'a, Message, Theme> {
    let header = container(
        row![
            text(title).size(18),
            Space::new().width(Length::Fill),
            text(format!("{} handlers", handlers.len())).size(12),
        ]
        .align_y(Alignment::Center)
        .padding(Padding::from([12, 16])),
    )
    .width(Length::Fill)
    .style(styles::header_style);

    let cards: Vec<Element<'a, Message, Theme>> = handlers
        .iter()
        .enumerate()
        .map(|(i, h)| handler_card(i, h))
        .collect();

    let cards_column = column(cards).spacing(12).padding(16);

    let scrollable_content = scrollable(cards_column)
        .height(Length::Fill)
        .width(Length::Fill);

    let content = column![header, scrollable_content,];

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(styles::list_container_style)
        .into()
}

/// Renders a Sequence as a list
pub fn sequence_list<'a, Message: 'a + Clone>(
    sequence: &'a Sequence,
    title: Option<&'a str>,
) -> Element<'a, Message, Theme> {
    let display_title = title.unwrap_or(&sequence.sequence_name);
    handler_list(&sequence.step_sequence, display_title)
}

/// Compact Handler List
pub fn compact_handler_list<'a, Message: 'a + Clone>(
    handlers: &'a [Handler],
    title: &'a str,
) -> Element<'a, Message, Theme> {
    let header = container(
        row![
            text(title).size(14),
            Space::new().width(Length::Fill),
            text(format!("({} items)", handlers.len())).size(10),
        ]
        .align_y(Alignment::Center)
        .padding(Padding::from([8, 12])),
    )
    .width(Length::Fill)
    .style(styles::header_style);

    let cards: Vec<Element<'a, Message, Theme>> = handlers
        .iter()
        .enumerate()
        .map(|(i, h)| compact_handler_card(i, h))
        .collect();

    let cards_column = column(cards).spacing(6).padding(12);

    let scrollable_content = scrollable(cards_column)
        .height(Length::Fill)
        .width(Length::Fill);

    let content = column![header, scrollable_content,];

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(styles::list_container_style)
        .into()
}

/// Empty Handler List (Placeholder)
pub fn empty_handler_list<'a, Message: 'a + Clone>(
    slot_count: usize,
    title: &'a str,
) -> Element<'a, Message, Theme> {
    let header = container(
        row![
            text(title).size(18),
            Space::new().width(Length::Fill),
            text(format!("{} slots", slot_count)).size(12),
        ]
        .align_y(Alignment::Center)
        .padding(Padding::from([12, 16])),
    )
    .width(Length::Fill)
    .style(styles::header_style);

    let cards: Vec<Element<'a, Message, Theme>> = (0..slot_count)
        .map(|i| empty_handler_card(i))
        .collect();

    let cards_column = column(cards).spacing(12).padding(16);

    let scrollable_content = scrollable(cards_column)
        .height(Length::Fill)
        .width(Length::Fill);

    let content = column![header, scrollable_content,];

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(styles::list_container_style)
        .into()
}

/// Mixed Handler List (Data + Empty Slots)
pub fn mixed_handler_list<'a, Message: 'a + Clone>(
    handlers: &'a [Handler],
    min_slots: usize,
    title: &'a str,
) -> Element<'a, Message, Theme> {
    let total_slots = min_slots.max(handlers.len());

    let header = container(
        row![
            text(title).size(18),
            Space::new().width(Length::Fill),
            text(format!("{}/{} handlers", handlers.len(), total_slots)).size(12),
        ]
        .align_y(Alignment::Center)
        .padding(Padding::from([12, 16])),
    )
    .width(Length::Fill)
    .style(styles::header_style);

    let mut cards: Vec<Element<'a, Message, Theme>> = handlers
        .iter()
        .enumerate()
        .map(|(i, h)| handler_card(i, h))
        .collect();

    for i in handlers.len()..total_slots {
        cards.push(empty_handler_card(i));
    }

    let cards_column = column(cards).spacing(12).padding(16);

    let scrollable_content = scrollable(cards_column)
        .height(Length::Fill)
        .width(Length::Fill);

    let content = column![header, scrollable_content,];

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(styles::list_container_style)
        .into()
}