//! Les réglages d'un lancement : ce que l'assistant de première
//! configuration a choisi pour une console, plus ce que le catalogue
//! recommande pour un jeu précis.

use crate::consoles::Console;
use crate::manettes::Mapping;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Le rendu de l'image, choisi dans l'assistant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Image {
    /// Pixels nets, sans filtre.
    #[default]
    PixelNet,
    /// Effet écran cathodique.
    Cathodique,
    /// Image lissée.
    Lisse,
}

/// La Wiimote utilisée pour la Wii.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Wiimote {
    /// Simulée sur une manette classique.
    #[default]
    Simulee,
    /// Une vraie Wiimote (Bluetooth ou DolphinBar).
    Vraie,
}

/// Les réglages d'une console (écrits par l'assistant).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profil {
    pub console: Console,
    #[serde(default)]
    pub image: Image,
    /// Rembobinage (par défaut sur NES et SNES).
    pub rembobinage: bool,
    /// Image en 16:9 quand le jeu le permet (GameCube, Wii).
    #[serde(default = "vrai")]
    pub ecran_large: bool,
    #[serde(default)]
    pub wiimote: Wiimote,
    /// Le mapping choisi pour le joueur 1 (None = mapping par défaut).
    #[serde(default)]
    pub mapping: Option<Mapping>,
    /// Vrai une fois l'assistant terminé pour cette console.
    #[serde(default)]
    pub configure: bool,
    /// Options de core en plus (venues du catalogue pour un jeu précis).
    #[serde(default)]
    pub options: BTreeMap<String, String>,
    /// Volume du jeu, de 0 à 100.
    #[serde(default = "cent")]
    pub volume: u8,
    /// Réduction de latence (run-ahead), pour NES et SNES.
    #[serde(default)]
    pub run_ahead: bool,
    /// Vibration des manettes.
    #[serde(default = "vrai")]
    pub vibration: bool,
}

fn cent() -> u8 {
    100
}

/// Les réglages propres à un jeu : chaque champ absent garde celui de la console.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReglagesJeu {
    pub image: Option<Image>,
    pub rembobinage: Option<bool>,
    pub ecran_large: Option<bool>,
    pub volume: Option<u8>,
    pub run_ahead: Option<bool>,
    pub vibration: Option<bool>,
}

fn vrai() -> bool {
    true
}

impl Profil {
    /// Le profil par défaut d'une console, avant l'assistant.
    pub fn par_defaut(console: Console) -> Profil {
        Profil {
            console,
            image: Image::PixelNet,
            rembobinage: matches!(console, Console::Nes | Console::Snes),
            ecran_large: true,
            wiimote: Wiimote::Simulee,
            mapping: None,
            configure: false,
            options: BTreeMap::new(),
            volume: 100,
            run_ahead: false,
            vibration: true,
        }
    }

    /// Le profil de la console, avec les réglages propres à un jeu par-dessus.
    pub fn avec_reglages(&self, r: &ReglagesJeu) -> Profil {
        let mut p = self.clone();
        if let Some(v) = r.image {
            p.image = v;
        }
        if let Some(v) = r.rembobinage {
            p.rembobinage = v;
        }
        if let Some(v) = r.ecran_large {
            p.ecran_large = v;
        }
        if let Some(v) = r.volume {
            p.volume = v.min(100);
        }
        if let Some(v) = r.run_ahead {
            p.run_ahead = v;
        }
        if let Some(v) = r.vibration {
            p.vibration = v;
        }
        p
    }

    /// Le même profil, complété par les réglages recommandés d'un jeu.
    pub fn avec_options(&self, options: &BTreeMap<String, String>) -> Profil {
        let mut p = self.clone();
        p.options
            .extend(options.iter().map(|(k, v)| (k.clone(), v.clone())));
        p
    }
}

#[cfg(test)]
mod tests_reglages_jeu {
    use super::*;

    #[test]
    fn reglages_du_jeu_par_dessus_la_console() {
        let base = Profil::par_defaut(Console::Snes);
        let r = ReglagesJeu { image: Some(Image::Cathodique), volume: Some(140), run_ahead: Some(true), ..Default::default() };
        let p = base.avec_reglages(&r);
        assert_eq!(p.image, Image::Cathodique);
        assert_eq!(p.volume, 100);
        assert!(p.run_ahead && p.rembobinage);
        assert_eq!(base.avec_reglages(&ReglagesJeu::default()), base);
    }
}
