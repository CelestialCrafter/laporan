use std::{any::Any, fmt};

/// A single key-value pair.
///
/// This can be used to attach additional data to a [`Report`] such as:
/// ids, timestamps, commands, etc.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Context {
    /// Name of the context field
    pub key: String,
    /// Value associated with the key
    pub value: String,
}

impl Context {
    /// Constructs a new [`Context`].
    #[inline]
    pub fn new<K, V>(key: K, value: V) -> Self
    where
        K: Into<String>,
        V: fmt::Display,
    {
        Self {
            key: key.into(),
            value: value.to_string(),
        }
    }
}

impl Frame for Context {}

/// A long-form piece of information.
///
/// This can be used to attach more detailed data such as:
/// stderr, build logs, files, etc.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Attachment {
    pub content: String,
}

impl Attachment {
    /// Constructs a new [`Attachment`].
    #[inline]
    pub fn new(content: impl fmt::Display) -> Self {
        Self {
            content: content.to_string(),
        }
    }
}

impl Frame for Attachment {}

/// An inline suggestion
///
/// This can be used to suggest actions to users to resolve issues.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Suggestion {
    pub content: String,
}

impl Suggestion {
    /// Constructs a new [`Suggestion`].
    #[inline]
    pub fn new(content: impl fmt::Display) -> Self {
        Self {
            content: content.to_string(),
        }
    }
}

impl Frame for Suggestion {}

#[cfg(not(feature = "serde"))]
pub trait Frame: Clone + fmt::Debug + Send + Sync + 'static {}

#[cfg(feature = "serde")]
pub trait Frame: serde::Serialize + Clone + fmt::Debug + Send + Sync + 'static {}

pub(crate) trait DynFrame: fmt::Debug + Send + Sync + Any {
    #[cfg(feature = "serde")]
    fn serialize_inner(&self) -> &dyn erased_serde::Serialize;
    fn clone_inner(&self) -> Box<dyn DynFrame>;
}

#[cfg(feature = "serde")]
impl serde::Serialize for Box<dyn DynFrame> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        erased_serde::serialize(self.as_ref().serialize_inner(), serializer)
    }
}

impl Clone for Box<dyn DynFrame> {
    fn clone(&self) -> Self {
        self.as_ref().clone_inner()
    }
}

impl<T: Frame> DynFrame for T {
    #[cfg(feature = "serde")]
    fn serialize_inner(&self) -> &dyn erased_serde::Serialize {
        self
    }

    fn clone_inner(&self) -> Box<dyn DynFrame> {
        Box::new(self.clone())
    }
}
