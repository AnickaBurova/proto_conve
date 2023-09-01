//! These are convenient traits to convert the proto values in to the appropriate rust values

/// Unwrap the value from the optional proto value, if the value is missing, an error is returned and
/// convert it in to result if appropriate.
trait Required {
    /// The result of this type
    type Result;
    /// Convert the value from proto buf option in to Result, if the value is missing (None) the
    /// error is returned
    fn required(self) -> anyhow::Result<Self::Result>;
}

/// Implementation of the required for all the options.
impl<T> Required for Option<T> {
    type Result = T;

    fn required(self) -> anyhow::Result<Self::Result> {
        if let Some(value) = self {
            Ok(value)
        } else {
            anyhow::bail!("Required is missing");
        }
    }
}

/// Convert the value in to appropriate proto value
pub trait ToProto {
    /// The proto value type
    type Result;

    /// Convert the value in to proto value
    fn to_proto(&self) -> Self::Result;

    /// Convert the value in to proto value and wrap it in to option
    fn to_proto_option(&self) -> Option<Self::Result> {
        Some(self.to_proto())
    }
}

/// Convert the value in to proto value
pub trait ToProtoAlias<T> {
    fn to_proto(&self) -> T;

    fn to_proto_option(&self) -> Option<T> {
        Some(self.to_proto())
    }
}

/// Convert the value in to proto value wrapped in an option
pub trait ToProtoOption<T> {
    fn to_proto(&self) -> Option<T>;
}

/// Convert the proto value in to the appropriate value
pub trait FromProto {
    type Result;
    fn from_proto(self) -> anyhow::Result<Self::Result>;
}

impl<T: FromProto> FromProto for Option<T> {
    type Result = Option<T::Result>;

    fn from_proto(self) -> anyhow::Result<Self::Result> {
        self.map(|v| v.from_proto()).transpose()
    }
}

impl ToProtoAlias<bool> for bool {
    fn to_proto(&self) -> bool {
        *self
    }
}
/// Implement FromProto and ToProto to some specific chrono types when the proto type has a specific format.
/// ```proto
/// // Duration specified in seconds and nanoseconds
/// message Duration {
///    int64 seconds = 1;
///    uint32 nanos = 2;
/// }
/// // Timestamp specified in seconds and nanoseconds
/// message DateTimeUtc {
///   int64 seconds = 1;
///   uint32 nanos = 2;
/// }
/// ```
#[macro_export]
macro_rules! impl_traits {
    ($type: ident, chrono::Duration) => {
        impl FromProto for $type {
            type Result = chrono::Duration;

            fn from_proto(self) -> anyhow::Result<Self::Result> {
                if self.seconds >= 0 {
                    Ok((chrono::Duration::seconds(self.seconds)
                        + chrono::Duration::nanoseconds(self.nanos as i64))
                        )
                } else {
                    Ok((chrono::Duration::seconds(self.seconds)
                        - chrono::Duration::nanoseconds(self.nanos as i64))
                       )
                }
            }
        }
        impl ToProto for chrono::Duration {
            type Result = $type;

            fn to_proto(&self) -> Self::Result {
                let (seconds, nanos) = if self >= &chrono::Duration::zero() {
                    let time = self.to_std().unwrap();
                    (time.as_secs() as i64, time.subsec_nanos())
                } else {
                    let time = (-*self).to_std().unwrap();
                    (-(time.as_secs() as i64), time.subsec_nanos())
                };
                $type { seconds, nanos }
            }
        }
    };
    ($type: ident, chrono::DateTime<chrono::Utc>) => {
        impl ToProto for chrono::DateTime<chrono::Utc> {
            type Result = $type;
            fn to_proto(&self) -> Self::Result {
                let value = self.naive_utc();
                $type {
                    seconds: value.timestamp(),
                    nanos: value.timestamp_subsec_nanos(),
                }
            }
        }

        impl FromProto for $type {
            type Result = chrono::DateTime<chrono::Utc>;

            fn from_proto(self) -> anyhow::Result<Self::Result> {
                use chrono::NaiveDateTime;
                match NaiveDateTime::from_timestamp_opt(self.seconds, self.nanos) {
                    None => {
                        anyhow::bail!(
                            "Failed to parse timestamp: {} s and {} ns",
                            self.seconds,
                            self.nanos
                        );
                    }
                    Some(value) => Ok(chrono::DateTime::from_naive_utc_and_offset(value, chrono::Utc)),
                }
            }
        }
    };
}


/// Convert list of proto elements in a vector
impl<T: FromProto> FromProto for Vec<T> {
    type Result = Vec<T::Result>;

    fn from_proto(self) -> anyhow::Result<Self::Result> {
        let mut result = Vec::new();
        for item in self.into_iter() {
            result.push(item.from_proto()?);
        }
        Ok(result)
    }
}

/// Convert list of elements in vector to proto list
impl<T: ToProto> ToProto for Vec<T> {
    type Result = Vec<T::Result>;

    fn to_proto(&self) -> Self::Result {
        self.iter().map(|v| v.to_proto()).collect()
    }
}

impl<T: ToProto> ToProto for Option<T> {
    type Result = Option<T::Result>;

    fn to_proto(&self) -> Self::Result {
        self.as_ref().map(|v| v.to_proto())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Duration specified in seconds and nanoseconds
    #[derive(Debug, PartialEq, Clone)]
    pub struct ProtoDuration {
        seconds: i64,
        nanos: u32,
    }

    #[derive(Debug, PartialEq, Clone)]
    pub struct ProtoDateTimeUtc {
        seconds: i64,
        nanos: u32,
    }

    impl_traits! (ProtoDuration, chrono::Duration);
    impl_traits! (ProtoDateTimeUtc, chrono::DateTime<chrono::Utc>);

    #[test]
    fn test_duration() {
        let pd = ProtoDuration {
            seconds: 1,
            nanos: 2,
        };

        let d = pd.clone().from_proto().unwrap();
        let pd2 = d.to_proto();
        assert_eq!(pd, pd2);

        let pd = ProtoDuration {
            seconds: -1,
            nanos: 2,
        };

        let d = pd.clone().from_proto().unwrap();
        let pd2 = d.to_proto();
        assert_eq!(pd, pd2);
    }

    #[test]
    fn test_date_time() {
        let pd = ProtoDateTimeUtc {
            seconds: -1,
            nanos: 2,
        };

        let d = pd.clone().from_proto().unwrap();
        let pd2 = d.to_proto();
        assert_eq!(pd, pd2);
    }
}