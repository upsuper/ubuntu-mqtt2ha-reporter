use serde::{Serialize, Serializer, ser::SerializeMap};

pub fn serialize_as_map<S, K, V>(value: &[(K, V)], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    K: Serialize,
    V: Serialize,
{
    let mut map = serializer.serialize_map(Some(value.len()))?;
    for (k, v) in value {
        map.serialize_entry(k, v)?;
    }
    map.end()
}
