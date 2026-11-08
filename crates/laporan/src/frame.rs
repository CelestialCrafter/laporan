use alloc::boxed::Box;
use alloc::string::{String, ToString};
use core::{any::Any, fmt};

/// A single arbitrary key-value pair.
///
/// This can be used to attach additional consumer exposed data to a [`Report`] such as:
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
        K: fmt::Display,
        V: fmt::Display,
    {
        Self {
            key: key.to_string(),
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

    // `type_name` is feature-gated with `serde` since we really only need it for serialization metadata
    #[cfg(feature = "serde")]
    fn type_name(&self) -> &'static str;

    fn clone_inner(&self) -> Box<dyn DynFrame>;
}

#[cfg(feature = "serde")]
impl serde::Serialize for Box<dyn DynFrame> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;

        struct SerializedFrame<'a>(&'a dyn erased_serde::Serialize);

        impl serde::Serialize for SerializedFrame<'_> {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                erased_serde::serialize(self.0, serializer)
            }
        }

        let frame = self.as_ref();
        let mut state = serializer.serialize_struct("Frame", 2)?;
        state.serialize_field("type_name", frame.type_name())?;
        state.serialize_field("data", &SerializedFrame(frame.serialize_inner()))?;
        state.end()
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

    #[cfg(feature = "serde")]
    fn type_name(&self) -> &'static str {
        core::any::type_name::<T>()
    }

    fn clone_inner(&self) -> Box<dyn DynFrame> {
        Box::new(self.clone())
    }
}
