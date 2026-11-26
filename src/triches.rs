//! Codes de triche et options de mods : des codes Gecko ou Action Replay
//! pour Dolphin (GameCube, Wii), des codes RetroArch pour les autres
//! consoles. Le moteur les vérifie et écrit les fichiers que l'émulateur lit
//! au lancement ; il n'en livre aucun.

use crate::Console;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FormatCode {
    /// Gecko (Dolphin) : lignes « XXXXXXXX YYYYYYYY ».
    #[default]
    Gecko,
    /// Action Replay (Dolphin), déchiffré : même forme de lignes.
    ActionReplay,
    /// RetroArch (Game Genie, Pro Action Replay…), tel que le core l'accepte.
    Retroarch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Code {
    pub nom: String,
    #[serde(default)]
    pub format: FormatCode,
    pub lignes: Vec<String>,
    #[serde(default)]
    pub actif: bool,
}

impl Code {
    /// Nettoie et vérifie un code saisi à la main.
    pub fn verifier(mut self, console: Console) -> Result<Code, String> {
        self.nom = self.nom.trim().chars().filter(|c| !c.is_control() && !"[]$".contains(*c)).take(60).collect();
        if self.nom.is_empty() {
            return Err("Donne un nom au code.".into());
        }
        let dolphin = matches!(console, Console::Gamecube | Console::Wii);
        if dolphin && self.format == FormatCode::Retroarch {
            self.format = FormatCode::Gecko;
        }
        if !dolphin {
            self.format = FormatCode::Retroarch;
        }
        self.lignes = self
            .lignes
            .iter()
            .flat_map(|l| l.lines())
            .map(|l| l.trim().to_uppercase())
            .filter(|l| !l.is_empty())
            .collect();
        if self.lignes.is_empty() {
            return Err("Le code est vide.".into());
        }
        for l in &self.lignes {
            let ok = if dolphin {
                let parties: Vec<&str> = l.split_whitespace().collect();
                parties.len() == 2 && parties.iter().all(|p| p.len() == 8 && p.chars().all(|c| c.is_ascii_hexdigit()))
            } else {
                l.len() <= 40 && l.chars().all(|c| c.is_ascii_alphanumeric() || "-:+ ".contains(c))
            };
            if !ok {
                return Err(format!(
                    "Ligne refusée : « {l} ». {}",
                    if dolphin { "Format attendu : 8 chiffres hexadécimaux, un espace, 8 chiffres." } else { "Lettres, chiffres, « - », « : » et « + » seulement." }
                ));
            }
        }
        Ok(self)
    }
}

/// Le fichier de réglages de jeu de Dolphin (`GameSettings/<ID>.ini`) avec
/// les codes actifs.
pub fn ini_dolphin(codes: &[Code]) -> String {
    let actifs: Vec<&Code> = codes.iter().filter(|c| c.actif).collect();
    let mut ini = String::from("# Écrit par SQUAD//RETRO à chaque lancement.\n");
    for (format, section) in [(FormatCode::Gecko, "Gecko"), (FormatCode::ActionReplay, "ActionReplay")] {
        let liste: Vec<&&Code> = actifs.iter().filter(|c| c.format == format).collect();
        if liste.is_empty() {
            continue;
        }
        ini.push_str(&format!("[{section}]\n"));
        for c in &liste {
            ini.push_str(&format!("${}\n", c.nom));
            for l in &c.lignes {
                ini.push_str(&format!("{l}\n"));
            }
        }
        ini.push_str(&format!("[{section}_Enabled]\n"));
        for c in &liste {
            ini.push_str(&format!("${}\n", c.nom));
        }
    }
    ini
}

/// Un fichier de triches RetroArch (`.cht`).
pub fn cht_retroarch(codes: &[Code]) -> String {
    let mut t = format!("cheats = {}\n", codes.len());
    for (i, c) in codes.iter().enumerate() {
        t.push_str(&format!(
            "cheat{i}_desc = \"{}\"\ncheat{i}_code = \"{}\"\ncheat{i}_enable = {}\n",
            c.nom.replace('"', ""),
            c.lignes.join("+"),
            c.actif
        ));
    }
    t
}

/// Le dossier où RetroArch cherche les triches d'un core (nom du core).
pub fn nom_core_retroarch(console: Console) -> &'static str {
    match console {
        Console::Nes => "Mesen",
        Console::Snes => "Snes9x",
        Console::N64 => "Mupen64Plus-Next",
        Console::Ps1 => "SwanStation",
        Console::Gamecube | Console::Wii => "",
    }
}

/// Une option du menu standard d'un mod : un nom, des choix, et pour chaque
/// choix les codes à activer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OptionMod {
    pub id: String,
    pub nom: String,
    pub choix: Vec<ChoixOption>,
    #[serde(default)]
    pub defaut: usize,
    #[serde(default)]
    pub aide: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChoixOption {
    pub nom: String,
    #[serde(default)]
    pub codes: Vec<Code>,
}

/// Les codes à activer d'après les choix faits dans le menu d'un mod.
pub fn codes_des_options(options: &[OptionMod], choix: &std::collections::BTreeMap<String, usize>) -> Vec<Code> {
    options
        .iter()
        .flat_map(|o| {
            let i = choix.get(&o.id).copied().unwrap_or(o.defaut);
            o.choix.get(i).or_else(|| o.choix.get(o.defaut)).map(|c| c.codes.clone()).unwrap_or_default()
        })
        .map(|c| Code { actif: true, ..c })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn code(nom: &str, lignes: &[&str], format: FormatCode) -> Code {
        Code { nom: nom.into(), format, lignes: lignes.iter().map(|s| s.to_string()).collect(), actif: true }
    }

    #[test]
    fn verifie_les_codes() {
        let c = code(" Vies [infinies] ", &["0412ab00 00000063\n", ""], FormatCode::Retroarch).verifier(Console::Gamecube).unwrap();
        assert_eq!(c.nom, "Vies infinies");
        assert_eq!(c.format, FormatCode::Gecko);
        assert_eq!(c.lignes, vec!["0412AB00 00000063"]);
        assert!(code("x", &["zz"], FormatCode::Gecko).verifier(Console::Gamecube).is_err());
        let gg = code("Saut", &["SXIOPO"], FormatCode::Gecko).verifier(Console::Nes).unwrap();
        assert_eq!(gg.format, FormatCode::Retroarch);
        assert!(code("x", &["rm -rf /"], FormatCode::Retroarch).verifier(Console::Snes).is_err());
    }

    #[test]
    fn ecrit_les_fichiers() {
        let mut inactif = code("Off", &["00000000 00000000"], FormatCode::Gecko);
        inactif.actif = false;
        let ini = ini_dolphin(&[code("60 FPS", &["04000000 00000001"], FormatCode::Gecko), code("AR", &["01234567 89ABCDEF"], FormatCode::ActionReplay), inactif]);
        assert!(ini.contains("[Gecko]\n$60 FPS\n04000000 00000001\n[Gecko_Enabled]\n$60 FPS\n"));
        assert!(ini.contains("[ActionReplay_Enabled]\n$AR"));
        assert!(!ini.contains("Off"));
        let cht = cht_retroarch(&[code("Saut", &["SXIOPO", "AAAA"], FormatCode::Retroarch)]);
        assert!(cht.contains("cheats = 1") && cht.contains("cheat0_code = \"SXIOPO+AAAA\"") && cht.contains("cheat0_enable = true"));
    }

    #[test]
    fn options_de_mod() {
        let options = vec![OptionMod {
            id: "vue".into(),
            nom: "Vue".into(),
            defaut: 0,
            aide: String::new(),
            choix: vec![
                ChoixOption { nom: "Écran partagé".into(), codes: vec![] },
                ChoixOption { nom: "Plein écran".into(), codes: vec![Code { actif: false, ..code("Plein", &["04000000 00000002"], FormatCode::Gecko) }] },
            ],
        }];
        assert!(codes_des_options(&options, &BTreeMap::new()).is_empty());
        let mut choix = BTreeMap::new();
        choix.insert("vue".to_string(), 1);
        let c = codes_des_options(&options, &choix);
        assert_eq!(c.len(), 1);
        assert!(c[0].actif);
        choix.insert("vue".to_string(), 9);
        assert!(codes_des_options(&options, &choix).is_empty());
    }
}
