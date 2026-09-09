use cuboid_core::Language;
use windows::Win32::Globalization::GetUserDefaultLocaleName;

/// The buffer size Win32 requires for a locale name, terminator included.
const LOCALE_NAME_MAX_LENGTH: usize = 85;

/// The interface language the user locale of this Windows session asks for.
///
/// Cuboid only reads this when it has no stored configuration: once a document
/// exists, the language it records is the user's own choice.
pub fn system_language() -> Language {
    match user_default_locale_name() {
        Some(tag) => {
            let language = Language::from_locale_tag(&tag);
            tracing::info!(locale = %tag, ?language, "picked the interface language from the user locale");
            language
        }
        None => {
            tracing::warn!("Windows reported no user locale; falling back to English");
            Language::default()
        }
    }
}

/// The BCP 47 tag of the user locale, such as `it-IT`, or `None` when Windows
/// reports nothing usable.
fn user_default_locale_name() -> Option<String> {
    let mut buffer = [0_u16; LOCALE_NAME_MAX_LENGTH];
    let written = unsafe { GetUserDefaultLocaleName(&mut buffer) };
    if written <= 0 {
        return None;
    }

    // The count includes the terminating null.
    let length = (written as usize).saturating_sub(1).min(buffer.len());
    let tag = String::from_utf16_lossy(&buffer[..length]);
    if tag.is_empty() { None } else { Some(tag) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_user_locale_names_a_language() {
        // Whatever this machine is set to, a locale must resolve to a language
        // Cuboid can render.
        let _ = system_language();
    }

    #[test]
    fn windows_locale_tags_map_to_an_interface_language() {
        assert_eq!(Language::from_locale_tag("it-IT"), Language::Italian);
        assert_eq!(Language::from_locale_tag("it-CH"), Language::Italian);
        assert_eq!(Language::from_locale_tag("en-US"), Language::English);
        assert_eq!(Language::from_locale_tag("de-DE"), Language::English);
        assert_eq!(Language::from_locale_tag(""), Language::English);
    }
}
