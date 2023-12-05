use super::changes_types::ChangeType;

#[derive(Clone)]
pub struct ChangeObject {
    path: String,
    x: ChangeType,
    y: ChangeType,
}

impl ChangeObject {
    /// Crea un ChangeObject
    pub fn new(path: String, x: ChangeType, y: ChangeType) -> Self {
        ChangeObject { path, x, y }
    }

    /// Crea un ChangeObject
    pub fn new_default() -> Self {
        ChangeObject {
            path: "".to_string(),
            x: ChangeType::Unmodified,
            y: ChangeType::Unmodified,
        }
    }

    /// Devuelve el formato en string del ChangeObject.
    pub fn to_string_change(&self) -> String {
        let x = self.x.get_short_type();
        let y = self.y.get_short_type();
        format!("{}{} {}", y, x, self.path)
    }
}
