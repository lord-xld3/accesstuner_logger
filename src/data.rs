use std::{
    hash::{Hash, Hasher},
    collections::{HashSet, HashMap},
};

/// A wrapper around the `f32` type to ensure consistent hashing and equality checks for floating point numbers.
/// This is useful to handle floating point comparisons and to use floats as keys in collections.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct F32(pub f32);

impl Eq for F32 {}

impl Hash for F32 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let bits = unsafe { std::mem::transmute::<f32, u32>(self.0) };
        bits.hash(state);
    }
}

/// A macro that provides a mechanism to define an enum and its associated methods.
/// It auto-generates methods to convert enum variants to strings (headers),
/// to convert strings back to enum variants, and to list all enum variants.
macro_rules! define_enum_and_variants {
    ($name:ident { $($variant:ident => $str:expr),* }) => {
        #[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
        pub enum $name {
            $($variant),*
        }

        impl $name {
            /// Converts an enum variant into its corresponding header string.
            pub fn to_header(&self) -> &'static str {
                match self {
                    $(Self::$variant => $str),*
                }
            }

            /// Converts a header string into its corresponding enum variant.
            /// Returns `None` if the string doesn't match any variant.
            pub fn from_header(header: &str) -> Option<Self> {
                match header {
                    $($str => Some(Self::$variant),)*
                    _ => None,
                }
            }

            /// Lists all the enum variants.
            pub fn variants() -> &'static [Self] {
                &[$(Self::$variant),*]
            }
        }
    };
}

// Utilizing the macro to define the `LogField` enum.
define_enum_and_variants!(LogField {
    MAFV => "MAF Voltage",
    MASS => "Mass Airflow",
    STFT => "Short Term FT",
    LTFT => "Long Term FT"
});

/// Represents the structured format for logging data with dynamic fields.
/// Uses a `HashMap` where the key is a `LogField` enum variant and the value is a vector of `f32` data points.
pub struct LogData {
    data: HashMap<LogField, Vec<f32>>,
}

impl LogData {
    /// Inserts a new data value into the appropriate vector based on the provided `LogField`.
    /// If the `LogField` is `MAFV`, ensures the value is unique before inserting.
    pub fn push(&mut self, field: LogField, value: f32, seen: &mut HashSet<F32>) {
        if let Some(vec) = self.data.get_mut(&field) {
            if let LogField::MAFV = field {
                if !seen.contains(&F32(value)) {
                    seen.insert(F32(value));
                } else {
                    return;
                }
            }
            vec.push(value);
        }
    }

    /// Retrieves the data vector associated with a given `LogField`.
    pub fn get(&self, field: &LogField) -> Option<&Vec<f32>> {
        self.data.get(field)
    }
}

impl Default for LogData {
    /// Provides a default instantiation of `LogData`.
    /// Initializes empty vectors for each `LogField` variant.
    fn default() -> Self {
        let mut data = HashMap::new();
        for &field in LogField::variants() {
            data.insert(field, Vec::new());
        }
        LogData { data }
    }
}
