use serde::{Deserialize, Serialize};

use crate::{
    error::GamePlayError,
    reporting::{BattleReport, BattleWord},
    rules,
};

use super::board::{Board, Square};
use std::{
    collections::{HashMap, HashSet},
    fmt::{self, Display},
};

#[derive(Clone)]
pub struct WordData {
    pub extensions: u32,
    pub rel_freq: f32,
}
pub type WordDict = HashMap<String, WordData>;

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

#[derive(Clone)]
pub struct Judge {
    builtin_dictionary: WordDict,
    aliases: HashMap<char, Vec<char>>,
}

impl Default for Judge {
    fn default() -> Self {
        Self {
            builtin_dictionary: HashMap::new(),
            aliases: HashMap::new(),
        }
    }
}

impl Judge {
    pub fn new(words: Vec<String>) -> Self {
        let mut dictionary = HashMap::new();
        for word in words {
            dictionary.insert(
                word.to_lowercase(),
                WordData {
                    extensions: 0,
                    rel_freq: 0.0,
                },
            );
        }
        Self {
            builtin_dictionary: dictionary,
            aliases: HashMap::new(),
        }
    }

    pub fn set_alias(&mut self, alias_target: Vec<char>) -> char {
        for p in ['①', '②', '③', '④', '⑤', '⑥', '⑦', '⑧', '⑨'] {
            if self.aliases.contains_key(&p) {
                continue;
            }
            self.aliases.insert(p, alias_target);
            return p;
        }
        panic!("Too many aliases!");
    }

    pub fn remove_aliases(&mut self) {
        self.aliases.clear();
    }

    // A player wins if they touch an opponent's town
    // TODO: accept a config that chooses between different win conditions, like occupying enough quadrants
    // TODO: error (or possibly return a tie) if there are multiple winners - this assume turn based play
    // TODO: put this somewhere better, it conceptually works as a judge associated function, but it only uses values from the board
    pub fn winner(board: &Board) -> Option<usize> {
        let win_squares = board
            .towns()
            .map(|town_coord| {
                let Ok(Square::Town(owner)) = board.get(*town_coord) else {
                panic!("The list of towns on the board should match valid squares");
            };

                (
                    owner,
                    town_coord
                        .neighbors_4()
                        .iter()
                        .map(|win_coord| board.get(*win_coord))
                        .flatten()
                        .filter_map(|square| match square {
                            Square::Occupied(tile_owner, _) if tile_owner != owner => {
                                Some(tile_owner)
                            }
                            _ => None,
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .filter(|(_, winning_squares)| !winning_squares.is_empty())
            .collect::<Vec<_>>();

        // TODO: Handle multiple entries in this array
        if let Some((losing_town, winning_tiles)) = win_squares.first() {
            return winning_tiles.first().cloned();
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
        battle_rules: &rules::BattleRules,
        external_dictionary: Option<&WordDict>,
    ) -> Option<BattleReport> {
        // If there are no attackers or no defenders there is no battle
        if attackers.is_empty() || defenders.is_empty() {
            return None;
        }

        let mut battle_report = BattleReport {
            attackers: attackers
                .iter()
                .map(|w| {
                    let valid = self.valid(w, external_dictionary, None);
                    BattleWord {
                        valid: Some(valid.is_some()),
                        meanings: None,
                        word: valid.unwrap_or_else(|| w.to_string()),
                    }
                })
                .collect(),
            defenders: defenders
                .iter()
                .map(|w| BattleWord {
                    word: w.to_string(),
                    meanings: None,
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
            let valid = self.valid(&*defense.word, external_dictionary, None);
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

        let attacker_wins_outright = attackers.iter().any(|word| word.as_ref().contains('¤'));
        if attacker_wins_outright {
            battle_report.outcome = Outcome::AttackerWins(vec![]);
            return Some(battle_report);
        }

        let weak_defenders: Vec<usize> = battle_report
            .defenders // Indices of the weak defenders
            .iter()
            .enumerate()
            .filter(|(_, word)| {
                word.valid != Some(true)
                    || word.word.len() as isize + battle_rules.length_delta as isize
                        <= longest_attacker.as_ref().len() as isize
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
    pub fn valid<S: AsRef<str>>(
        &self,
        word: S,
        external_dictionary: Option<&WordDict>,
        used_aliases: Option<HashMap<char, Vec<usize>>>,
    ) -> Option<String> {
        if word.as_ref().contains('¤') {
            return Some(word.as_ref().to_string().to_uppercase());
        }

        // Handle the first matching alias we find (others will be handled in the next recursion)
        for (alias, resolved) in &self.aliases {
            if word.as_ref().contains(*alias) {
                return resolved.iter().enumerate().find_map(|(i, c)| {
                    let mut used = used_aliases.clone().unwrap_or_default();
                    if used.get(c).map(|tiles| tiles.contains(&i)) == Some(true) {
                        return None;
                    }

                    if let Some(used_alias) = used.get_mut(c) {
                        used_alias.push(i);
                    } else {
                        used.insert(*c, vec![i]);
                    }

                    self.valid(
                        word.as_ref().replacen(*alias, &c.to_string(), 1),
                        external_dictionary,
                        Some(used),
                    )
                });
            }
        }

        if word.as_ref().contains('*') {
            // Try all letters in the first wildcard spot
            // TODO: find a fun way to optimize this to not be 26^wildcard_count (regex?)
            return (97..=122_u8).find_map(|c| {
                self.valid(
                    word.as_ref().replacen('*', &(c as char).to_string(), 1),
                    external_dictionary,
                    used_aliases.clone(),
                )
            });
        }

        if external_dictionary
            .unwrap_or(&self.builtin_dictionary)
            .contains_key(&word.as_ref().to_lowercase())
        {
            Some(word.as_ref().to_string().to_uppercase())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::board::{tests as BoardUtils, Coordinate, Direction};

    use super::*;

    fn test_rules() -> rules::BattleRules {
        rules::BattleRules { length_delta: 2 }
    }

    #[test]
    fn no_battle_without_combatants() {
        let j = short_dict();
        assert_eq!(j.battle(vec!["WORD"], vec![], &test_rules(), None), None);
        assert_eq!(j.battle(vec![], vec!["WORD"], &test_rules(), None), None);
        // need to specify a generic here since the vecs are empty, only needed in test
        assert_eq!(
            j.battle::<&'static str>(vec![], vec![], &test_rules(), None),
            None
        );
    }

    #[test]
    fn attacker_invalid() {
        let j = short_dict();
        assert_eq!(
            j.battle(vec!["XYZ"], vec!["BIG"], &test_rules(), None)
                .unwrap()
                .outcome,
            Outcome::DefenderWins
        );
        assert_eq!(
            j.battle(vec!["XYZXYZXYZ"], vec!["BIG"], &test_rules(), None)
                .unwrap()
                .outcome,
            Outcome::DefenderWins
        );
        assert_eq!(
            j.battle(vec!["XYZ", "JOLLY"], vec!["BIG"], &test_rules(), None)
                .unwrap()
                .outcome,
            Outcome::DefenderWins
        );
        assert_eq!(
            j.battle(vec!["BIG", "XYZ"], vec!["BIG"], &test_rules(), None)
                .unwrap()
                .outcome,
            Outcome::DefenderWins
        );
        assert_eq!(
            j.battle(vec!["XYZ", "BIG"], vec!["BIG"], &test_rules(), None)
                .unwrap()
                .outcome,
            Outcome::DefenderWins
        );
    }

    #[test]
    fn defender_invalid() {
        let j = short_dict();
        assert_eq!(
            j.battle(vec!["BIG"], vec!["XYZ"], &test_rules(), None)
                .unwrap()
                .outcome,
            Outcome::AttackerWins(vec![0])
        );
        assert_eq!(
            j.battle(vec!["BIG"], vec!["XYZXYZXYZ"], &test_rules(), None)
                .unwrap()
                .outcome,
            Outcome::AttackerWins(vec![0])
        );
        assert_eq!(
            j.battle(vec!["BIG"], vec!["BIG", "XYZ"], &test_rules(), None)
                .unwrap()
                .outcome,
            Outcome::AttackerWins(vec![1])
        );
        assert_eq!(
            j.battle(vec!["BIG"], vec!["XYZ", "BIG"], &test_rules(), None)
                .unwrap()
                .outcome,
            Outcome::AttackerWins(vec![0])
        );
    }

    #[test]
    fn attacker_weaker() {
        let j = short_dict();
        assert_eq!(
            j.battle(vec!["JOLLY"], vec!["FOLK"], &test_rules(), None)
                .unwrap()
                .outcome,
            Outcome::DefenderWins
        );
        assert_eq!(
            j.battle(vec!["JOLLY", "BIG"], vec!["FOLK"], &test_rules(), None)
                .unwrap()
                .outcome,
            Outcome::DefenderWins
        );
    }

    #[test]
    fn defender_weaker() {
        let j = short_dict();
        assert_eq!(
            j.battle(vec!["JOLLY"], vec!["FAT"], &test_rules(), None)
                .unwrap()
                .outcome,
            Outcome::AttackerWins(vec![0])
        );
        assert_eq!(
            j.battle(vec!["JOLLY", "BIG"], vec!["FAT"], &test_rules(), None)
                .unwrap()
                .outcome,
            Outcome::AttackerWins(vec![0])
        );
        assert_eq!(
            j.battle(
                vec!["JOLLY"],
                vec!["FAT", "BIG", "JOLLY", "FOLK", "XYZXYZXYZ"],
                &test_rules(),
                None
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
            j.battle(vec!["B*G"], vec!["XYZ"], &test_rules(), None)
                .unwrap()
                .outcome,
            Outcome::AttackerWins(vec![0])
        );
        assert_eq!(
            j.battle(vec!["R*G"], vec!["XYZ"], &test_rules(), None)
                .unwrap()
                .outcome,
            Outcome::DefenderWins
        );
        assert_eq!(
            j.battle(vec!["ARTS"], vec!["JALL*"], &test_rules(), None)
                .unwrap()
                .outcome,
            Outcome::AttackerWins(vec![0])
        );
        assert_eq!(
            j.battle(vec!["BAG"], vec!["JOLL*"], &test_rules(), None)
                .unwrap()
                .outcome,
            Outcome::DefenderWins
        );
    }

    #[test]
    fn aliases() {
        let mut j = short_dict();

        let a_or_b = j.set_alias(vec!['a', 'b']);
        let b_or_c = j.set_alias(vec!['b', 'c']);

        assert_eq!(
            j.battle(
                vec![format!("B{a_or_b}G").as_str()],
                vec!["XYZ"],
                &test_rules(),
                None
            )
            .unwrap()
            .outcome,
            Outcome::AttackerWins(vec![0])
        );
        assert_eq!(
            j.battle(
                vec![format!("B{b_or_c}G").as_str()],
                vec!["XYZ"],
                &test_rules(),
                None
            )
            .unwrap()
            .outcome,
            Outcome::DefenderWins
        );
        assert_eq!(
            j.battle(
                vec![format!("{b_or_c}{a_or_b}G").as_str()],
                vec!["XYZ"],
                &test_rules(),
                None
            )
            .unwrap()
            .outcome,
            Outcome::AttackerWins(vec![0])
        );
    }

    #[test]
    fn multi_aliases() {
        let mut j = short_dict();

        let o_or_l = j.set_alias(vec!['o', 'l']);
        let two_ls = j.set_alias(vec!['l', 'l']);

        // We can't double-dip on a tile with an alias
        assert_eq!(
            j.battle(
                vec![format!("JO{o_or_l}{o_or_l}Y").as_str()],
                vec!["XYZ"],
                &test_rules(),
                None
            )
            .unwrap()
            .outcome,
            Outcome::DefenderWins
        );
        // But we can use multiple of the same
        assert_eq!(
            j.battle(
                vec![format!("JO{two_ls}{two_ls}Y").as_str()],
                vec!["XYZ"],
                &test_rules(),
                None
            )
            .unwrap()
            .outcome,
            Outcome::AttackerWins(vec![0])
        );
        // Or multiple that are different
        assert_eq!(
            j.battle(
                vec![format!("J{o_or_l}L{o_or_l}Y").as_str()],
                vec!["XYZ"],
                &test_rules(),
                None
            )
            .unwrap()
            .outcome,
            Outcome::AttackerWins(vec![0])
        );
    }

    #[test]
    fn battle_report() {
        let j = short_dict();
        assert_eq!(
            j.battle(vec!["B*G"], vec!["XYZ"], &test_rules(), None),
            Some(BattleReport {
                attackers: vec![BattleWord {
                    word: "BAG".into(),
                    meanings: None,
                    valid: Some(true)
                }],
                defenders: vec![BattleWord {
                    word: "XYZ".into(),
                    meanings: None,
                    valid: Some(false)
                }],
                outcome: Outcome::AttackerWins(vec![0])
            })
        );
        assert_eq!(
            j.battle(vec!["R*G"], vec!["XYZ"], &test_rules(), None),
            Some(BattleReport {
                attackers: vec![BattleWord {
                    word: "R*G".into(),
                    meanings: None,
                    valid: Some(false)
                }],
                defenders: vec![BattleWord {
                    word: "XYZ".into(),
                    meanings: None,
                    valid: None
                }],
                outcome: Outcome::DefenderWins
            })
        );

        assert_eq!(
            j.battle(vec!["ARTS"], vec!["JALL*"], &test_rules(), None),
            Some(BattleReport {
                attackers: vec![BattleWord {
                    word: "ARTS".into(),
                    meanings: None,
                    valid: Some(true)
                }],
                defenders: vec![BattleWord {
                    word: "JALL*".into(),
                    meanings: None,
                    valid: Some(false)
                }],
                outcome: Outcome::AttackerWins(vec![0])
            })
        );
        assert_eq!(
            j.battle(vec!["BAG"], vec!["JOLL*"], &test_rules(), None),
            Some(BattleReport {
                attackers: vec![BattleWord {
                    word: "BAG".into(),
                    meanings: None,
                    valid: Some(true)
                }],
                defenders: vec![BattleWord {
                    word: "JOLLY".into(),
                    meanings: None,
                    valid: Some(true)
                }],
                outcome: Outcome::DefenderWins
            })
        );
    }

    // #[test]
    // fn main_dict() {
    //     let j = Judge::default();
    //     assert_eq!(j.valid("R*G", None), Some("RAG".into()));
    //     assert_eq!(j.valid("zyzzyva", None), Some("ZYZZYVA".into()));
    //     assert_eq!(j.valid("zyzzyvava", None), None);
    //     // Casing indepdendent
    //     assert_eq!(j.valid("ZYZZYVA", None), Some("ZYZZYVA".into()));
    // }

    // #[test]
    // fn win_condition() {
    //     let mut b = BoardUtils::from_string(
    //         [
    //             "    X    ",
    //             "X X X _ _",
    //             "X _ _ _ _",
    //             "X _ _ _ _",
    //             "_ _ _ _ _",
    //             "    _    ",
    //         ]
    //         .join("\n"),
    //         vec![Coordinate { x: 0, y: 0 }],
    //         vec![Direction::North],
    //     )
    //     .unwrap();

    //     assert_eq!(Judge::winner(&b), None);
    //     b.set(Coordinate { x: 0, y: 4 }, 0, 'X').unwrap();
    //     assert_eq!(Judge::winner(&b), Some(0));
    // }

    // Utils
    pub fn short_dict() -> Judge {
        Judge::new(vec![
            "BIG".into(),
            "BAG".into(),
            "FAT".into(),
            "JOLLY".into(),
            "AND".into(),
            "SILLY".into(),
            "FOLK".into(),
            "ARTS".into(),
        ]) // TODO: Collins 2018 list
    }
}