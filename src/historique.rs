//! L'historique de jeu : ce qui alimente l'accueil « Reprendre » (les 5
//! derniers jeux), le temps joué, et le journal du diagnostic.

use crate::consoles::Console;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Partie {
    pub console: Console,
    pub titre: String,
    pub chemin: PathBuf,
    /// Début, en millisecondes depuis 1970.
    pub debut_ms: u64,
    /// Durée en secondes (0 tant que la partie tourne).
    #[serde(default)]
    pub duree_s: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Historique {
    pub parties: Vec<Partie>,
}

/// Au-delà, les plus anciennes parties sont oubliées.
pub const MAX_PARTIES: usize = 500;

impl Historique {
    pub fn charger(fichier: &Path) -> Historique {
        fs::read_to_string(fichier)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn ecrire(&self, fichier: &Path) -> std::io::Result<()> {
        if let Some(d) = fichier.parent() {
            fs::create_dir_all(d)?;
        }
        fs::write(fichier, serde_json::to_string(self).unwrap_or_default())
    }

    pub fn commencer(&mut self, partie: Partie) {
        self.parties.push(partie);
        if self.parties.len() > MAX_PARTIES {
            let trop = self.parties.len() - MAX_PARTIES;
            self.parties.drain(..trop);
        }
    }

    /// Termine la dernière partie de ce fichier.
    pub fn terminer(&mut self, chemin: &Path, fin_ms: u64) {
        if let Some(p) = self
            .parties
            .iter_mut()
            .rev()
            .find(|p| p.chemin == chemin && p.duree_s == 0)
        {
            p.duree_s = fin_ms.saturating_sub(p.debut_ms) / 1000;
        }
    }

    /// Les derniers jeux joués, sans doublon, du plus récent au plus ancien.
    pub fn reprendre(&self, combien: usize) -> Vec<&Partie> {
        let mut vus = std::collections::HashSet::new();
        self.parties
            .iter()
            .rev()
            .filter(|p| vus.insert(p.chemin.clone()))
            .take(combien)
            .collect()
    }

    /// Le temps total joué à un jeu, en secondes.
    pub fn temps(&self, console: Console, titre: &str) -> u64 {
        self.parties
            .iter()
            .filter(|p| p.console == console && p.titre.eq_ignore_ascii_case(titre))
            .map(|p| p.duree_s)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(titre: &str, t: u64) -> Partie {
        Partie {
            console: Console::Snes,
            titre: titre.into(),
            chemin: format!("/{titre}").into(),
            debut_ms: t,
            duree_s: 0,
        }
    }

    #[test]
    fn reprendre_et_temps() {
        let mut h = Historique::default();
        h.commencer(p("A", 0));
        h.terminer(Path::new("/A"), 60_000);
        h.commencer(p("B", 100_000));
        h.commencer(p("A", 200_000));
        h.terminer(Path::new("/A"), 230_000);
        let r: Vec<_> = h.reprendre(5).iter().map(|p| p.titre.clone()).collect();
        assert_eq!(r, vec!["A", "B"]);
        assert_eq!(h.temps(Console::Snes, "a"), 90);
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("h.json");
        h.ecrire(&f).unwrap();
        assert_eq!(Historique::charger(&f).parties.len(), 3);
    }
}
