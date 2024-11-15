use std::collections::HashMap;
use truncate_core::judge::{WordData, WordDict};

pub mod service {
    tonic::include_proto!("service");
}

pub static TRUNCATE_DICT: &str = include_str!("../../dict_builder/final_wordlist.txt");

pub fn init_dict() -> anyhow::Result<WordDict> {
    let mut valid_words = HashMap::new();
    let lines = TRUNCATE_DICT.lines();

    for line in lines {
        let mut chunks = line.split(' ');

        let mut word = chunks.next().unwrap().to_string();
        let objectionable = word.chars().next() == Some('*');
        if objectionable {
            word.remove(0);
        }

        valid_words.insert(
            word,
            WordData {
                extensions: chunks.next().unwrap().parse()?,
                rel_freq: chunks.next().unwrap().parse()?,
                objectionable,
            },
        );
    }

    Ok(valid_words)
}
