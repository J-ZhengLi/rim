use serde::ser::SerializeSeq;

/// A customized serialize function that serializing empty `Vec` to `None`.
///
/// This is useful when you don't want a `xxx = []` appears in your serialized file.
///
/// # Example
/// You can use this function on any struct field with `Vec` or `slice` type, such as:
/// ```ignore
/// #[derive(Default)]
/// struct Foo {
///     msg: String,
///     #[serde(serialize_with = "ser_empty_vec_to_none")]
///     seq: Vec<String>,
/// }
/// ```
///
/// Without this function, the default of above struct will get serialized to:
/// ```toml
/// msg = ""
/// seq = []
/// ```
///
/// But with this function, the default implementation will be serialized to:
/// ```toml
/// msg = ""
/// ```
pub fn ser_empty_vec_to_none<S, T>(vec: &[T], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
    T: serde::Serialize,
{
    if vec.is_empty() {
        serializer.serialize_none()
    } else {
        let mut seq = serializer.serialize_seq(Some(vec.len()))?;
        for elem in vec {
            seq.serialize_element(elem)?;
        }
        seq.end()
    }
}
