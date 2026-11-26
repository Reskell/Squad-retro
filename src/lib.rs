//! SQUAD//RETRO — le moteur du launcher rétro.
//!
//! Une seule bibliothèque, deux interfaces posées dessus : la section rétro
//! de l'app de bureau SQUAD (squad-desktop) et l'app autonome en mode canapé.
//! Le moteur ne dessine rien : il connaît les consoles, reconnaît les jeux,
//! prépare la commande qui lance l'émulateur et vérifie les versions.
//!
//! Vague 1 (ce qui est là) :
//! - `consoles` : les six consoles de la V1, leurs extensions et leur moteur ;
//! - `dat` : la lecture des bases No-Intro et Redump (fichiers .dat) ;
//! - `identification` : l'empreinte d'un fichier (CRC32 + SHA-1), sans
//!   l'en-tête iNES pour la NES, comme les bases No-Intro ;
//! - `region` : la région d'un jeu d'après son nom, et l'ordre FR > EU > US ;
//! - `bibliotheque` : le scan d'un dossier par console, une fiche par jeu ;
//! - `lancement` : la commande qui lance RetroArch (caché) ou Dolphin ;
//! - `versions` : le fichier des versions validées des émulateurs.

pub mod bibliotheque;
pub mod consoles;
pub mod dat;
pub mod identification;
pub mod lancement;
pub mod region;
pub mod versions;

pub use bibliotheque::{Bibliotheque, Fiche, Variante};
pub use consoles::{Console, Moteur};
pub use lancement::{Commande, Emplacements};
pub use region::Region;
