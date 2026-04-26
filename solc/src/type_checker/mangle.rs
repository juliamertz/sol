use crate::traits::AsStr;

pub fn assoc_item(def_name: impl AsStr, item_name: impl AsStr) -> String {
    format!("_{}_{}", def_name.as_str(), item_name.as_str())
}
