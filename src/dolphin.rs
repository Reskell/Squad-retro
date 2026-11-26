//! Tout ce que SQUAD écrit pour Dolphin (GameCube et Wii), dans son dossier
//! utilisateur à lui (option -u) : la config de Dolphin ne touche jamais une
//! installation que la personne aurait déjà.

use crate::consoles::Console;
use crate::emplacements::Emplacements;
use crate::manettes::Mapping;
use crate::netplay::Session;
use crate::profil::{Image, Profil, Wiimote};
use std::fs;
use std::io;
use std::path::Path;

/// Dolphin.ini : plein écran, pas de confirmation pour quitter, pas de
/// statistiques envoyées, pas de mise à jour par Dolphin lui-même (c'est
/// SQUAD qui choisit la version).
pub fn dolphin_ini(session: Option<&Session>) -> String {
    dolphin_ini_avec(session, 100)
}

/// Dolphin.ini avec le volume du jeu (0 à 100).
pub fn dolphin_ini_avec(session: Option<&Session>, volume: u8) -> String {
    let mut ini = String::from(
        "[Interface]\nConfirmStop = False\nUsePanicHandlers = False\nOnScreenDisplayMessages = True\nShowActiveTitle = False\n\
         [Display]\nFullscreen = True\nRenderToMain = False\n\
         [Core]\nSIDevice0 = 6\nSIDevice1 = 6\nSIDevice2 = 6\nSIDevice3 = 6\nWiiKeyboard = False\n\
         [Analytics]\nEnabled = False\nPermissionAsked = True\n\
         [AutoUpdate]\nUpdateTrack = \n",
    );
    ini.push_str(&format!("[DSP]\nVolume = {}\n", volume.min(100)));
    if let Some(s) = session {
        ini.push_str(&s.netplay_dolphin());
    }
    ini
}

/// GFX.ini : l'écran large, la précompilation des shaders (pas de saccade
/// au premier passage), et le lissage.
pub fn gfx_ini(profil: &Profil) -> String {
    let mut ini = String::from("[Settings]\n");
    if profil.ecran_large {
        ini.push_str("wideScreenHack = True\nAspectRatio = 1\n");
    } else {
        ini.push_str("wideScreenHack = False\nAspectRatio = 0\n");
    }
    // 2 = « ubershaders asynchrones » : aucune saccade, au prix d'un peu de GPU.
    ini.push_str("ShaderCompilationMode = 2\nWaitForShadersBeforeStarting = True\n");
    ini.push_str("[Enhancements]\n");
    ini.push_str(match profil.image {
        Image::Lisse => "ForceTextureFiltering = True\n",
        _ => "ForceTextureFiltering = False\n",
    });
    ini
}

/// Hotkeys.ini : Échap arrête le jeu (Dolphin se ferme alors, mode batch).
pub fn hotkeys_ini() -> String {
    "[Hotkeys]\nDevice = DInput/0/Keyboard Mouse\nGeneral/Stop = ESCAPE\nGeneral/Toggle Fullscreen = `Alt` & RETURN\n".into()
}

/// WiimoteNew.ini : une vraie Wiimote, ou un Classic Controller simulé sur
/// la manette.
pub fn wiimote_ini(profil: &Profil) -> String {
    match profil.wiimote {
        Wiimote::Vraie => "[Wiimote1]\nSource = 2\n[BalanceBoard]\nSource = 0\n".into(),
        Wiimote::Simulee => {
            [
                "[Wiimote1]",
                "Source = 1",
                "Device = XInput/0/Gamepad",
                "Extension = Classic",
                "Classic/Buttons/A = `Button B`",
                "Classic/Buttons/B = `Button A`",
                "Classic/Buttons/X = `Button Y`",
                "Classic/Buttons/Y = `Button X`",
                "Classic/Buttons/ZL = `Shoulder L`",
                "Classic/Buttons/ZR = `Shoulder R`",
                "Classic/Buttons/- = `Back`",
                "Classic/Buttons/+ = `Start`",
                "Classic/Buttons/Home = `Thumb R`",
                "Classic/Left Stick/Up = `Left Y+`",
                "Classic/Left Stick/Down = `Left Y-`",
                "Classic/Left Stick/Left = `Left X-`",
                "Classic/Left Stick/Right = `Left X+`",
                "Classic/Right Stick/Up = `Right Y+`",
                "Classic/Right Stick/Down = `Right Y-`",
                "Classic/Right Stick/Left = `Right X-`",
                "Classic/Right Stick/Right = `Right X+`",
                "Classic/Triggers/L = `Trigger L`",
                "Classic/Triggers/R = `Trigger R`",
                "Classic/D-Pad/Up = `Pad N`",
                "Classic/D-Pad/Down = `Pad S`",
                "Classic/D-Pad/Left = `Pad W`",
                "Classic/D-Pad/Right = `Pad E`",
                "Buttons/A = `Button A`",
                "Buttons/B = `Trigger R`",
                "Buttons/Home = `Thumb R`",
            ]
            .join("\n")
                + "\n"
        }
    }
}

/// Écrit tous les fichiers de config de Dolphin.
pub fn preparer(
    console: Console,
    ou: &Emplacements,
    profil: &Profil,
    session: Option<&Session>,
) -> io::Result<()> {
    let config = ou.dossier_dolphin().join("Config");
    fs::create_dir_all(&config)?;
    fs::write(config.join("Dolphin.ini"), dolphin_ini_avec(session, profil.volume))?;
    fs::write(config.join("GFX.ini"), gfx_ini(profil))?;
    fs::write(config.join("Hotkeys.ini"), hotkeys_ini())?;
    let mapping = profil
        .mapping
        .clone()
        .unwrap_or_else(|| Mapping::par_defaut(console));
    fs::write(config.join("GCPadNew.ini"), mapping.gcpad_dolphin_avec(console, profil.vibration))?;
    if console == Console::Wii {
        fs::write(config.join("WiimoteNew.ini"), wiimote_ini(profil))?;
    }
    Ok(())
}

/// Les arguments de Dolphin. En solo : mode batch (le jeu démarre tout de
/// suite, Dolphin se ferme avec lui). En netplay : Dolphin s'ouvre sur sa
/// fenêtre principale, la session NetPlay pré-remplie.
pub fn arguments(rom: &Path, ou: &Emplacements, session: Option<&Session>) -> Vec<String> {
    let mut a = vec![
        "-u".to_string(),
        ou.dossier_dolphin().to_string_lossy().into_owned(),
    ];
    if session.is_none() {
        a.extend([
            "-b".to_string(),
            "-e".into(),
            rom.to_string_lossy().into_owned(),
        ]);
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fichiers_wii() {
        let dir = tempfile::tempdir().unwrap();
        let ou = Emplacements::geres(dir.path());
        let profil = Profil::par_defaut(Console::Wii);
        preparer(Console::Wii, &ou, &profil, None).unwrap();
        let config = ou.dossier_dolphin().join("Config");
        let gfx = fs::read_to_string(config.join("GFX.ini")).unwrap();
        assert!(gfx.contains("wideScreenHack = True"));
        assert!(gfx.contains("WaitForShadersBeforeStarting = True"));
        assert!(fs::read_to_string(config.join("WiimoteNew.ini"))
            .unwrap()
            .contains("Extension = Classic"));
        assert!(config.join("GCPadNew.ini").is_file());
        let a = arguments(Path::new("/r/mkdd.rvz"), &ou, None);
        assert_eq!(a[2..], ["-b", "-e", "/r/mkdd.rvz"]);
    }
}
