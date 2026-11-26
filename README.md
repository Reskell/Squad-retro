# SQUAD//RETRO — le moteur

Le moteur du launcher rétro de la bande. Il sert à deux interfaces :

- la section rétro de l'app de bureau SQUAD ([squad-desktop](https://github.com/Reskell/squad-desktop)) ;
- l'app autonome SQUAD//RETRO, en mode canapé à la manette (à venir).

Le moteur ne dessine rien. Il connaît les consoles, reconnaît les jeux, prépare
la commande qui lance l'émulateur et vérifie les versions des émulateurs.

## Ce qui est fait (vague 1)

| Module | Rôle |
| --- | --- |
| `consoles` | NES, SNES, N64, PS1, GameCube, Wii : extensions, moteur, BIOS |
| `dat` | Lecture des bases No-Intro / Redump (.dat) |
| `identification` | Empreinte CRC32 + SHA-1 (sans en-tête iNES, N64 remise en ordre z64) |
| `region` | Région d'un jeu d'après son nom, ordre FR > EU > US |
| `bibliotheque` | Scan de `roms/<console>/`, une fiche par jeu, version préférée |
| `lancement` | Commande RetroArch (menu fermé, plein écran) ou Dolphin (batch) |
| `versions` | Fichier des versions validées, mises à jour et compatibilité netplay |

## Organisation des jeux

```text
roms/
  nes/  snes/  n64/  ps1/  gamecube/  wii/
```

Uniquement tes propres copies de tes jeux (dumps de tes cartouches et disques).
Le launcher ne télécharge aucun jeu.

## Tester

```bash
cd squad-retro
cargo test
```
