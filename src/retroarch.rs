//! Tout ce que SQUAD écrit pour RetroArch : la config d'une console, le
//! remappage des boutons, les options du core, et la commande.

use crate::consoles::{Console, Moteur};
use crate::emplacements::Emplacements;
use crate::manettes::Mapping;
use crate::netplay::Session;
use crate::profil::{Image, Profil};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Le nom de la bibliothèque d'un core, tel que RetroArch l'utilise pour
/// ranger les remappages et les options.
pub fn nom_du_core(core: &str) -> &'static str {
    match core {
        "mesen" => "Mesen",
        "snes9x" => "Snes9x",
        "mupen64plus_next" => "Mupen64Plus-Next",
        "swanstation" => "SwanStation",
        _ => "Inconnu",
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

pub fn fichier_core(ou: &Emplacements, core: &str) -> PathBuf {
    ou.cores
        .join(format!("{core}_libretro.{}", extension_core()))
}

pub fn chemin_config(configs: &Path, console: Console) -> PathBuf {
    configs.join(format!("{}.cfg", console.dossier()))
}

/// La config RetroArch d'une console.
pub fn config(
    console: Console,
    ou: &Emplacements,
    profil: &Profil,
    session: Option<&Session>,
) -> String {
    let chemin = |p: PathBuf| p.to_string_lossy().replace('\\', "/");
    let sauvegardes = chemin(ou.sauvegardes().join(console.dossier()));
    let configs = ou.configs();
    let mut l = vec![
        "# Écrit par SQUAD//RETRO : ne pas modifier, ce fichier est réécrit à chaque lancement."
            .to_string(),
        r#"video_fullscreen = "true""#.into(),
        r#"config_save_on_exit = "false""#.into(),
        r#"pause_nonactive = "false""#.into(),
        // Une manette débranchée met le jeu en pause.
        r#"pause_on_disconnect = "true""#.into(),
        r#"quit_press_twice = "false""#.into(),
        r#"input_exit_emulator = "escape""#.into(),
        // L1 + R1 + Start + Select : retour au launcher depuis la manette.
        r#"input_quit_gamepad_combo = "3""#.into(),
        // Le menu rapide (sauver, charger, quitter) : L3 + R3 ou F1, en mode
        // kiosque, sans accès aux réglages de RetroArch.
        r#"input_menu_toggle = "f1""#.into(),
        r#"input_menu_toggle_gamepad_combo = "2""#.into(),
        r#"kiosk_mode_enable = "true""#.into(),
        r#"menu_show_online_updater = "false""#.into(),
        r#"savestate_auto_save = "true""#.into(),
        r#"savestate_auto_load = "true""#.into(),
        r#"savestate_thumbnail_enable = "true""#.into(),
        format!(r#"savefile_directory = "{sauvegardes}""#),
        format!(r#"savestate_directory = "{sauvegardes}""#),
        format!(r#"system_directory = "{}""#, chemin(ou.bios())),
        format!(
            r#"input_remapping_directory = "{}""#,
            chemin(configs.join("remaps"))
        ),
        r#"input_remap_binds_enable = "true""#.into(),
        r#"input_autodetect_enable = "true""#.into(),
        r#"notification_show_autoconfig = "false""#.into(),
        r#"notification_show_remap_load = "false""#.into(),
        format!(r#"rewind_enable = "{}""#, profil.rembobinage),
        r#"global_core_options = "true""#.into(),
        format!(
            r#"core_options_path = "{}""#,
            chemin(configs.join(format!("{}.opt", console.dossier())))
        ),
    ];
    match profil.image {
        Image::PixelNet => {
            l.push(r#"video_smooth = "false""#.into());
            l.push(r#"video_shader_enable = "false""#.into());
        }
        Image::Lisse => {
            l.push(r#"video_smooth = "true""#.into());
            l.push(r#"video_shader_enable = "false""#.into());
        }
        Image::Cathodique => {
            let shader = ou
                .retroarch
                .parent()
                .unwrap_or(Path::new("."))
                .join("shaders/shaders_slang/crt/crt-geom.slangp");
            l.push(r#"video_smooth = "false""#.into());
            l.push(r#"video_shader_enable = "true""#.into());
            l.push(format!(r#"video_shader = "{}""#, chemin(shader)));
        }
    }
    if let Some(s) = session {
        l.extend(s.config_retroarch());
    }
    l.join("\n") + "\n"
}

/// Écrit la config, le remappage et les options du core.
pub fn preparer(
    console: Console,
    ou: &Emplacements,
    profil: &Profil,
    session: Option<&Session>,
) -> io::Result<()> {
    let Moteur::Retroarch { core } = console.moteur() else {
        return Ok(());
    };
    let configs = ou.configs();
    fs::create_dir_all(&configs)?;
    fs::create_dir_all(ou.sauvegardes().join(console.dossier()))?;
    fs::create_dir_all(ou.bios())?;
    fs::write(
        chemin_config(&configs, console),
        config(console, ou, profil, session),
    )?;

    let nom = nom_du_core(core);
    let remaps = configs.join("remaps").join(nom);
    fs::create_dir_all(&remaps)?;
    let mapping = profil
        .mapping
        .clone()
        .unwrap_or_else(|| Mapping::par_defaut(console));
    fs::write(
        remaps.join(format!("{nom}.rmp")),
        mapping.remap_retroarch(console),
    )?;

    let options: String = profil
        .options
        .iter()
        .map(|(k, v)| format!("{k} = \"{}\"\n", v.replace('"', "")))
        .collect();
    fs::write(configs.join(format!("{}.opt", console.dossier())), options)?;
    Ok(())
}

/// La commande qui lance un jeu.
pub fn arguments(
    console: Console,
    rom: &Path,
    ou: &Emplacements,
    session: Option<&Session>,
) -> Vec<String> {
    let Moteur::Retroarch { core } = console.moteur() else {
        return Vec::new();
    };
    let mut a = vec![
        "-L".to_string(),
        fichier_core(ou, core).to_string_lossy().into_owned(),
        "--config".into(),
        chemin_config(&ou.configs(), console)
            .to_string_lossy()
            .into_owned(),
        "--fullscreen".into(),
    ];
    if let Some(s) = session {
        a.extend(s.arguments_retroarch());
    }
    a.push(rom.to_string_lossy().into_owned());
    a
}

/// Les fichiers de BIOS reconnus pour la PlayStation (SwanStation).
pub const BIOS_PS1: [&str; 6] = [
    "scph5502.bin",
    "scph5501.bin",
    "scph5500.bin",
    "scph1001.bin",
    "scph7502.bin",
    "scph101.bin",
];
/// Un BIOS PS1 fait exactement 512 Kio.
pub const TAILLE_BIOS_PS1: u64 = 512 * 1024;

/// Le BIOS PS1 présent et valide, s'il y en a un.
pub fn bios_ps1(ou: &Emplacements) -> Option<PathBuf> {
    BIOS_PS1.iter().map(|n| ou.bios().join(n)).find(|p| {
        fs::metadata(p)
            .map(|m| m.len() == TAILLE_BIOS_PS1)
            .unwrap_or(false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::netplay::Role;

    #[test]
    fn config_et_fichiers() {
        let dir = tempfile::tempdir().unwrap();
        let ou = Emplacements::geres(dir.path());
        let mut profil = Profil::par_defaut(Console::Snes);
        profil.image = Image::Cathodique;
        profil
            .options
            .insert("snes9x_overclock".into(), "on".into());
        let session = Session {
            role: Role::Heberger { port: 55435 },
            pseudo: "Swift".into(),
            mot_de_passe: None,
            relais: None,
        };
        preparer(Console::Snes, &ou, &profil, Some(&session)).unwrap();
        let cfg = fs::read_to_string(chemin_config(&ou.configs(), Console::Snes)).unwrap();
        assert!(cfg.contains(r#"rewind_enable = "true""#));
        assert!(cfg.contains("crt-geom"));
        assert!(cfg.contains("netplay_nickname"));
        assert!(cfg.contains("pause_on_disconnect"));
        assert!(ou.configs().join("remaps/Snes9x/Snes9x.rmp").is_file());
        let opt = fs::read_to_string(ou.configs().join("snes.opt")).unwrap();
        assert!(opt.contains(r#"snes9x_overclock = "on""#));

        let a = arguments(Console::Snes, Path::new("/r/a.sfc"), &ou, Some(&session));
        assert!(a.contains(&"--host".to_string()));
        assert_eq!(a.last().unwrap(), "/r/a.sfc");
    }

    #[test]
    fn bios() {
        let dir = tempfile::tempdir().unwrap();
        let ou = Emplacements::geres(dir.path());
        fs::create_dir_all(ou.bios()).unwrap();
        fs::write(ou.bios().join("scph5502.bin"), vec![0u8; 100]).unwrap();
        assert!(bios_ps1(&ou).is_none());
        fs::write(
            ou.bios().join("scph5502.bin"),
            vec![0u8; TAILLE_BIOS_PS1 as usize],
        )
        .unwrap();
        assert!(bios_ps1(&ou).is_some());
    }
}
