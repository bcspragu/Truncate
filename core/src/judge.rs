use serde::{Deserialize, Serialize};

use crate::reporting::{BattleReport, BattleWord, Change};

use super::board::{Board, Square};
use std::{
    collections::HashSet,
    fmt::{self, Display},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    AttackerWins(Vec<usize>), // A list of specific defenders who are defeated
    DefenderWins,             // If the defender wins, all attackers lose
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Outcome::AttackerWins(losers) => {
                write!(f, "Attacker wins against {:#?}", losers)
            }
            Outcome::DefenderWins => write!(f, "Defender wins"),
        }
    }
}

pub struct Judge {
    dictionary: HashSet<String>,
}

// TODO: Allow dictionary to be swapped out.
// pub static DICT: &str = include_str!("../dictionary.txt");
// pub static CSWDICT: &str = include_str!("../csw19.txt");
pub static WORDNIK: &str = include_str!("../wordnik_wordlist.txt");

impl Default for Judge {
    fn default() -> Self {
        let mut dictionary = HashSet::new();

        // for line in DICT.lines() {
        //     dictionary.insert(line.to_lowercase());
        // }

        // let mut lines = CSWDICT.lines();
        // lines.next(); // Skip copyright

        let mut lines = WORDNIK.lines();
        lines.next(); // Skip copyright

        // TODO: Store definitions somewhere to return to the client
        for line in lines {
            dictionary.insert(line.to_string());
        }
        Self { dictionary }
    }
}

impl Judge {
    pub fn new(words: Vec<&str>) -> Self {
        let mut dictionary = HashSet::new();
        for word in words {
            dictionary.insert(word.to_lowercase());
        }
        Self { dictionary }
    }

    // A player wins if they reach the opposite side of the board
    // TODO: accept a config that chooses between different win conditions, like occupying enough quadrants
    // TODO: error (or possibly return a tie) if there are multiple winners - this assume turn based play
    // TODO: put this somewhere better, it conceptually works as a judge associated function, but it only uses values from the board
    pub fn winner(board: &Board) -> Option<usize> {
        for (potential_winner, orientation) in board.get_orientations().iter().enumerate() {
            for coordinate in board.get_near_edge(orientation.opposite()) {
                if let Ok(Square::Occupied(occupier, _)) = board.get(coordinate) {
                    if potential_winner == occupier {
                        return Some(potential_winner);
                    }
                }
            }
        }
        None
    }

    // If there are no attackers or no defenders there is no battle
    // The defender wins if any attacking word is invalid, or all defending words are valid and stronger than the longest attacking words
    // Otherwise the attacker wins
    //
    // There is a defender's advantage, so an attacking word has to be at least 2 letters longer than a defending word to be stronger than it.
    pub fn battle<S: AsRef<str> + Clone + Display>(
        &self,
        attackers: Vec<S>,
        defenders: Vec<S>,
    ) -> Option<BattleReport> {
        // If there are no attackers or no defenders there is no battle
        if attackers.is_empty() || defenders.is_empty() {
            return None;
        }

        let mut battle_report = BattleReport {
            attackers: attackers
                .iter()
                .map(|w| {
                    let valid = self.valid(w);
                    BattleWord {
                        valid: Some(valid.is_some()),
                        definition: None,
                        word: valid.unwrap_or_else(|| w.to_string()),
                    }
                })
                .collect(),
            defenders: defenders
                .iter()
                .map(|w| BattleWord {
                    word: w.to_string(),
                    definition: None,
                    valid: None,
                })
                .collect(),
            outcome: Outcome::DefenderWins,
        };

        // The defender wins if any attacking word is invalid
        if battle_report
            .attackers
            .iter()
            .any(|word| word.valid == Some(false))
        {
            battle_report.outcome = Outcome::DefenderWins;
            return Some(battle_report);
        }

        for defense in &mut battle_report.defenders {
            let valid = self.valid(&*defense.word);
            if let Some(valid) = valid {
                defense.word = valid;
                defense.valid = Some(true);
            } else {
                defense.valid = Some(false);
            }
        }

        // The defender wins if all their words are valid and long enough to defend against the longest attacker
        let longest_attacker = attackers
            .iter()
            .reduce(|longest, curr| {
                if curr.as_ref().len() > longest.as_ref().len() {
                    curr
                } else {
                    longest
                }
            })
            .expect("already checked length");

        let weak_defenders: Vec<usize> = battle_report
            .defenders // Indices of the weak defenders
            .iter()
            .enumerate()
            .filter(|(_, word)| {
                word.valid != Some(true) || word.word.len() + 1 < longest_attacker.as_ref().len()
            })
            .map(|(index, _)| index)
            .collect();
        if weak_defenders.is_empty() {
            battle_report.outcome = Outcome::DefenderWins;
            return Some(battle_report);
        }

        // Otherwise the attacker wins
        battle_report.outcome = Outcome::AttackerWins(weak_defenders);
        Some(battle_report)
    }

    /// Returns the string that was matched if word was a wildcard
    fn valid<S: AsRef<str>>(&self, word: S) -> Option<String> {
        if word.as_ref().contains('*') {
            // Try all letters in the first wildcard spot
            // TODO: find a fun way to optimize this to not be 26^wildcard_count (regex?)
            // TODO: return the validated word all the way to the client for info
            (97..=122_u8)
                .find_map(|c| self.valid(word.as_ref().replacen('*', &(c as char).to_string(), 1)))
        } else {
            if self.dictionary.contains(&word.as_ref().to_lowercase()) {
                Some(word.as_ref().to_string().to_uppercase())
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::board::{tests as BoardUtils, Coordinate, Direction};

    use super::*;

    #[test]
    fn no_battle_without_combatants() {
        let j = short_dict();
        assert_eq!(j.battle(vec!["WORD"], vec![]), None);
        assert_eq!(j.battle(vec![], vec!["WORD"]), None);
        // need to specify a generic here since the vecs are empty, only needed in test
        assert_eq!(j.battle::<&'static str>(vec![], vec![]), None);
    }

    #[test]
    fn attacker_invalid() {
        let j = short_dict();
        assert_eq!(
            j.battle(vec!["XYZ"], vec!["BIG"]).unwrap().outcome,
            Outcome::DefenderWins
        );
        assert_eq!(
            j.battle(vec!["XYZXYZXYZ"], vec!["BIG"]).unwrap().outcome,
            Outcome::DefenderWins
        );
        assert_eq!(
            j.battle(vec!["XYZ", "JOLLY"], vec!["BIG"]).unwrap().outcome,
            Outcome::DefenderWins
        );
        assert_eq!(
            j.battle(vec!["BIG", "XYZ"], vec!["BIG"]).unwrap().outcome,
            Outcome::DefenderWins
        );
        assert_eq!(
            j.battle(vec!["XYZ", "BIG"], vec!["BIG"]).unwrap().outcome,
            Outcome::DefenderWins
        );
    }

    #[test]
    fn defender_invalid() {
        let j = short_dict();
        assert_eq!(
            j.battle(vec!["BIG"], vec!["XYZ"]).unwrap().outcome,
            Outcome::AttackerWins(vec![0])
        );
        assert_eq!(
            j.battle(vec!["BIG"], vec!["XYZXYZXYZ"]).unwrap().outcome,
            Outcome::AttackerWins(vec![0])
        );
        assert_eq!(
            j.battle(vec!["BIG"], vec!["BIG", "XYZ"]).unwrap().outcome,
            Outcome::AttackerWins(vec![1])
        );
        assert_eq!(
            j.battle(vec!["BIG"], vec!["XYZ", "BIG"]).unwrap().outcome,
            Outcome::AttackerWins(vec![0])
        );
    }

    #[test]
    fn attacker_weaker() {
        let j = short_dict();
        assert_eq!(
            j.battle(vec!["JOLLY"], vec!["FOLK"]).unwrap().outcome,
            Outcome::DefenderWins
        );
        assert_eq!(
            j.battle(vec!["JOLLY", "BIG"], vec!["FOLK"])
                .unwrap()
                .outcome,
            Outcome::DefenderWins
        );
    }

    #[test]
    fn defender_weaker() {
        let j = short_dict();
        assert_eq!(
            j.battle(vec!["JOLLY"], vec!["FAT"]).unwrap().outcome,
            Outcome::AttackerWins(vec![0])
        );
        assert_eq!(
            j.battle(vec!["JOLLY", "BIG"], vec!["FAT"]).unwrap().outcome,
            Outcome::AttackerWins(vec![0])
        );
        assert_eq!(
            j.battle(
                vec!["JOLLY"],
                vec!["FAT", "BIG", "JOLLY", "FOLK", "XYZXYZXYZ"]
            )
            .unwrap()
            .outcome,
            Outcome::AttackerWins(vec![0, 1, 4])
        );
    }

    #[test]
    fn wildcards() {
        let j = short_dict();
        assert_eq!(
            j.battle(vec!["B*G"], vec!["XYZ"]).unwrap().outcome,
            Outcome::AttackerWins(vec![0])
        );
        assert_eq!(
            j.battle(vec!["R*G"], vec!["XYZ"]).unwrap().outcome,
            Outcome::DefenderWins
        );
        assert_eq!(
            j.battle(vec!["ARTS"], vec!["JALL*"]).unwrap().outcome,
            Outcome::AttackerWins(vec![0])
        );
        assert_eq!(
            j.battle(vec!["BAG"], vec!["JOLL*"]).unwrap().outcome,
            Outcome::DefenderWins
        );
    }

    #[test]
    fn battle_report() {
        let j = short_dict();
        assert_eq!(
            j.battle(vec!["B*G"], vec!["XYZ"]),
            Some(BattleReport {
                attackers: vec![BattleWord {
                    word: "BAG".into(),
                    definition: None,
                    valid: Some(true)
                }],
                defenders: vec![BattleWord {
                    word: "XYZ".into(),
                    definition: None,
                    valid: Some(false)
                }],
                outcome: Outcome::AttackerWins(vec![0])
            })
        );
        assert_eq!(
            j.battle(vec!["R*G"], vec!["XYZ"]),
            Some(BattleReport {
                attackers: vec![BattleWord {
                    word: "R*G".into(),
                    definition: None,
                    valid: Some(false)
                }],
                defenders: vec![BattleWord {
                    word: "XYZ".into(),
                    definition: None,
                    valid: None
                }],
                outcome: Outcome::DefenderWins
            })
        );

        assert_eq!(
            j.battle(vec!["ARTS"], vec!["JALL*"]),
            Some(BattleReport {
                attackers: vec![BattleWord {
                    word: "ARTS".into(),
                    definition: None,
                    valid: Some(true)
                }],
                defenders: vec![BattleWord {
                    word: "JALL*".into(),
                    definition: None,
                    valid: Some(false)
                }],
                outcome: Outcome::AttackerWins(vec![0])
            })
        );
        assert_eq!(
            j.battle(vec!["BAG"], vec!["JOLL*"]),
            Some(BattleReport {
                attackers: vec![BattleWord {
                    word: "BAG".into(),
                    definition: None,
                    valid: Some(true)
                }],
                defenders: vec![BattleWord {
                    word: "JOLLY".into(),
                    definition: None,
                    valid: Some(true)
                }],
                outcome: Outcome::DefenderWins
            })
        );
    }

    #[test]
    fn main_dict() {
        let j = Judge::default();
        assert_eq!(j.valid("R*G"), Some("RAG".into()));
        assert_eq!(j.valid("zyzzyva"), Some("ZYZZYVA".into()));
        assert_eq!(j.valid("zyzzyvava"), None);
        // Casing indepdendent
        assert_eq!(j.valid("ZYZZYVA"), Some("ZYZZYVA".into()));
    }

    #[test]
    fn win_condition() {
        let mut b = BoardUtils::from_string(
            [
                "    X    ",
                "X X X _ _",
                "X _ _ _ _",
                "X _ _ _ _",
                "_ _ _ _ _",
                "    _    ",
            ]
            .join("\n"),
            vec![Coordinate { x: 0, y: 0 }],
            vec![Direction::North],
        )
        .unwrap();

        assert_eq!(Judge::winner(&b), None);
        b.set(Coordinate { x: 0, y: 4 }, 0, 'X').unwrap();
        assert_eq!(Judge::winner(&b), Some(0));
    }

    // Utils
    pub fn short_dict() -> Judge {
        Judge::new(vec![
            "BIG", "BAG", "FAT", "JOLLY", "AND", "SILLY", "FOLK", "ARTS",
        ]) // TODO: Collins 2018 list
    }
}