//! Les versions validées des émulateurs et des cores.
//!
//! En netplay, tout le monde doit avoir exactement la même version. Le
//! launcher ne prend donc jamais « la dernière » : il installe celle que le
//! fichier `versions.json` (hébergé sur GitHub) a validée, avec son empreinte.
//!
//! ```json
//! {
//!   "revision": 3,
//!   "composants": {
//!     "retroarch":  { "version": "1.21.0", "url": "https://…", "sha256": "…" },
//!     "dolphin":    { "version": "2512",   "url": "https://…", "sha256": "…" },
//!     "core:snes9x":{ "version": "1.63",   "url": "https://…", "sha256": "…" }
//!   }
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Un composant à installer (émulateur ou core).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Composant {
    pub version: String,
    pub url: String,
    pub sha256: String,
}

/// Le fichier des versions validées.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Validees {
    /// Monte à chaque validation : une révision plus petite que celle
    /// installée est ignorée (pas de retour en arrière accidentel).
    pub revision: u32,
    pub composants: BTreeMap<String, Composant>,
}

/// Ce qui est installé sur ce PC : nom → version.
pub type Installees = BTreeMap<String, String>;

/// Ce qu'il faut faire pour un composant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Installer { nom: String, composant: Composant },
    MettreAJour { nom: String, de: String, composant: Composant },
}

impl Validees {
    pub fn lire(json: &str) -> Result<Validees, String> {
        serde_json::from_str(json).map_err(|e| format!("versions.json illisible : {e}"))
    }

    /// Les installations et mises à jour à faire pour être à la version validée.
    pub fn a_faire(&self, installees: &Installees) -> Vec<Action> {
        self.composants
            .iter()
            .filter_map(|(nom, composant)| match installees.get(nom) {
                None => Some(Action::Installer {
                    nom: nom.clone(),
                    composant: composant.clone(),
                }),
                Some(v) if *v != composant.version => Some(Action::MettreAJour {
                    nom: nom.clone(),
                    de: v.clone(),
                    composant: composant.clone(),
                }),
                Some(_) => None,
            })
            .collect()
    }

    /// Vrai si deux joueurs ont les mêmes versions pour ces composants
    /// (vérifié avant d'ouvrir une session netplay).
    pub fn compatibles(a: &Installees, b: &Installees, composants: &[&str]) -> bool {
        composants.iter().all(|c| a.get(*c) == b.get(*c))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const JSON: &str = r#"{"revision":1,"composants":{
        "retroarch":{"version":"1.21.0","url":"u","sha256":"s"},
        "dolphin":{"version":"2512","url":"u","sha256":"s"}}}"#;

    #[test]
    fn calcule_les_mises_a_jour() {
        let v = Validees::lire(JSON).unwrap();
        let mut installees = Installees::new();
        installees.insert("dolphin".into(), "2509".into());
        let actions = v.a_faire(&installees);
        assert_eq!(actions.len(), 2);
        assert!(matches!(&actions[0], Action::MettreAJour { nom, de, .. } if nom == "dolphin" && de == "2509"));
        assert!(matches!(&actions[1], Action::Installer { nom, .. } if nom == "retroarch"));
    }

    #[test]
    fn compatibilite_netplay() {
        let mut a = Installees::new();
        a.insert("dolphin".into(), "2512".into());
        let mut b = a.clone();
        assert!(Validees::compatibles(&a, &b, &["dolphin"]));
        b.insert("dolphin".into(), "2509".into());
        assert!(!Validees::compatibles(&a, &b, &["dolphin"]));
    }
}
