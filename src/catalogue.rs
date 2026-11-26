//! Le catalogue des jeux : ce que SQUAD sait de chaque jeu, au-delà du fichier.
//!
//! Hébergé sur GitHub comme les versions validées (`catalogue.json`), il dit
//! pour chaque jeu :
//! - s'il tourne bien (parfait, bugs connus, injouable) et pourquoi ;
//! - les réglages recommandés, appliqués tout seuls au lancement ;
//! - le nombre de joueurs et si le netplay marche (filtres, jeu au hasard).
//!
//! ```json
//! { "revision": 1, "jeux": [
//!   { "console": "snes", "titre": "Super Mario World", "statut": "parfait",
//!     "joueurs": 2, "netplay": true, "options": {} } ] }
//! ```

use crate::consoles::Console;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Statut {
    Parfait,
    BugsConnus,
    Injouable,
    #[default]
    NonTeste,
}

impl Statut {
    pub fn texte(self) -> &'static str {
        match self {
            Statut::Parfait => "Tourne parfaitement",
            Statut::BugsConnus => "Jouable, bugs connus",
            Statut::Injouable => "Injouable pour l'instant",
            Statut::NonTeste => "Pas encore testé",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InfoJeu {
    pub console: Console,
    pub titre: String,
    #[serde(default)]
    pub statut: Statut,
    #[serde(default)]
    pub notes: String,
    /// Le nombre de joueurs maximum en local.
    #[serde(default = "un")]
    pub joueurs: u8,
    /// Vrai si le netplay a été testé et marche.
    #[serde(default)]
    pub netplay: bool,
    /// Les options de core ou réglages recommandés pour ce jeu.
    #[serde(default)]
    pub options: BTreeMap<String, String>,
}

fn un() -> u8 {
    1
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Catalogue {
    pub revision: u32,
    pub jeux: Vec<InfoJeu>,
    #[serde(skip)]
    index: HashMap<(Console, String), usize>,
}

impl Catalogue {
    pub fn lire(json: &str) -> Result<Catalogue, String> {
        let mut c: Catalogue =
            serde_json::from_str(json).map_err(|e| format!("catalogue.json illisible : {e}"))?;
        c.indexer();
        Ok(c)
    }

    fn indexer(&mut self) {
        self.index = self
            .jeux
            .iter()
            .enumerate()
            .map(|(i, j)| ((j.console, j.titre.to_lowercase()), i))
            .collect();
    }

    pub fn info(&self, console: Console, titre: &str) -> Option<&InfoJeu> {
        self.index
            .get(&(console, titre.to_lowercase()))
            .map(|i| &self.jeux[*i])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lit_et_trouve() {
        let c = Catalogue::lire(
            r#"{"revision":1,"jeux":[{"console":"snes","titre":"Super Mario World","statut":"parfait","joueurs":2,"netplay":true,"options":{"a":"b"}}]}"#,
        )
        .unwrap();
        let info = c.info(Console::Snes, "super mario world").unwrap();
        assert_eq!(info.statut, Statut::Parfait);
        assert_eq!(info.joueurs, 2);
        assert!(c.info(Console::Nes, "Super Mario World").is_none());
    }
}

#[cfg(test)]
mod tests_fichier {
    #[test]
    fn le_catalogue_publie_se_lit() {
        let texte = include_str!("../manifestes/catalogue.json");
        assert!(super::Catalogue::lire(texte).unwrap().jeux.len() >= 5);
        crate::mods::CatalogueMods::lire(include_str!("../manifestes/mods.json")).unwrap();
    }
}
