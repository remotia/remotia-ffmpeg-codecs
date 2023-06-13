macro_rules! builder_set {
    ($attr_name: ident, $attr_type: ty) => {
        pub fn $attr_name(mut self, $attr_name: $attr_type) -> Self {
            self.$attr_name = Some($attr_name);
            self
        }
    }
}

pub fn unwrap_mandatory<V>(value: Option<V>) -> V {
    value.expect("Missing mandatory field")
}