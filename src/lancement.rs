//! La commande qui lance un jeu.
//!
//! RetroArch est lancé avec le bon core, une config préparée par SQUAD et son
//! menu inaccessible : le joueur ne voit que le jeu, en plein écran. Échap
//! quitte et ramène au launcher. Dolphin est lancé en mode « batch » : il
//! démarre le jeu tout de suite et se ferme avec lui.
//!
//! Le moteur prépare la commande sans la lancer : l'app (squad-desktop ou
//! l'app autonome) la lance et suit le processus, comme pour les jeux Steam.

use crate::bibliotheque::Variante;
use crate::consoles::{Console, Moteur};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Où sont installés les émulateurs (gérés par le launcher, pas par le joueur).
#[derive(Debug, Clone)]
pub struct Emplacements {
    /// L'exécutable RetroArch.
    pub retroarch: PathBuf,
    /// Le dossier des cores libretro.
    pub cores: PathBuf,
    /// Le dossier où SQUAD écrit une config RetroArch par console.
    pub configs: PathBuf,
    /// L'exécutable Dolphin.
    pub dolphin: PathBuf,
    /// Le dossier utilisateur de Dolphin, propre à SQUAD (option -u).
    pub dossier_dolphin: PathBuf,
}

impl Emplacements {
    /// Les emplacements déduits de l'exécutable RetroArch : les cores sont
    /// dans son dossier `cores/`, les configs SQUAD dans `donnees/configs`.
    /// Dolphin n'est pas encore géré par la vague 1 : ses chemins pointent
    /// vers `donnees/dolphin` en attendant l'installation automatique.
    pub fn depuis_retroarch(retroarch: &Path, donnees: &Path) -> Emplacements {
        let dossier = retroarch.parent().unwrap_or(Path::new("."));
        Emplacements {
            retroarch: retroarch.to_path_buf(),
            cores: dossier.join("cores"),
            configs: donnees.join("configs"),
            dolphin: donnees.join("dolphin").join(if cfg!(windows) {
                "Dolphin.exe"
            } else {
                "dolphin-emu"
            }),
            dossier_dolphin: donnees.join("dolphin").join("User"),
        }
    }
}

/// Une commande prête à lancer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Commande {
    pub programme: PathBuf,
    pub arguments: Vec<String>,
}

impl Commande {
    /// La commande système correspondante.
    pub fn vers_process(&self) -> Command {
        let mut c = Command::new(&self.programme);
        c.args(&self.arguments);
        c
    }
}

/// L'extension des cores libretro sur ce système.
pub fn extension_core() -> &'static str {
    if cfg!(windows) {
        "dll"
    } else if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    }
}

/// Prépare la commande qui lance cette version d'un jeu.
pub fn commande(console: Console, variante: &Variante, ou: &Emplacements) -> Commande {
    let rom = variante.chemin.to_string_lossy().into_owned();
    match console.moteur() {
        Moteur::Retroarch { core } => {
            let core = ou
                .cores
                .join(format!("{core}_libretro.{}", extension_core()));
            let config = chemin_config(&ou.configs, console);
            Commande {
                programme: ou.retroarch.clone(),
                arguments: vec![
                    "-L".into(),
                    core.to_string_lossy().into_owned(),
                    "--config".into(),
                    config.to_string_lossy().into_owned(),
                    "--fullscreen".into(),
                    rom,
                ],
            }
        }
        Moteur::Dolphin => Commande {
            programme: ou.dolphin.clone(),
            arguments: vec![
                "-u".into(),
                ou.dossier_dolphin.to_string_lossy().into_owned(),
                "-b".into(),
                "-e".into(),
                rom,
            ],
        },
    }
}

/// Vérifie que tout est en place pour lancer une console, avec un message
/// clair sinon.
pub fn verifier(console: Console, ou: &Emplacements) -> Result<(), String> {
    match console.moteur() {
        Moteur::Retroarch { core } => {
            if !ou.retroarch.is_file() {
                return Err(
                    "RetroArch est introuvable : choisis retroarch.exe dans les réglages.".into(),
                );
            }
            let fichier = ou
                .cores
                .join(format!("{core}_libretro.{}", extension_core()));
            if !fichier.is_file() {
                return Err(format!(
                    "Le core « {core} » ({}) manque dans {}. L'installation automatique arrive avec la vague 3 ; \
                     en attendant, installe-le une fois depuis RetroArch (Mise à jour en ligne > Télécharger un core).",
                    console.nom(),
                    ou.cores.display()
                ));
            }
            Ok(())
        }
        Moteur::Dolphin => {
            if ou.dolphin.is_file() {
                Ok(())
            } else {
                Err("Dolphin n'est pas encore installé : la GameCube et la Wii arrivent avec la vague 2.".into())
            }
        }
    }
}

/// Écrit la config SQUAD d'une console avant de la lancer (RetroArch seulement).
pub fn preparer(
    console: Console,
    ou: &Emplacements,
    dossier_sauvegardes: &Path,
) -> std::io::Result<()> {
    if let Moteur::Retroarch { .. } = console.moteur() {
        std::fs::create_dir_all(&ou.configs)?;
        std::fs::create_dir_all(dossier_sauvegardes.join(console.dossier()))?;
        std::fs::write(
            chemin_config(&ou.configs, console),
            config_retroarch(console, dossier_sauvegardes),
        )?;
    }
    Ok(())
}

/// Le fichier de config RetroArch d'une console.
pub fn chemin_config(configs: &Path, console: Console) -> PathBuf {
    configs.join(format!("{}.cfg", console.dossier()))
}

/// La config RetroArch écrite par SQUAD pour une console : plein écran, menu
/// fermé, reprise automatique là où on s'était arrêté, rembobinage sur les
/// consoles 8 et 16 bits.
pub fn config_retroarch(console: Console, dossier_sauvegardes: &Path) -> String {
    let sauvegardes = dossier_sauvegardes.join(console.dossier());
    let sauvegardes = sauvegardes.to_string_lossy();
    let rembobinage = matches!(console, Console::Nes | Console::Snes);
    let mut lignes = vec![
        "# Écrit par SQUAD//RETRO : ne pas modifier, ce fichier est réécrit.".to_string(),
        r#"video_fullscreen = "true""#.into(),
        r#"config_save_on_exit = "false""#.into(),
        r#"pause_nonactive = "false""#.into(),
        r#"quit_press_twice = "false""#.into(),
        r#"input_exit_emulator = "escape""#.into(),
        // Le menu RetroArch n'est jamais ouvert : SQUAD a le sien.
        r#"input_menu_toggle = "nul""#.into(),
        r#"input_menu_toggle_gamepad_combo = "0""#.into(),
        r#"savestate_auto_save = "true""#.into(),
        r#"savestate_auto_load = "true""#.into(),
        format!(r#"savefile_directory = "{sauvegardes}""#),
        format!(r#"savestate_directory = "{sauvegardes}""#),
        r#"notification_show_autoconfig = "false""#.into(),
    ];
    lignes.push(format!(r#"rewind_enable = "{rembobinage}""#));
    lignes.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::region::Region;

    fn emplacements() -> Emplacements {
        Emplacements {
            retroarch: "/emu/retroarch".into(),
            cores: "/emu/cores".into(),
            configs: "/squad/configs".into(),
            dolphin: "/emu/dolphin".into(),
            dossier_dolphin: "/squad/dolphin".into(),
        }
    }

    fn variante(chemin: &str) -> Variante {
        Variante {
            chemin: chemin.into(),
            nom: "Jeu".into(),
            region: Region::Europe,
            reconnu: true,
            empreintes: None,
        }
    }

    #[test]
    fn commande_retroarch() {
        let c = commande(
            Console::Snes,
            &variante("/roms/snes/a.sfc"),
            &emplacements(),
        );
        assert_eq!(c.programme, PathBuf::from("/emu/retroarch"));
        assert_eq!(c.arguments[0], "-L");
        assert!(c.arguments[1].contains("snes9x_libretro."));
        assert_eq!(c.arguments.last().unwrap(), "/roms/snes/a.sfc");
    }

    #[test]
    fn commande_dolphin() {
        let c = commande(
            Console::Gamecube,
            &variante("/roms/gc/mkdd.rvz"),
            &emplacements(),
        );
        assert_eq!(
            c.arguments,
            vec!["-u", "/squad/dolphin", "-b", "-e", "/roms/gc/mkdd.rvz"]
        );
    }

    #[test]
    fn verifie_et_prepare() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("retroarch.exe");
        let ou = Emplacements::depuis_retroarch(&exe, &dir.path().join("donnees"));
        assert!(verifier(Console::Snes, &ou)
            .unwrap_err()
            .contains("RetroArch"));
        std::fs::write(&exe, b"").unwrap();
        assert!(verifier(Console::Snes, &ou).unwrap_err().contains("snes9x"));
        std::fs::create_dir_all(&ou.cores).unwrap();
        std::fs::write(
            ou.cores
                .join(format!("snes9x_libretro.{}", extension_core())),
            b"",
        )
        .unwrap();
        assert!(verifier(Console::Snes, &ou).is_ok());
        preparer(Console::Snes, &ou, &dir.path().join("saves")).unwrap();
        assert!(chemin_config(&ou.configs, Console::Snes).is_file());
    }

    #[test]
    fn config_avec_rembobinage() {
        let snes = config_retroarch(Console::Snes, Path::new("/saves"));
        assert!(snes.contains(r#"rewind_enable = "true""#));
        let n64 = config_retroarch(Console::N64, Path::new("/saves"));
        assert!(n64.contains(r#"rewind_enable = "false""#));
    }
}
