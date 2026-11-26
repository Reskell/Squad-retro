//! SQUAD//RETRO — le moteur du launcher rétro.
//!
//! Une seule bibliothèque, deux interfaces posées dessus : la section rétro
//! de l'app de bureau SQUAD (squad-desktop) et l'app autonome en mode canapé.
//! Le moteur ne dessine rien : il connaît les consoles, reconnaît les jeux,
//! prépare la commande qui lance l'émulateur et vérifie les versions.
//!
//! Modules :
//! - `consoles` : les six consoles de la V1, leurs extensions et leur moteur ;
//! - `dat` : la lecture des bases No-Intro et Redump (fichiers .dat) ;
//! - `identification` : l'empreinte d'un fichier (CRC32 + SHA-1), sans
//!   l'en-tête iNES pour la NES, comme les bases No-Intro ;
//! - `region` : la région d'un jeu d'après son nom, et l'ordre FR > EU > US ;
//! - `bibliotheque` : le scan d'un dossier par console, une fiche par jeu ;
//! - `lancement`, `retroarch`, `dolphin` : vérifier, configurer, lancer ;
//! - `emplacements` : où tout est installé et écrit ;
//! - `profil`, `manettes` : les choix de l'assistant de configuration ;
//! - `catalogue` : compatibilité et réglages recommandés par jeu ;
//! - `versions`, `installation` : versions validées, installation vérifiée,
//!   retour arrière, intégrité ;
//! - `mods` : les variantes de jeux (patchs IPS/BPS) et leurs mises à jour ;
//! - `netplay` : sessions, room, lien d'invitation, ping ;
//! - `historique` : l'accueil « Reprendre » et le temps joué ;
//! - `sauvegarde` : export et restauration complets ;
//! - `diagnostic` : le rapport en un clic.

pub mod bibliotheque;
pub mod catalogue;
pub mod consoles;
pub mod dat;
pub mod diagnostic;
pub mod dolphin;
pub mod emplacements;
pub mod historique;
pub mod identification;
pub mod installation;
pub mod lancement;
pub mod manettes;
pub mod mods;
pub mod netplay;
pub mod profil;
pub mod region;
pub mod retroarch;
pub mod sauvegarde;
pub mod versions;

pub use bibliotheque::{Bibliotheque, Fiche, Variante};
pub use consoles::{Console, Moteur};
pub use emplacements::Emplacements;
pub use lancement::Commande;
pub use profil::Profil;
pub use region::Region;
