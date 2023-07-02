mod get;
mod post;

pub use get::change_password_form;
pub use post::{change_password, MAX_PASSWORD_LENGTH, MIN_PASSWORD_LENGTH};
