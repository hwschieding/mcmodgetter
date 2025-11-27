use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

pub static FILE_EXT: &'static str = "mmg";

pub enum IdType<'a> {
    Modrinth(&'a str),
    Curseforge(&'a str),
}

pub struct FileIDs {
    modrinth: Option<Vec<String>>,
    curseforge: Option<Vec<String>>,
}

impl FileIDs {
    pub fn build(modrinth_ids: Vec<String>, curse_ids: Vec<String>) -> FileIDs {
        let modrinth = match modrinth_ids.len() {
            0 => None,
            _ => Some(modrinth_ids)
        };
        let curseforge = match curse_ids.len() {
            0 => None,
            _ => Some(curse_ids)
        };
        FileIDs { modrinth, curseforge }
    }

    pub fn build_modrinth_only(ids: Vec<String>) -> FileIDs {
        let modrinth = match ids.len() {
            0 => None,
            _ => Some(ids)
        };
        let curseforge = None;
        FileIDs { modrinth, curseforge }
    }
    
    pub fn modrinth(&self) -> &Option<Vec<String>> {
        &self.modrinth
    }

    pub fn curseforge(&self) -> &Option<Vec<String>> {
        &self.curseforge
    }
}

pub fn parse_ids(mmg_filepath: &Path) -> io::Result<FileIDs> {
    let mut modrinth_ids: Vec<String> = Vec::new();
    let mut curse_ids: Vec<String> = Vec::new();

    let f_in = File::open(mmg_filepath)?;
    let reader = BufReader::new(f_in);
    for line_res in reader.lines() {
        let line = line_res?;
        if let Some(val) = parse_mmg_line(&line){
            match val {
                IdType::Modrinth(id) => { modrinth_ids.push(String::from(id)); },
                IdType::Curseforge(id) => { curse_ids.push(String::from(id)); }
            }
        }
    }

    Ok(FileIDs::build(modrinth_ids, curse_ids))
}

pub fn parse_mmg_line<'a>(line: &'a String) -> Option<IdType<'a>> {
    let mut line_iter = line.split(" ");
    let id: &'a str = match line_iter.next() {
        Some(val) => val,
        None => { return None; }
    };
    if let Some(val) = line_iter.next() {
        match val {
            "-curse" => Some(IdType::Curseforge(id)),
            _ => Some(IdType::Modrinth(id))
        }
    } else {
        Some(IdType::Modrinth(id))
    }
}

pub fn parse_ids_txt(txt_filepath: &Path) -> io::Result<FileIDs> {
    let mut ids: Vec<String> = Vec::new();
    let f_in = File::open(txt_filepath)?;
    let reader = BufReader::new(f_in);
    for line_res in reader.lines() {
        let line = line_res?;
        ids.push(line);
    };
    Ok(FileIDs::build_modrinth_only(ids))
}