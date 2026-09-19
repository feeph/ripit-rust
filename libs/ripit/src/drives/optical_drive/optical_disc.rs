/*
    data object representing a disc
*/

// standard library imports
use std::fmt::Display;

// third-party imports
// <none>

// crate-provided imports
// <none>

// ------------------------------------------------------------------------
// public interface
// ------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize)]
pub struct Dvd {
    pub name: String,
    pub uid: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize)]
pub struct HdDvd {
    pub name: String,
    pub uid: String,
}

// AACS and BD+ (BDSVM) can be applied simultaneously on the same disc
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize)]
pub struct BluRay {
    /// The disc's volume name (label).
    pub name: String,

    /// The disc's unique ID.
    ///
    /// (The value depends on the operating system.)
    pub uid: String,

    /// Advanced Access Content System
    ///
    /// <https://en.wikipedia.org/wiki/Advanced_Access_Content_System>
    pub has_aacs: bool,

    /// BD+ (BD-SVM)
    ///
    /// <https://en.wikipedia.org/wiki/BD%2B>
    pub has_bdsvm: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize)]
pub enum OpticalDisc {
    Dvd(Dvd),
    HdDvd(HdDvd),
    BluRay(BluRay),
}

// augment the enumeration

impl OpticalDisc {

    pub fn get_name(&self) -> &str {
        match self {
            Self::Dvd(dvd) => &dvd.name,
            Self::HdDvd(hd_dvd) => &hd_dvd.name,
            Self::BluRay(bd) => &bd.name,
        }
    }

    pub fn get_type(&self) -> &str {
        match &self {
            Self::Dvd(_) => "DVD",
            Self::HdDvd(_) => "HD-DVD",
            Self::BluRay(_) => "Blu-Ray",
        }
    }

}

impl Display for OpticalDisc {

    // provides '.to_string()'
    // <https://doc.rust-lang.org/std/fmt/trait.Display.html>
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Self::Dvd(disc) => write!(f, "{} (DVD)", disc.name),
            Self::HdDvd(disc) => write!(f, "{} (HD-DVD)", disc.name),
            Self::BluRay(disc) => write!(f, "{} (Blu-Ray)", disc.name),
        }
    }

}

// ------------------------------------------------------------------------
// unit tests
// ------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_dvd() {
        let disc = OpticalDisc::Dvd(Dvd{
            name: "DVDVolume".to_string(),
            uid: "deadbeef".to_string(), 
        });
        // ----------------------------------------------------------------
        let computed = disc.to_string();
        let expected = "DVDVolume (DVD)".to_string();
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_format_hd_dvd() {
        let disc = OpticalDisc::HdDvd(HdDvd{
            name: "Wrong Horse".to_string(),
            uid: "cafebabe".to_string(), 
        });
        // ----------------------------------------------------------------
        let computed = disc.to_string();
        let expected = "Wrong Horse (HD-DVD)".to_string();
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

    #[test]
    fn test_format_bluray() {
        let disc = OpticalDisc::BluRay(BluRay{
            name: "Locked Down".to_string(),
            uid: "facefeed".to_string(),
            has_aacs: true,
            has_bdsvm: true,
        });
        // ----------------------------------------------------------------
        let computed = disc.to_string();
        let expected = "Locked Down (Blu-Ray)".to_string();
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }
}
