#[derive(Clone, Debug)]
pub enum Pattern {
    StartsWith(String, String, bool, bool),
    EndsWith(String, String, bool, bool),
    RelativeToDirLevel(String, String, bool),
    NotRelativeToDirLevel(String, String, bool),
    // si queda tiempo, agregar: a*a, ? (MatchesOne), [a-z] (MatchesRange), **
}

impl Pattern {
    /// Devuelve true si el patrón debe ser ignorado, es decir, si es de la forma: !patrón
    pub fn negate_pattern(&self) -> bool {
        match self {
            Self::StartsWith(_, _, _, negate) => negate.to_owned(),
            Self::EndsWith(_, _, _, negate) => negate.to_owned(),
            Self::RelativeToDirLevel(_, _, negate) => negate.to_owned(),
            Self::NotRelativeToDirLevel(_, _, negate) => negate.to_owned(),
        }
    }

    /// Devuelve true si el patrón se refiere a un path relativo al directorio actual.
    pub fn is_relative(&self) -> bool {
        match self {
            Self::StartsWith(_, _, is_relative, _) => is_relative.to_owned(),
            Self::EndsWith(_, _, is_relative, _) => is_relative.to_owned(),
            Self::RelativeToDirLevel(_, _, _) => true,
            Self::NotRelativeToDirLevel(_, _, _) => false,
        }
    }

    /// Devuelve el patrón leído del archivo de exclusión.
    pub fn get_pattern_read(&self) -> String {
        match self {
            Self::StartsWith(pattern_extracted, _, _, _) => pattern_extracted.to_string(),

            Self::EndsWith(pattern_extracted, _, _, _) => pattern_extracted.to_string(),
            Self::RelativeToDirLevel(pattern_extracted, _, _) => pattern_extracted.to_string(),
            Self::NotRelativeToDirLevel(pattern_extracted, _, _) => pattern_extracted.to_string(),
        }
    }

    /// Devuelve el formato en string del patrón.
    pub fn to_string(
        &self,
        path: &str,
        gitignore_path: &str,
        line_number: usize,
        verbose: bool,
    ) -> String {
        if verbose {
            return format!(
                "{}:{}:{}\t{}\n",
                gitignore_path,
                line_number,
                self.get_pattern_read(),
                path
            );
        }
        format!("{}\n", path)
    }
}
