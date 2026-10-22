//! Discord's limits on components, in one place. A [`View`](crate::command::View) answered by a
//! handler (or sent through REST) is checked against them in one walk, so a violation is a
//! [`HandlerError`](crate::command::HandlerError) naming the limit instead of a bare "This
//! interaction failed". `rsx!` checks the counts it can see at compile time.

use std::ops::RangeInclusive;

/// Components in a message, nested ones included
pub const MAX_COMPONENTS: usize = 40;

/// Characters of text across every text display of a message
pub const MAX_TEXT_LENGTH: usize = 4000;

/// Buttons in an action row (or exactly one select)
pub const MAX_ROW_BUTTONS: usize = 5;

/// Options of a string select
pub const MAX_SELECT_OPTIONS: usize = 25;

/// Options of a radio group
pub const RADIO_OPTIONS: RangeInclusive<usize> = 2..=10;

/// Options of a checkbox group
pub const CHECKBOX_OPTIONS: RangeInclusive<usize> = 1..=10;

/// Text displays in a section
pub const SECTION_TEXTS: RangeInclusive<usize> = 1..=3;

/// Items of a media gallery
pub const GALLERY_ITEMS: RangeInclusive<usize> = 1..=10;

/// Characters of a `custom_id`
pub const MAX_CUSTOM_ID_LENGTH: usize = crate::command::custom_id::MAX_LEN;

/// Characters of a button's label
pub const MAX_BUTTON_LABEL_LENGTH: usize = 80;

/// Top-level components of a modal
pub const MODAL_COMPONENTS: RangeInclusive<usize> = 1..=5;

/// Characters of a modal's title
pub const MAX_MODAL_TITLE_LENGTH: usize = 45;
