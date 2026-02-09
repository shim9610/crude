// crates/collector/src/handler_card.rs
//! Handler Card Widget — Uses workflow_v2 type directly
//!
//! Layout specification identical to existing handler_card.rs:
//!   ┌─────────────────────────────────────────┐
//!   │ [idx] [🏷 Type Badge]  description      │
//!   │  Selector: ...                           │
//!   │  key: value                              │
//!   │  key: value                              │
//!   └─────────────────────────────────────────┘

use iced::widget::{column, container, row, text, Space};
use iced::{Alignment, Element, Length, Padding, Theme};

use crate::styles;
use crate::workflow::{Action, BindValue, Extraction, Handler, Step};

// =========================================================================
// UI Metadata — Implemented directly on workflow type
// =========================================================================

/// Display info by Handler kind
struct HandlerMeta {
    icon: &'static str,
    name: &'static str,
    color: (f32, f32, f32),
}

fn handler_meta(handler: &Handler) -> HandlerMeta {
    match handler {
        Handler::Item(Step::Act(_)) => HandlerMeta {
            icon: "⚡",
            name: "Action",
            color: (0.4, 0.7, 1.0),
        },
        Handler::Item(Step::Extract(_)) => HandlerMeta {
            icon: "📊",
            name: "Extract",
            color: (0.7, 0.5, 0.9),
        },
        Handler::Container { .. } => HandlerMeta {
            icon: "📦",
            name: "Container",
            color: (0.5, 0.8, 0.5),
        },
        Handler::SubSequence(_) => HandlerMeta {
            icon: "🔄",
            name: "SubSequence",
            color: (0.9, 0.6, 0.7),
        },
    }
}

/// One-line summary per Action variant
fn action_summary(action: &Action) -> (&'static str, Option<String>, Vec<(&'static str, String)>) {
    // returns (variant_name, selector_display, detail_pairs)
    match action {
        Action::Click { selector_string } => (
            "Click",
            Some(bind_display(selector_string)),
            vec![],
        ),
        Action::Navigate { url } => (
            "Navigate",
            None,
            vec![("URL", bind_display(url))],
        ),
        Action::WaitFor { selector_string, time_ms } => (
            "WaitFor",
            Some(bind_display(selector_string)),
            vec![("Timeout", bind_display_u64(time_ms))],
        ),
        Action::ScrollAll => ("ScrollAll", None, vec![]),
        Action::ClickByText { selector_string, text } => (
            "ClickByText",
            Some(bind_display(selector_string)),
            vec![("Text", bind_display(text))],
        ),
        Action::GetHTML { selector_string, time_ms } => (
            "GetHTML",
            Some(bind_display(selector_string)),
            vec![("Retry ms", bind_display_u64(time_ms))],
        ),
        Action::DismissPermission => ("DismissPermission", None, vec![]),
        Action::SwitchToDefaultContent => ("SwitchToDefault", None, vec![]),
        Action::SwitchToFrame { selector_string } => (
            "SwitchToFrame",
            Some(bind_display(selector_string)),
            vec![],
        ),
        Action::Refresh => ("Refresh", None, vec![]),
        Action::Forward => ("Forward", None, vec![]),
        Action::Backward => ("Backward", None, vec![]),
        Action::NewTab { url } => (
            "NewTab",
            None,
            vec![("URL", bind_display(url))],
        ),
        Action::SwitchTab { index } => (
            "SwitchTab",
            None,
            vec![("Index", bind_display_u64(index))],
        ),
        Action::CloseTab => ("CloseTab", None, vec![]),
        Action::SwitchToLastTab => ("SwitchToLastTab", None, vec![]),
        Action::Type { selector, text } => (
            "Type",
            Some(bind_display(selector)),
            vec![("Text", bind_display(text))],
        ),
        Action::PressKey { key } => (
            "PressKey",
            None,
            vec![("Key", format!("{:?}", key))],
        ),
        Action::ClearAndType { selector, text } => (
            "ClearAndType",
            Some(bind_display(selector)),
            vec![("Text", bind_display(text))],
        ),
        Action::NavigateHref { base, href } => (
            "NavigateHref",
            None,
            vec![("Base", bind_display(base)), ("Href", bind_display(href))],
        ),
        Action::ScrollDown { scroll } => (
            "ScrollDown",
            None,
            vec![("Pixels", bind_display_i64(scroll))],
        ),
        Action::ScrollUp { scroll } => (
            "ScrollUp",
            None,
            vec![("Pixels", bind_display_i64(scroll))],
        ),
        Action::Wait { time_ms } => (
            "Wait",
            None,
            vec![("Duration", bind_display_u64(time_ms))],
        ),
    }
}

/// Summary per Extraction variant
fn extraction_summary(ext: &Extraction) -> (&'static str, Option<String>, Vec<(&'static str, String)>) {
    match ext {
        Extraction::Text { selector_string, field_name } => (
            "Text",
            Some(bind_display(selector_string)),
            vec![("Field", bind_display(field_name))],
        ),
        Extraction::Count { selector_string, field_name } => (
            "Count",
            Some(bind_display(selector_string)),
            vec![("Field", bind_display(field_name))],
        ),
        Extraction::Attribute { selector_string, field_name, attr_str } => (
            "Attribute",
            Some(bind_display(selector_string)),
            vec![
                ("Field", bind_display(field_name)),
                ("Attr", bind_display(attr_str)),
            ],
        ),
        Extraction::Exists { selector_string, field_name } => (
            "Exists",
            Some(bind_display(selector_string)),
            vec![("Field", bind_display(field_name))],
        ),
        Extraction::MultipleText { selector_string, field_name } => (
            "MultipleText",
            Some(bind_display(selector_string)),
            vec![("Field", bind_display(field_name))],
        ),
    }
}

// =========================================================================
// BindValue Display Helper
// =========================================================================

fn bind_display(bv: &BindValue<String>) -> String {
    match &bv.binding {
        Some(key) => format!("${{{}}}", key),
        None => bv.value.clone(),
    }
}

fn bind_display_u64(bv: &BindValue<u64>) -> String {
    match &bv.binding {
        Some(key) => format!("${{{}}}", key),
        None => bv.value.to_string(),
    }
}
fn bind_display_i64(bv: &BindValue<i64>) -> String {
    if let Some(key) = &bv.binding {
        format!("{{{{{}}}}}", key)
    } else {
        bv.value.to_string()
    }
}
// =========================================================================
// Extract description / selector / details per Handler
// =========================================================================

struct CardInfo {
    description: String,
    selector: Option<String>,
    details: Vec<(String, String)>,
}

fn extract_card_info(handler: &Handler) -> CardInfo {
    match handler {
        Handler::Item(Step::Act(action)) => {
            let (variant_name, selector, details) = action_summary(action);
            CardInfo {
                description: variant_name.to_string(),
                selector,
                details: details
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v))
                    .collect(),
            }
        }
        Handler::Item(Step::Extract(ext)) => {
            let (variant_name, selector, details) = extraction_summary(ext);
            CardInfo {
                description: variant_name.to_string(),
                selector,
                details: details
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v))
                    .collect(),
            }
        }
        Handler::Container { selector, steps, dedup } => {
            let sel_display = bind_display(selector);
            let step_count = steps.len();
            let extract_count = steps
                .iter()
                .filter(|s| matches!(s, Step::Extract(_)))
                .count();
            CardInfo {
                description: format!("{} steps ({} extractions)", step_count, extract_count),
                selector: Some(sel_display),
                details: vec![("Dedup".to_string(), dedup.to_string())],
            }
        }
        Handler::SubSequence(seq) => CardInfo {
            description: seq.sequence_name.clone(),
            selector: None,
            details: vec![
                ("Steps".to_string(), seq.step_squence.len().to_string()),
                (
                    "Fields".to_string(),
                    seq.target_data.keys().cloned().collect::<Vec<_>>().join(", "),
                ),
            ],
        },
    }
}

// =========================================================================
// Main Card Widget
// =========================================================================

/// Create Handler Card Widget
///
/// Maintains the same layout specification as the existing handler_card.
pub fn handler_card<'a, Message: 'a>(
    index: usize,
    handler: &'a Handler,
) -> Element<'a, Message, Theme> {
    let meta = handler_meta(handler);
    let info = extract_card_info(handler);
    let (r, g, b) = meta.color;

    // ── Index Badge ──
    let index_badge = container(text(format!("{}", index)).size(12))
        .padding(Padding::from([4, 8]))
        .style(styles::index_badge_style);

    // ── Handler Type Badge ──
    let type_badge = container(
        row![
            text(meta.icon).size(14),
            Space::new().width(4),
            text(meta.name).size(12),
        ]
        .align_y(Alignment::Center),
    )
    .padding(Padding::from([4, 10]))
    .style(styles::badge_style(r, g, b));

    // ── Header Row: Index + Type Badge + Description ──
    let header_row = row![
        index_badge,
        Space::new().width(8),
        type_badge,
        Space::new().width(12),
        text(info.description.clone()).size(14),
    ]
    .align_y(Alignment::Center);

    // ── Selector Display (if present) ──
    let selector_section: Element<'a, Message, Theme> = if let Some(ref selector) = info.selector {
        container(
            row![
                text("Selector: ").size(11),
                container(text(truncate_text(selector, 50)).size(11))
                    .padding(Padding::from([2, 6]))
                    .style(styles::code_style),
            ]
            .align_y(Alignment::Center),
        )
        .padding(Padding::from([4, 0]))
        .into()
    } else {
        Space::new().height(0).into()
    };

    // ── Detailed Info Section ──
    let details_section: Element<'a, Message, Theme> = if !info.details.is_empty() {
        let detail_rows: Vec<Element<'a, Message, Theme>> = info
            .details
            .iter()
            .map(|(key, value)| {
                container(
                    row![
                        text(format!("{}: ", key)).size(11),
                        text(truncate_text(value, 40)).size(11),
                    ]
                    .align_y(Alignment::Center),
                )
                .padding(Padding::from([2, 6]))
                .style(styles::detail_row_style)
                .into()
            })
            .collect();

        column(detail_rows).spacing(4).into()
    } else {
        Space::new().height(0).into()
    };

    // ── Full Card Composition ──
    let card_content = column![header_row, selector_section, details_section,]
        .spacing(8)
        .padding(16);

    container(card_content)
        .width(Length::Fill)
        .style(styles::card_style)
        .into()
}

/// Compact Handler Card (Simplified Display)
pub fn compact_handler_card<'a, Message: 'a>(
    index: usize,
    handler: &'a Handler,
) -> Element<'a, Message, Theme> {
    let meta = handler_meta(handler);
    let info = extract_card_info(handler);
    let (r, g, b) = meta.color;

    let index_text = text(format!("#{}", index)).size(11);

    let type_badge = container(text(meta.icon).size(12))
        .padding(Padding::from([2, 4]))
        .style(styles::badge_style(r, g, b));

    let description = text(truncate_text(&info.description, 30)).size(12);

    let content = row![
        index_text,
        Space::new().width(6),
        type_badge,
        Space::new().width(8),
        description,
    ]
    .align_y(Alignment::Center)
    .padding(8);

    container(content)
        .width(Length::Fill)
        .style(styles::card_style)
        .into()
}

/// Empty Handler Card (Placeholder) — Same as existing
pub fn empty_handler_card<'a, Message: 'a>(index: usize) -> Element<'a, Message, Theme> {
    let index_badge = container(text(format!("{}", index)).size(12))
        .padding(Padding::from([4, 8]))
        .style(styles::index_badge_style);

    let placeholder_text = text("Empty Handler Slot").size(14);

    let content = row![index_badge, Space::new().width(12), placeholder_text,]
        .align_y(Alignment::Center)
        .padding(16);

    container(content)
        .width(Length::Fill)
        .style(|theme: &Theme| {
            let mut style = styles::card_style(theme);
            style.border.color = iced::Color::from_rgba(0.3, 0.3, 0.3, 0.3);
            style
        })
        .into()
}



// =========================================================================
// Utils
// =========================================================================

fn truncate_text(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}