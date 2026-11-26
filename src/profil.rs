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
        }
    }

    /// Le même profil, complété par les réglages recommandés d'un jeu.
    pub fn avec_options(&self, options: &BTreeMap<String, String>) -> Profil {
        let mut p = self.clone();
        p.options
            .extend(options.iter().map(|(k, v)| (k.clone(), v.clone())));
        p
    }
}
