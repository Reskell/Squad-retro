//! Les manettes et l'assistant de première configuration.
//!
//! Toute manette moderne (Xbox, PlayStation, Switch Pro…) se ramène à la
//! même disposition : quatre boutons en losange, deux gâchettes de chaque
//! côté, la croix, deux sticks. `BoutonPad` nomme ces boutons par leur
//! position (Sud = le bouton du bas : A sur Xbox, Croix sur PlayStation).
//!
//! Chaque console a ses propres boutons (le A de la GameCube, le C-haut de
//! la N64, le Rond de la PlayStation). Un `Mapping` dit, pour une console,
//! quel bouton physique appuie sur quel bouton de la console. L'assistant
//! propose le mapping par défaut, la personne le teste et le corrige.
//!
//! Le moteur écrit ensuite le mapping :
//! - pour RetroArch, en fichier de remappage du core (indépendant du modèle
//!   de manette : RetroArch reconnaît déjà la manette tout seul) ;
//! - pour Dolphin, dans GCPadNew.ini, avec les noms XInput de Dolphin.

use crate::consoles::Console;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Un bouton physique, par sa position sur une manette moderne.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoutonPad {
    Sud,
    Est,
    Ouest,
    Nord,
    L1,
    R1,
    L2,
    R2,
    L3,
    R3,
    Start,
    Select,
    Haut,
    Bas,
    Gauche,
    Droite,
}

impl BoutonPad {
    pub const TOUS: [BoutonPad; 16] = [
        BoutonPad::Sud,
        BoutonPad::Est,
        BoutonPad::Ouest,
        BoutonPad::Nord,
        BoutonPad::L1,
        BoutonPad::R1,
        BoutonPad::L2,
        BoutonPad::R2,
        BoutonPad::L3,
        BoutonPad::R3,
        BoutonPad::Start,
        BoutonPad::Select,
        BoutonPad::Haut,
        BoutonPad::Bas,
        BoutonPad::Gauche,
        BoutonPad::Droite,
    ];

    /// Le numéro RetroPad de RetroArch (B=0, Y=1, Select=2, Start=3, Haut=4,
    /// Bas=5, Gauche=6, Droite=7, A=8, X=9, L=10, R=11, L2=12, R2=13, L3=14, R3=15).
    /// Sur la RetroPad, B est en bas, A à droite, Y à gauche, X en haut.
    pub fn retropad(self) -> u8 {
        match self {
            BoutonPad::Sud => 0,
            BoutonPad::Ouest => 1,
            BoutonPad::Select => 2,
            BoutonPad::Start => 3,
            BoutonPad::Haut => 4,
            BoutonPad::Bas => 5,
            BoutonPad::Gauche => 6,
            BoutonPad::Droite => 7,
            BoutonPad::Est => 8,
            BoutonPad::Nord => 9,
            BoutonPad::L1 => 10,
            BoutonPad::R1 => 11,
            BoutonPad::L2 => 12,
            BoutonPad::R2 => 13,
            BoutonPad::L3 => 14,
            BoutonPad::R3 => 15,
        }
    }

    /// Le nom du bouton dans les fichiers de remappage RetroArch.
    pub fn nom_retroarch(self) -> &'static str {
        match self {
            BoutonPad::Sud => "b",
            BoutonPad::Ouest => "y",
            BoutonPad::Select => "select",
            BoutonPad::Start => "start",
            BoutonPad::Haut => "up",
            BoutonPad::Bas => "down",
            BoutonPad::Gauche => "left",
            BoutonPad::Droite => "right",
            BoutonPad::Est => "a",
            BoutonPad::Nord => "x",
            BoutonPad::L1 => "l",
            BoutonPad::R1 => "r",
            BoutonPad::L2 => "l2",
            BoutonPad::R2 => "r2",
            BoutonPad::L3 => "l3",
            BoutonPad::R3 => "r3",
        }
    }

    /// Le nom du bouton XInput dans les fichiers de Dolphin.
    pub fn nom_dolphin(self) -> &'static str {
        match self {
            BoutonPad::Sud => "Button A",
            BoutonPad::Est => "Button B",
            BoutonPad::Ouest => "Button X",
            BoutonPad::Nord => "Button Y",
            BoutonPad::L1 => "Shoulder L",
            BoutonPad::R1 => "Shoulder R",
            BoutonPad::L2 => "Trigger L",
            BoutonPad::R2 => "Trigger R",
            BoutonPad::L3 => "Thumb L",
            BoutonPad::R3 => "Thumb R",
            BoutonPad::Start => "Start",
            BoutonPad::Select => "Back",
            BoutonPad::Haut => "Pad N",
            BoutonPad::Bas => "Pad S",
            BoutonPad::Gauche => "Pad W",
            BoutonPad::Droite => "Pad E",
        }
    }
}

/// Un bouton de la console, avec ce qu'il envoie et son bouton physique par défaut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct BoutonConsole {
    /// L'identifiant (« a », « c_haut », « rond »…).
    pub id: &'static str,
    /// Le nom affiché dans l'assistant.
    pub nom: &'static str,
    /// Ce que le core reçoit (RetroArch) ou le nom de la touche (Dolphin).
    pub cible: Cible,
    /// Le bouton physique proposé par défaut.
    pub par_defaut: BoutonPad,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "type", content = "valeur", rename_all = "snake_case")]
pub enum Cible {
    /// Un bouton RetroPad (numéro reçu par le core).
    Retropad(u8),
    /// Une touche de manette GameCube dans GCPadNew.ini.
    Dolphin(&'static str),
}

const fn rp(id: &'static str, nom: &'static str, pad: BoutonPad) -> BoutonConsole {
    BoutonConsole {
        id,
        nom,
        cible: Cible::Retropad(pad_num(pad)),
        par_defaut: pad,
    }
}

const fn pad_num(pad: BoutonPad) -> u8 {
    match pad {
        BoutonPad::Sud => 0,
        BoutonPad::Ouest => 1,
        BoutonPad::Select => 2,
        BoutonPad::Start => 3,
        BoutonPad::Haut => 4,
        BoutonPad::Bas => 5,
        BoutonPad::Gauche => 6,
        BoutonPad::Droite => 7,
        BoutonPad::Est => 8,
        BoutonPad::Nord => 9,
        BoutonPad::L1 => 10,
        BoutonPad::R1 => 11,
        BoutonPad::L2 => 12,
        BoutonPad::R2 => 13,
        BoutonPad::L3 => 14,
        BoutonPad::R3 => 15,
    }
}

const fn gc(
    id: &'static str,
    nom: &'static str,
    touche: &'static str,
    pad: BoutonPad,
) -> BoutonConsole {
    BoutonConsole {
        id,
        nom,
        cible: Cible::Dolphin(touche),
        par_defaut: pad,
    }
}

use BoutonPad::*;

const NES: &[BoutonConsole] = &[
    rp("a", "A", Est),
    rp("b", "B", Sud),
    rp("start", "Start", Start),
    rp("select", "Select", Select),
];
const SNES: &[BoutonConsole] = &[
    rp("a", "A", Est),
    rp("b", "B", Sud),
    rp("x", "X", Nord),
    rp("y", "Y", Ouest),
    rp("l", "L", L1),
    rp("r", "R", R1),
    rp("start", "Start", Start),
    rp("select", "Select", Select),
];
/// Mupen64Plus-Next : A = RetroPad B, B = RetroPad Y, Z = L2, C au stick droit.
const N64: &[BoutonConsole] = &[
    rp("a", "A", Sud),
    rp("b", "B", Ouest),
    rp("z", "Z", L2),
    rp("l", "L", L1),
    rp("r", "R", R1),
    rp("start", "Start", Start),
];
const PS1: &[BoutonConsole] = &[
    rp("croix", "Croix", Sud),
    rp("rond", "Rond", Est),
    rp("carre", "Carré", Ouest),
    rp("triangle", "Triangle", Nord),
    rp("l1", "L1", L1),
    rp("r1", "R1", R1),
    rp("l2", "L2", L2),
    rp("r2", "R2", R2),
    rp("start", "Start", Start),
    rp("select", "Select", Select),
];
const GAMECUBE: &[BoutonConsole] = &[
    gc("a", "A (le gros bouton vert)", "Buttons/A", Sud),
    gc("b", "B", "Buttons/B", Ouest),
    gc("x", "X", "Buttons/X", Est),
    gc("y", "Y", "Buttons/Y", Nord),
    gc("z", "Z", "Buttons/Z", R1),
    gc("l", "L (gâchette)", "Triggers/L", L2),
    gc("r", "R (gâchette)", "Triggers/R", R2),
    gc("start", "Start", "Buttons/Start", Start),
];

/// Les boutons d'une console. La Wii simulée reprend la manette GameCube
/// (Classic Controller) ; une vraie Wiimote ne se mappe pas.
pub fn boutons(console: Console) -> &'static [BoutonConsole] {
    match console {
        Console::Nes => NES,
        Console::Snes => SNES,
        Console::N64 => N64,
        Console::Ps1 => PS1,
        Console::Gamecube | Console::Wii => GAMECUBE,
    }
}

/// Le mapping d'une console : bouton de la console → bouton physique.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Mapping(pub BTreeMap<String, BoutonPad>);

impl Mapping {
    pub fn par_defaut(console: Console) -> Mapping {
        Mapping(
            boutons(console)
                .iter()
                .map(|b| (b.id.to_string(), b.par_defaut))
                .collect(),
        )
    }

    /// Le bouton physique d'un bouton de la console.
    pub fn physique(&self, console: Console, id: &str) -> Option<BoutonPad> {
        self.0.get(id).copied().or_else(|| {
            boutons(console)
                .iter()
                .find(|b| b.id == id)
                .map(|b| b.par_defaut)
        })
    }

    /// Les boutons physiques utilisés deux fois (à signaler dans l'assistant).
    pub fn conflits(&self) -> Vec<BoutonPad> {
        let mut vus = BTreeMap::new();
        for p in self.0.values() {
            *vus.entry(*p).or_insert(0) += 1;
        }
        vus.into_iter()
            .filter(|(_, n)| *n > 1)
            .map(|(p, _)| p)
            .collect()
    }

    /// Le fichier de remappage RetroArch (.rmp) du joueur 1.
    pub fn remap_retroarch(&self, console: Console) -> String {
        let mut lignes = vec!["# Écrit par SQUAD//RETRO (assistant de configuration).".to_string()];
        // Par défaut, chaque bouton physique n'envoie rien de spécial : on
        // écrit seulement ceux de la console, les autres gardent leur rôle.
        for b in boutons(console) {
            if let Cible::Retropad(n) = b.cible {
                let p = self.physique(console, b.id).unwrap_or(b.par_defaut);
                lignes.push(format!(
                    r#"input_player1_btn_{} = "{n}""#,
                    p.nom_retroarch()
                ));
            }
        }
        lignes.join("\n") + "\n"
    }

    /// La section [GCPad1] de GCPadNew.ini (Dolphin), manette XInput n° 0.
    pub fn gcpad_dolphin(&self, console: Console) -> String {
        let mut lignes = vec!["[GCPad1]".to_string(), "Device = XInput/0/Gamepad".into()];
        for b in boutons(console) {
            if let Cible::Dolphin(touche) = b.cible {
                let p = self.physique(console, b.id).unwrap_or(b.par_defaut);
                lignes.push(format!("{touche} = `{}`", p.nom_dolphin()));
            }
        }
        lignes.extend([
            "Main Stick/Up = `Left Y+`".into(),
            "Main Stick/Down = `Left Y-`".into(),
            "Main Stick/Left = `Left X-`".into(),
            "Main Stick/Right = `Left X+`".into(),
            "C-Stick/Up = `Right Y+`".into(),
            "C-Stick/Down = `Right Y-`".into(),
            "C-Stick/Left = `Right X-`".into(),
            "C-Stick/Right = `Right X+`".into(),
            "Triggers/L-Analog = `Trigger L`".into(),
            "Triggers/R-Analog = `Trigger R`".into(),
            "D-Pad/Up = `Pad N`".into(),
            "D-Pad/Down = `Pad S`".into(),
            "D-Pad/Left = `Pad W`".into(),
            "D-Pad/Right = `Pad E`".into(),
            "Rumble/Motor = `Motor L` | `Motor R`".into(),
        ]);
        lignes.join("\n") + "\n"
    }
}

/// Une étape de l'assistant de première configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Etape {
    /// Détecter la manette et proposer le mapping par défaut.
    Manette,
    /// Tester les boutons sur la manette dessinée, corriger si besoin.
    Test { boutons: Vec<BoutonConsole> },
    /// Choisir la Wiimote (Wii seulement).
    Wiimote,
    /// Vérifier le BIOS (PS1 seulement).
    Bios,
    /// Choisir le rendu de l'image.
    Image,
    /// Lancer une partie test de 30 secondes.
    PartieTest,
}

/// Les étapes de l'assistant pour une console : seulement celles utiles.
pub fn etapes(console: Console) -> Vec<Etape> {
    let mut e = vec![
        Etape::Manette,
        Etape::Test {
            boutons: boutons(console).to_vec(),
        },
    ];
    if console == Console::Wii {
        e.push(Etape::Wiimote);
    }
    if console.demande_un_bios() {
        e.push(Etape::Bios);
    }
    e.push(Etape::Image);
    e.push(Etape::PartieTest);
    e
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remap_par_defaut_et_corrige() {
        let mut m = Mapping::par_defaut(Console::Snes);
        let rmp = m.remap_retroarch(Console::Snes);
        assert!(rmp.contains(r#"input_player1_btn_a = "8""#));
        // La personne préfère A sur le bouton du bas.
        m.0.insert("a".into(), BoutonPad::Sud);
        m.0.insert("b".into(), BoutonPad::Est);
        let rmp = m.remap_retroarch(Console::Snes);
        assert!(rmp.contains(r#"input_player1_btn_b = "8""#));
        assert!(rmp.contains(r#"input_player1_btn_a = "0""#));
        assert!(m.conflits().is_empty());
        m.0.insert("x".into(), BoutonPad::Sud);
        assert_eq!(m.conflits(), vec![BoutonPad::Sud]);
    }

    #[test]
    fn gcpad_dolphin() {
        let ini = Mapping::par_defaut(Console::Gamecube).gcpad_dolphin(Console::Gamecube);
        assert!(ini.contains("Buttons/A = `Button A`"));
        assert!(ini.contains("Triggers/L = `Trigger L`"));
        assert!(ini.contains("C-Stick/Up"));
    }

    #[test]
    fn etapes_utiles() {
        assert!(etapes(Console::Wii).contains(&Etape::Wiimote));
        assert!(etapes(Console::Ps1).contains(&Etape::Bios));
        assert!(!etapes(Console::Snes).contains(&Etape::Bios));
        assert!(BoutonPad::TOUS.iter().all(|b| b.retropad() == pad_num(*b)));
    }
}
