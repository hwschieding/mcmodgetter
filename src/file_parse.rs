use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{self, Path, PathBuf};

use csv;
use serde::{Deserialize, Serialize};

use crate::modrinth;

static TRACKER_FILENAME: &'static str = ".tracker.mcmg";
static TRACKER_SIG: &'static str = "TRACKER";

fn print_tracker_error(msg: String)
{
    println!("[{}/ERROR] {}", TRACKER_SIG, msg);
}

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
    filename: PathBuf,
}
impl TrackerEntry
{
    pub fn new_entry(v_id: String, filename: PathBuf) -> Self
    {
        TrackerEntry { version_id: (v_id), filename }
    }
}

pub struct TrackerFile
{
    entries: HashMap<String, TrackerEntry>,
    filepath: PathBuf
}
impl TrackerFile
{
    pub fn build(folder: &Path) -> csv::Result<Self>
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

    fn entry(&self, key: &str) -> Option<&TrackerEntry>
    {
        self.entries.get(key)
    }

    pub fn entry_count(&self) -> usize
    {
        self.entries.len()
    }

    pub fn matches_entry(&self, item: &modrinth::ModrinthItem) -> bool
    {
        match self.entry(item.id())
        {
            Some(ent) => item.version_id() == &ent.version_id,
            None => false
        }
    }

    fn delete_entry_file(&mut self, key: &str) -> ()
    {
        if let Some(ent) = self.entry(key)
            && let Err(err) = fs::remove_file(&ent.filename)
        {
            print_tracker_error(format!("Failed to remove old file: {}", err));
        }
    }

    fn build_from_file(filepath: path::PathBuf) -> csv::Result<Self>
    {
        let mut entries = HashMap::<String, TrackerEntry>::new();

        let mut reader = csv::ReaderBuilder::new()
            .from_path(&filepath)?
        ;

        for result in reader.deserialize()
        {
            let record: CsvRecord = match result
            {
                Ok(r) => r,
                Err(e) => {
                    print_tracker_error(format!("Couldn't retrieve record: {}", e));
                    continue;
                }
            };

            entries.insert(
                record.id,
                TrackerEntry::new_entry(record.vid, record.filename)
            );
        }

        Ok(TrackerFile { filepath, entries })
    }

    pub fn update(&mut self, modlist: Vec<modrinth::ModrinthItem>) -> ()
    {
        for m in modlist
        {
            if m.downloaded()
            {
                self.delete_entry_file(m.id());

                // let filename = directory.join(m.filename());

                self.entries.insert(m.id().to_string(), TrackerEntry::new_entry(
                    m.version_id().to_string(),
                    match self.filepath.parent()
                    {
                        Some(p) => p,
                        None => {
                            print_tracker_error(String::from("Dunno how this one happened."));
                            return ()
                        }   
                    }.join(m.filename())
                ));
            }
        }
    }

    pub fn write_to_tracker_file(&self) -> io::Result<()>
    {
        let mut f_out = File::create(&self.filepath)?;

        for (key, val) in &self.entries
        {
            if let Err(err) = writeln!(f_out, "{},{},{}", key, val.version_id, val.filename.display())
            {
                print_tracker_error(format!("Failed to save entry for '{}': {}", key, err));
            };
        }

        Ok(())
    }

    pub fn write_csv(&self) -> ()
    {
        let mut writer = match csv::WriterBuilder::new()
            .from_path(&self.filepath)
        {
            Ok(w) => w,
            Err(e) => {
                print_tracker_error(format!("Couldn't create tracker file: {}", e));
                return ()
            }
        };

        for (key, val) in &self.entries
        {
            let record = CsvRefRecord{ id: key, vid: &val.version_id, filename: &val.filename };
            if let Err(e) = writer.serialize(record)
            {
                print_tracker_error(format!("Entry for '{}' failed to write: {}", key, e));
            }
        };
        
        ()
    }
}

#[derive(Serialize)]
struct CsvRefRecord<'a>
{
    id: &'a str,
    vid: &'a str,
    filename: &'a Path,
}

#[derive(Deserialize)]
struct CsvRecord
{
    id: String,
    vid: String,
    filename: PathBuf,
}