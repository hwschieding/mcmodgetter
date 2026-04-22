use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

pub enum IdType<'a> {
    Modrinth(&'a str),
    Curseforge(&'a str),
    Hangar(&'a str),
}

pub struct FileIDs {
    modrinth: Option<Vec<String>>,
    curseforge: Option<Vec<String>>,
    hangar: Option<Vec<String>>,
}

impl FileIDs {
    pub fn build(
        modrinth_ids: Vec<String>,
        curse_ids: Vec<String>,
        hangar_ids: Vec<String>,
    ) -> FileIDs {
        let modrinth = match modrinth_ids.len() {
            0 => None,
            _ => Some(modrinth_ids)
        };
        let curseforge = match curse_ids.len() {
            0 => None,
            _ => Some(curse_ids)
        };
        let hangar = match hangar_ids.len() {
            0 => None,
            _ => Some(hangar_ids)
        };
        FileIDs { modrinth, curseforge, hangar }
    }

    pub fn build_modrinth_only(ids: Vec<String>) -> FileIDs {
        let modrinth = match ids.len() {
            0 => None,
            _ => Some(ids)
        };
        let curseforge = None;
        let hangar = None;
        FileIDs { modrinth, curseforge, hangar }
    }
    
    pub fn modrinth(&self) -> &Option<Vec<String>> {
        &self.modrinth
    }

    pub fn curseforge(&self) -> &Option<Vec<String>> {
        &self.curseforge
    }

    pub fn hangar(&self) -> &Option<Vec<String>> {
        &self.hangar
    }
}

pub fn parse_ids(filepath: &Path) -> io::Result<FileIDs> {
    let mut modrinth_ids: Vec<String> = Vec::new();
    let mut curse_ids: Vec<String> = Vec::new();
    let mut hangar_ids: Vec<String> = Vec::new();

    let f_in = File::open(filepath)?;
    let reader = BufReader::new(f_in);
    for line_res in reader.lines() {
        let line = line_res?;
        if let Some(c) = line.chars().nth(0) && c == '#' {
            println!("Skipping line '{line}'");
        } else if let Some(val) = parse_input_line(&line){
            match val {
                IdType::Modrinth(id) => { modrinth_ids.push(String::from(id)); },
                IdType::Curseforge(id) => { curse_ids.push(String::from(id)); },
                IdType::Hangar(id) => { hangar_ids.push(String::from(id)); },
            }
        }
    }

    Ok(FileIDs::build(modrinth_ids, curse_ids, hangar_ids))
}

pub fn parse_input_line<'a>(line: &'a String) -> Option<IdType<'a>> {
    let mut line_iter = line.split(" ");
    let id: &'a str = match line_iter.next() {
        Some(val) => val,
        None => { return None; }
    };
    if let Some(val) = line_iter.next() {
        match val {
            "-curse" => Some(IdType::Curseforge(id)),
            "-hang" => Some(IdType::Hangar(id)),
            _ => Some(IdType::Modrinth(id)),
        }
    } else {
        Some(IdType::Modrinth(id))
    }
}