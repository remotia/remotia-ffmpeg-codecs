/// Generates a builder-style setter method for an optional field.
///
/// For a field named `foo` of type `Option<T>`, calling `builder_set!(foo, T)` produces:
///
/// ```ignore
/// pub fn foo(mut self, foo: T) -> Self {
///     self.foo = Some(foo);
///     self
/// }
/// ```
///
/// This is used by [`ScalerBuilder`], [`EncoderBuilder`], and [`DecoderBuilder`] to
/// reduce boilerplate for their setter methods.
///
/// [`ScalerBuilder`]: crate::scaling::ScalerBuilder
/// [`EncoderBuilder`]: crate::encoders::EncoderBuilder
/// [`DecoderBuilder`]: crate::decoders::DecoderBuilder
macro_rules! builder_set {
    ($attr_name: ident, $attr_type: ty) => {
        pub fn $attr_name(mut self, $attr_name: $attr_type) -> Self {
            self.$attr_name = Some($attr_name);
            self
        }
    }
}

/// Unwraps a mandatory builder field, panicking with a descriptive message if missing.
///
/// This is called by [`ScalerBuilder::build`], [`EncoderBuilder::build`], and
/// [`DecoderBuilder::build`] for fields that have no sensible default.
///
/// [`ScalerBuilder::build`]: crate::scaling::ScalerBuilder::build
/// [`EncoderBuilder::build`]: crate::encoders::EncoderBuilder::build
/// [`DecoderBuilder::build`]: crate::decoders::DecoderBuilder::build
pub fn unwrap_mandatory<V>(value: Option<V>) -> V {
    value.expect("Missing mandatory field")
}
