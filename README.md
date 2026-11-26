# SQUAD//RETRO — le moteur

Le moteur du launcher rétro. Il sert à deux interfaces :

- la fenêtre SQUAD//RETRO de l'app de bureau SQUAD ([squad-desktop](https://github.com/Reskell/squad-desktop)) ;
- à terme, une app autonome et publique, alternative simple à RetroArch.

Le moteur ne dessine rien. Il connaît les consoles, reconnaît les jeux, écrit
les configs des émulateurs, prépare les commandes, installe et met à jour les
émulateurs, et tient la room de netplay.

## Modules

| Module | Rôle |
| --- | --- |
| `consoles` | NES, SNES, N64, PS1, GameCube, Wii : extensions, moteur, BIOS |
| `dat`, `identification` | Bases No-Intro / Redump, empreintes CRC32 + SHA-1 |
| `region`, `bibliotheque` | Région (FR > EU > US), scan, une fiche par jeu, doublons, recherche, jeu au hasard |
| `catalogue` | Compatibilité, réglages recommandés, nombre de joueurs, netplay testé |
| `emplacements` | Où tout est installé et écrit |
| `lancement`, `retroarch`, `dolphin` | Vérifier ce qui manque, écrire les configs, lancer, compresser (RVZ/CHD) |
| `profil`, `manettes` | Choix de l'assistant : image, rembobinage, 16:9, Wiimote, mapping |
| `versions`, `installation` | Versions validées (stable/bêta), installation vérifiée, retour arrière, intégrité |
| `mods` | Variantes (patchs IPS/BPS), mises à jour, retour arrière |
| `netplay`, `room_reseau` | Sessions RetroArch/Dolphin, room (prêt, compte à rebours), lien d'invitation, ping |
| `historique` | Accueil « Reprendre », temps joué |
| `sauvegarde`, `diagnostic` | Export/restauration complets, rapport en un clic |

## Les versions des émulateurs

1. Modifier `manifestes/sources.json`.
2. L'action **Préparer la bêta** télécharge tout, recopie les cores RetroArch
   dans une version GitHub (adresse fixe), écrit `manifestes/versions-beta.json`,
   puis installe tout pour de vrai sur Windows pour vérifier.
3. Tester avec l'app en canal bêta.
4. L'action **Valider pour tout le monde** copie la bêta en stable.

## Organisation des jeux

```text
roms/
  nes/  snes/  n64/  ps1/  gamecube/  wii/   (dats/ pour les bases No-Intro, facultatif)
```

Uniquement tes propres copies de tes jeux. Le launcher ne télécharge aucun jeu.

## Tester

```bash
cd squad-retro
cargo test
```
