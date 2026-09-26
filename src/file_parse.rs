use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{self, Path};

use crate::modrinth::ModrinthItem;
use crate::{items, modrinth};

static TRACKER_FILENAME: &'static str = ".tracker.mcmg";

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

pub struct TrackerEntry
{
    version_id: String,
    filename: String,
}
impl TrackerEntry
{
    pub fn new_entry(v_id: String, filename: String) -> Self
    {
        TrackerEntry { version_id: (v_id), filename }
    }
    fn from_csv(csv: &String) -> Option<(String, Self)>
    {
        let mut values = csv.split(',');
        let key = match values.next()
        {
            Some(v) => String::from(v),
            None => return None
        };
        let v_id = match values.next()
        {
            Some(v) => String::from(v),
            None => return None
        };
        let filename = match values.next()
        {
            Some(v) => String::from(v),
            None => return None
        };

        Some((key, Self::new_entry(v_id, filename)))
    }

}

pub struct TrackerFile
{
    entries: HashMap<String, TrackerEntry>,
    filepath: path::PathBuf,
}
impl TrackerFile
{
    pub fn build(folder: &Path) -> io::Result<Self>
    {
        let filepath = folder.join(TRACKER_FILENAME);
        if filepath.exists()
        {
            Self::build_from_file(filepath)
        }
        else
        {
            Ok(TrackerFile { filepath, entries: HashMap::<String, TrackerEntry>::new() })
        }
    }

    pub fn entry(&self, key: &str) -> Option<&TrackerEntry>
    {
        self.entries.get(key)
    }

    pub fn entry_count(&self) -> usize
    {
        self.entries.len()
    }

    pub fn matches_entry(&self, item: &ModrinthItem) -> bool
    {
        match self.entry(item.id())
        {
            Some(ent) => item.version_id() == &ent.version_id,
            None => false
        }
    }

    pub fn build_from_file(filepath: path::PathBuf) -> io::Result<Self>
    {
        let mut entries = HashMap::<String, TrackerEntry>::new();

        let f_in = File::open(&filepath)?;
        let reader = BufReader::new(f_in);
        
        for line in reader.lines()
        {
            if let Ok(l) = line && let Some(entry) = TrackerEntry::from_csv(&l)
            {
                entries.insert(entry.0, entry.1);
            }
        }

        Ok(TrackerFile { filepath, entries })
    }

    pub fn update_entry(&mut self, id: &String, entry: TrackerEntry) -> ()
    {
        self.entries.insert(id.to_string(), entry);
    }

    pub fn update(&mut self, modlist: Vec<modrinth::ModrinthItem>) -> ()
    {
        for m in modlist
        {
            if m.downloaded()
            {
                self.update_entry(m.id(), TrackerEntry::new_entry(
                    m.version_id().to_string(),
                    m.filename().to_string())
                );
            }
        }
    }

    pub fn write_to_tracker_file(&self) -> io::Result<()>
    {
        let mut f_out = File::create(&self.filepath)?;

        for (key, val) in &self.entries
        {
            if let Err(err) = writeln!(f_out, "{},{},{}", key, val.version_id, val.filename)
            {
                println!("[TRACKER/ERROR] Failed to save entry for '{}': {}", key, err)
            };
        }

        Ok(())
    }
}