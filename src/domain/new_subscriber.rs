use crate::domain::{
    subscriber_email::SubscriberEmail, subscriber_name::SubscriberName,
};

pub struct NewSubscriber {
    // We are not using `String` anymore!
    pub email: SubscriberEmail,
    pub name: SubscriberName,
}
