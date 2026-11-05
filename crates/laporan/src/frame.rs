use std::{any::Any, fmt, sync::Arc};

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

impl From<Context> for Frame {
    fn from(value: Context) -> Self {
        Frame::new(value)
    }
}

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

impl From<Attachment> for Frame {
    fn from(value: Attachment) -> Self {
        Frame::new(value)
    }
}

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

impl From<Suggestion> for Frame {
    fn from(value: Suggestion) -> Self {
        Frame::new(value)
    }
}

#[derive(Clone)]
pub struct Frame {
    inner: Arc<dyn DynFrame>,
}

impl Frame {
    pub fn new(payload: impl DynFrame) -> Self {
        Self {
            inner: Arc::new(payload),
        }
    }

    pub fn payload(&self) -> &dyn Any {
        self.inner.as_any()
    }
}

impl fmt::Debug for Frame {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.inner.as_debug(), formatter)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Frame {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.inner.as_serialize().serialize(serializer)
    }
}

trait DynFrame: Send + Sync + 'static {
    fn as_debug(&self) -> &dyn fmt::Debug;
    fn as_any(&self) -> &dyn Any;
    #[cfg(feature = "serde")]
    fn as_serialize(&self) -> &dyn erased_serde::Serialize;
}

impl<T: fmt::Debug + serde::Serialize + Send + Sync + 'static> DynFrame for T {
    fn as_debug(&self) -> &dyn fmt::Debug {
        self
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_serialize(&self) -> &dyn erased_serde::Serialize {
        self
    }
}

/// A single piece of information inside of a [`Report`].
///
/// `Frame`s can be created via the [`Self::suggestion`], [`Self::context`], or [`Self::attachment`] methods.
pub struct FrameWrapper<T> {
    payload: T,
    #[cfg(feature = "serde")]
    serialize: fn(&T) -> &dyn erased_serde::Serialize,
    debug: fn(&T) -> &dyn fmt::Debug,
}

impl<T: Send + Sync + 'static> FrameWrapper<T> {
    /// Constructs a new [`Frame`].
    #[inline]
    pub fn new(payload: T) -> Self {
        Self {
            payload,
            #[cfg(feature = "serde")]
            serialize: |_| &(),
            debug: |_| &"",
        }
    }

    #[cfg(feature = "serde")]
    #[inline]
    pub fn with_serializer(mut self) -> Self
    where
        T: serde::Serialize,
    {
        self.serialize = |value| value as &dyn erased_serde::Serialize;
        self
    }

    pub fn build(self) -> Frame {
        Frame::new(self)
    }
}

impl<T: Send + Sync + 'static> DynFrame for FrameWrapper<T> {
    #[inline]
    fn as_debug(&self) -> &dyn fmt::Debug {
        (self.debug)(&self.payload)
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        &self.payload
    }

    #[cfg(feature = "serde")]
    #[inline]
    fn as_serialize(&self) -> &dyn erased_serde::Serialize {
        (self.serialize)(&self.payload)
    }
}
