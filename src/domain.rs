use unicode_segmentation::UnicodeSegmentation;

pub struct NewSubscriber {
    pub email: String,
    pub name: SubscriberName,
}

pub struct SubscriberName(String);

impl SubscriberName {
    pub fn parse(s: String) -> SubscriberName {
        let is_empty_or_whitespace = s.trim().is_empty();
        let is_to_long = s.graphemes(true).count() > 256;
        let forbidden_chars = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];
        let contains_forbidden_chars =
            s.chars().any(|g| forbidden_chars.contains(&g));

        if is_empty_or_whitespace || is_to_long || contains_forbidden_chars {
            panic!("{} is not a valid subscriber name.", s)
        }

        Self(s)
    }
}
