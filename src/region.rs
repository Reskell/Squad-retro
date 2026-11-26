//! La région d'un jeu, lue dans son nom No-Intro ou Redump.
//!
//! Les bases nomment les jeux ainsi : « Super Mario World (USA) »,
//! « Zelda no Densetsu (Japan) », « Kirby (Europe) (En,Fr,De) ».
//! On lit la première parenthèse qui contient une région connue.

use serde::{Deserialize, Serialize};

/// Une région, de la plus préférée à la moins préférée par défaut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Region {
    France,
    Europe,
    Usa,
    Japon,
    Monde,
    Autre,
}

/// L'ordre par défaut : FR, puis EU, puis US, puis le reste.
pub const ORDRE_PAR_DEFAUT: [Region; 6] = [
    Region::France,
    Region::Europe,
    Region::Monde,
    Region::Usa,
    Region::Japon,
    Region::Autre,
];

impl Region {
    /// Lit la région dans un nom de jeu. Un jeu sans région connue est « Autre ».
    pub fn depuis_nom(nom: &str) -> Region {
        for groupe in parentheses(nom) {
            let mut trouvees: Vec<Region> = groupe
                .split(',')
                .filter_map(|mot| Region::depuis_mot(mot.trim()))
                .collect();
            if trouvees.is_empty() {
                continue;
            }
            // « (USA, Europe) » : on garde la plus préférée des deux.
            trouvees.sort_by_key(|r| rang(*r, &ORDRE_PAR_DEFAUT));
            return trouvees[0];
        }
        Region::Autre
    }

    fn depuis_mot(mot: &str) -> Option<Region> {
        Some(match mot {
            "France" => Region::France,
            "Europe" | "Germany" | "Spain" | "Italy" | "UK" => Region::Europe,
            "USA" | "Canada" => Region::Usa,
            "Japan" => Region::Japon,
            "World" => Region::Monde,
            _ => return None,
        })
    }
}

/// La place d'une région dans un ordre de préférence (0 = la meilleure).
pub fn rang(region: Region, ordre: &[Region]) -> usize {
    ordre.iter().position(|r| *r == region).unwrap_or(ordre.len())
}

/// Le titre d'un jeu sans ses parenthèses : « Super Mario World (USA) (Rev 1) »
/// devient « Super Mario World ». Deux versions du même jeu ont le même titre.
pub fn titre(nom: &str) -> String {
    let mut sortie = String::new();
    let mut profondeur = 0u32;
    for c in nom.chars() {
        match c {
            '(' | '[' => profondeur += 1,
            ')' | ']' => profondeur = profondeur.saturating_sub(1),
            _ if profondeur == 0 => sortie.push(c),
            _ => {}
        }
    }
    sortie.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn parentheses(nom: &str) -> Vec<&str> {
    let mut groupes = Vec::new();
    let mut reste = nom;
    while let Some(debut) = reste.find('(') {
        let apres = &reste[debut + 1..];
        match apres.find(')') {
            Some(fin) => {
                groupes.push(&apres[..fin]);
                reste = &apres[fin + 1..];
            }
            None => break,
        }
    }
    groupes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lit_les_regions() {
        assert_eq!(Region::depuis_nom("Super Mario World (USA)"), Region::Usa);
        assert_eq!(Region::depuis_nom("Kirby (Europe) (En,Fr,De)"), Region::Europe);
        assert_eq!(Region::depuis_nom("Tintin (France)"), Region::France);
        assert_eq!(Region::depuis_nom("Tetris (Japan, USA)"), Region::Usa);
        assert_eq!(Region::depuis_nom("Homebrew sans region"), Region::Autre);
    }

    #[test]
    fn titres() {
        assert_eq!(titre("Super Mario World (USA) (Rev 1)"), "Super Mario World");
        assert_eq!(titre("Zelda [!] (Europe)"), "Zelda");
    }
}
