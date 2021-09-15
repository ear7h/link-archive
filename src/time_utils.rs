use serde::{Serialize, Serializer};

#[derive(Debug)]
pub struct Time(time::OffsetDateTime);

pub(crate) const TIME_FORMAT : &'static [time::format_description::FormatItem<
    'static,
>] = time::macros::format_description!(
    "[year]-[month]-[day] [hour]:[minute]:[second]"
);

impl Serialize for Time {
    fn serialize<S>(
        &self,
        serializer : S,
    ) -> std::result::Result<S::Ok, S::Error>
    where
        S : Serializer,
    {
        self.format(&TIME_FORMAT).unwrap().serialize(serializer)
    }
}

impl From<time::OffsetDateTime> for Time {
    fn from(t : time::OffsetDateTime) -> Self {
        Time(t)
    }
}

impl std::ops::Deref for Time {
    type Target = time::OffsetDateTime;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Time {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
