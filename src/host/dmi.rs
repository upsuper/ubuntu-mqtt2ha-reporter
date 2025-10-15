use std::fs;

pub struct Dmi {
    pub manufacturer: Option<String>,
    pub model: Option<String>,
}

pub fn get_dmi() -> Dmi {
    let manufacturer = fs::read_to_string("/sys/class/dmi/id/sys_vendor")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s != "System manufacturer");
    let model = fs::read_to_string("/sys/class/dmi/id/product_name")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s != "System Product Name");
    Dmi {
        manufacturer,
        model,
    }
}
