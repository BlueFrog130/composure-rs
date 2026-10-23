//! Discord's component limits ([`types::limits`](crate::types::limits)), checked in one walk
//! without allocating when a View is answered with or sent

use std::ops::RangeInclusive;

use crate::types::{Component, ModalCallbackData, limits::*};

use super::ViewError;

fn at_most(what: &'static str, max: usize, found: usize) -> Result<(), ViewError> {
    within(what, 0..=max, found)
}

fn within(what: &'static str, range: RangeInclusive<usize>, found: usize) -> Result<(), ViewError> {
    if range.contains(&found) {
        Ok(())
    } else {
        Err(ViewError::Limit {
            what,
            min: *range.start(),
            max: *range.end(),
            found,
        })
    }
}

fn chars(text: &str) -> usize {
    text.chars().count()
}

fn custom_id(id: &str) -> Result<(), ViewError> {
    at_most("custom_id characters", MAX_CUSTOM_ID_LENGTH, chars(id))
}

#[derive(Default)]
struct Totals {
    components: usize,
    text: usize,
}

/// A Components V2 message's components
pub(crate) fn check_message(components: &[Component]) -> Result<(), ViewError> {
    let mut totals = Totals::default();
    walk(components, &mut totals)?;
    at_most("components in a message", MAX_COMPONENTS, totals.components)?;
    at_most(
        "characters of text in a message",
        MAX_TEXT_LENGTH,
        totals.text,
    )
}

/// A modal: its title, `custom_id`, top-level count, and every component in it
pub(crate) fn check_modal(data: &ModalCallbackData) -> Result<(), ViewError> {
    at_most(
        "characters in a modal's title",
        MAX_MODAL_TITLE_LENGTH,
        chars(&data.title),
    )?;
    custom_id(&data.custom_id)?;
    let components = data.components.as_deref().unwrap_or_default();
    within(
        "top-level components in a modal",
        MODAL_COMPONENTS,
        components.len(),
    )?;
    walk(components, &mut Totals::default())
}

fn walk(components: &[Component], totals: &mut Totals) -> Result<(), ViewError> {
    for component in components {
        one(component, totals)?;
    }
    Ok(())
}

fn one(component: &Component, totals: &mut Totals) -> Result<(), ViewError> {
    totals.components += 1;
    match component {
        Component::Container(container) => walk(&container.components, totals),
        Component::Section(section) => {
            within(
                "text displays in a section",
                SECTION_TEXTS,
                section.components.len(),
            )?;
            walk(&section.components, totals)?;
            one(&section.accessory, totals)
        }
        Component::ActionRow(row) => {
            let selects = row
                .components
                .iter()
                .filter(|component| !matches!(component, Component::Button(_)))
                .count();
            if selects > 0 {
                at_most("components in a row with a select", 1, row.components.len())?;
            } else {
                at_most("buttons in a row", MAX_ROW_BUTTONS, row.components.len())?;
            }
            walk(&row.components, totals)
        }
        Component::Label(label) => one(&label.component, totals),
        Component::TextDisplay(text) => {
            totals.text += chars(&text.content);
            Ok(())
        }
        Component::Button(button) => {
            if let Some(label) = &button.label {
                at_most(
                    "characters in a button's label",
                    MAX_BUTTON_LABEL_LENGTH,
                    chars(label),
                )?;
            }
            button.custom_id.as_deref().map_or(Ok(()), custom_id)
        }
        Component::StringSelect(select) => {
            let options = select.options.as_deref().unwrap_or_default();
            at_most("options in a select", MAX_SELECT_OPTIONS, options.len())?;
            // Discord drops the whole response over a repeated value, with no error to the bot
            for (i, option) in options.iter().enumerate() {
                if options[..i].iter().any(|other| other.value == option.value) {
                    return Err(ViewError::Duplicate {
                        what: "option value in a select",
                        value: option.value.clone(),
                    });
                }
            }
            custom_id(&select.custom_id)
        }
        Component::UserSelect(select) => custom_id(&select.custom_id),
        Component::RoleSelect(select) => custom_id(&select.custom_id),
        Component::MentionableSelect(select) => custom_id(&select.custom_id),
        Component::ChannelSelect(select) => custom_id(&select.custom_id),
        Component::TextInput(input) => custom_id(&input.custom_id),
        Component::RadioGroup(group) => {
            within(
                "options in a radio group",
                RADIO_OPTIONS,
                group.options.len(),
            )?;
            custom_id(&group.custom_id)
        }
        Component::CheckboxGroup(group) => {
            within(
                "options in a checkbox group",
                CHECKBOX_OPTIONS,
                group.options.len(),
            )?;
            custom_id(&group.custom_id)
        }
        Component::Checkbox(checkbox) => custom_id(&checkbox.custom_id),
        Component::FileUpload(upload) => custom_id(&upload.custom_id),
        Component::MediaGallery(gallery) => within(
            "items in a media gallery",
            GALLERY_ITEMS,
            gallery.items.len(),
        ),
        Component::Thumbnail(_)
        | Component::File(_)
        | Component::Separator(_)
        | Component::Unknown(_) => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{
        ActionRow, ButtonComponent, ButtonStyle, CheckboxGroup, Component, Container, GroupOption,
        MediaGallery, MediaGalleryItem, ModalCallbackData, RadioGroup, Section, SelectOption,
        StringSelect,
    };

    use super::*;

    fn button(id: &str) -> Component {
        ButtonComponent::new(
            ButtonStyle::Primary,
            Some("b".into()),
            None,
            Some(id.into()),
            None,
            None,
        )
        .into()
    }

    fn select(options: usize) -> Component {
        let options = (0..options)
            .map(|i| SelectOption::new(i.to_string(), i.to_string(), None, None, None))
            .collect();
        Component::StringSelect(StringSelect::new(
            "s".into(),
            Some(options),
            None,
            None,
            None,
            None,
            None,
        ))
    }

    fn group(n: usize) -> Vec<GroupOption> {
        (0..n)
            .map(|i| GroupOption::new(i.to_string(), i.to_string()))
            .collect()
    }

    fn limit(result: Result<(), ViewError>) -> (&'static str, usize) {
        match result {
            Err(ViewError::Limit { what, found, .. }) => (what, found),
            other => panic!("not a limit error: {other:?}"),
        }
    }

    fn modal(components: Vec<Component>) -> ModalCallbackData {
        ModalCallbackData {
            custom_id: "m".into(),
            title: "Title".into(),
            components: Some(components),
            tts: None,
            content: None,
            embeds: None,
            allowed_mentions: None,
            flags: None,
        }
    }

    #[test]
    fn within_limits_passes() {
        let message = [Container::new(vec![
            Component::text("hi"),
            Section::new(vec![Component::text("a")], button("x")).into(),
            ActionRow::new(vec![button("1"), button("2")]).into(),
        ])
        .into()];
        check_message(&message).unwrap();
    }

    #[test]
    fn components_in_a_message() {
        let nested = Container::new(vec![Component::text("x"); 40]).into();
        assert_eq!(
            limit(check_message(&[nested])),
            ("components in a message", 41)
        );
    }

    #[test]
    fn text_in_a_message() {
        let text = [
            Component::text("x".repeat(3000)),
            Component::text("é".repeat(1001)),
        ];
        assert_eq!(
            limit(check_message(&text)),
            ("characters of text in a message", 4001)
        );
    }

    #[test]
    fn buttons_or_one_select_per_row() {
        let row = ActionRow::new((0..6).map(|i| button(&i.to_string())).collect()).into();
        assert_eq!(limit(check_message(&[row])), ("buttons in a row", 6));
        let row = ActionRow::new(vec![select(1), button("b")]).into();
        assert_eq!(
            limit(check_message(&[row])),
            ("components in a row with a select", 2)
        );
    }

    #[test]
    fn options_of_a_select() {
        let row = ActionRow::new(vec![select(26)]).into();
        assert_eq!(limit(check_message(&[row])), ("options in a select", 26));
    }

    #[test]
    fn option_values_of_a_select_are_unique() {
        let mut row = select(3);
        if let Component::StringSelect(select) = &mut row {
            select.options.as_mut().unwrap()[2].value = "0".into();
        }
        let error = check_message(&[ActionRow::new(vec![row]).into()]).unwrap_err();
        assert_eq!(
            error,
            ViewError::Duplicate {
                what: "option value in a select",
                value: "0".into()
            }
        );
        assert_eq!(
            error.to_string(),
            "option value in a select \"0\" repeats, Discord requires each to be unique"
        );
    }

    #[test]
    fn options_of_radio_and_checkbox_groups() {
        let radio = Component::RadioGroup(RadioGroup::new("r", group(1)));
        assert_eq!(
            limit(check_modal(&modal(vec![radio]))),
            ("options in a radio group", 1)
        );
        let checkbox = Component::CheckboxGroup(CheckboxGroup::new("c", group(11)));
        assert_eq!(
            limit(check_modal(&modal(vec![checkbox]))),
            ("options in a checkbox group", 11)
        );
    }

    #[test]
    fn texts_in_a_section() {
        let section = Section::new(vec![Component::text("t"); 4], button("x")).into();
        assert_eq!(
            limit(check_message(&[section])),
            ("text displays in a section", 4)
        );
    }

    #[test]
    fn items_in_a_gallery() {
        let items = vec![MediaGalleryItem::new("https://example.com/a.png"); 11];
        let gallery = MediaGallery::new(items).into();
        assert_eq!(
            limit(check_message(&[gallery])),
            ("items in a media gallery", 11)
        );
    }

    #[test]
    fn custom_ids() {
        let row = ActionRow::new(vec![button(&"x".repeat(101))]).into();
        assert_eq!(limit(check_message(&[row])), ("custom_id characters", 101));
    }

    #[test]
    fn button_labels() {
        let long = ButtonComponent::new(
            ButtonStyle::Link,
            Some("l".repeat(81)),
            None,
            None,
            Some("https://example.com".into()),
            None,
        );
        let row = ActionRow::new(vec![long.into()]).into();
        assert_eq!(
            limit(check_message(&[row])),
            ("characters in a button's label", 81)
        );
    }

    #[test]
    fn modals() {
        assert_eq!(
            limit(check_modal(&modal(vec![Component::text("t"); 6]))),
            ("top-level components in a modal", 6)
        );
        let mut long = modal(vec![Component::text("t")]);
        long.title = "t".repeat(46);
        assert_eq!(
            limit(check_modal(&long)),
            ("characters in a modal's title", 46)
        );
    }

    #[test]
    fn errors_name_the_limit() {
        let error = ViewError::Limit {
            what: "buttons in a row",
            min: 0,
            max: 5,
            found: 6,
        };
        assert_eq!(
            error.to_string(),
            "6 buttons in a row, Discord allows at most 5"
        );
        let error = ViewError::Limit {
            what: "options in a radio group",
            min: 2,
            max: 10,
            found: 1,
        };
        assert_eq!(
            error.to_string(),
            "1 options in a radio group, Discord allows 2 to 10"
        );
    }
}
