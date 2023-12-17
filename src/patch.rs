use crate::error::PatchError;
use patch::Patch;
use std::{error::Error, fmt::Display, path::PathBuf};

pub struct PatchOptions {
    pub line_ending: String,
    pub work_directory: PathBuf,
    pub strip_num: Option<u32>,
}

impl Default for PatchOptions {
    fn default() -> Self {
        Self {
            line_ending: if std::env::consts::OS == "windows" {
                "\r\n".to_string()
            } else {
                "\n".to_string()
            },
            work_directory: PathBuf::from(""),
            strip_num: None,
        }
    }
}

impl Display for PatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::ParseError(err) => format!("ParseError::ParseError: {}", err),
                Self::IOError(err) => format!("PatchError::FileError: {}", err),
                Self::Unknown => "PatchError::Unknown: Unknown error.".to_string(),
            }
        )
    }
}

impl Error for PatchError {}

pub fn apply_patch(patch: &str, options: PatchOptions) -> Result<(), PatchError> {
    let workdir = options.work_directory;
    let patch = if patch.ends_with(&options.line_ending) {
        patch.to_string()
    } else {
        patch.to_string() + &options.line_ending
    };
    match Patch::from_multiple(&patch) {
        Ok(patches) => {
            for patch in patches {
                match std::fs::read_to_string(
                    workdir
                        .join(
                            patch
                                .old
                                .path
                                .trim()
                                .split_ascii_whitespace()
                                .nth(1)
                                .unwrap()
                                .to_string(),
                        )
                        .components()
                        .skip(options.strip_num.unwrap_or(0) as usize)
                        .collect::<PathBuf>(),
                ) {
                    Ok(old) => {
                        let mut hunks = patch.hunks.into_iter();
                        let old_lines: Vec<_> = old.split(&options.line_ending).collect();
                        let mut new_lines = Vec::<String>::new();
                        let mut old_line_num = 1usize;
                        while let Some(hunk) = hunks.next() {
                            if old_line_num < hunk.old_range.start as usize {
                                new_lines.extend(
                                    old_lines[(old_line_num as usize - 1)
                                        ..hunk.old_range.start as usize]
                                        .iter()
                                        .map(|x| x.to_string()),
                                );
                            }
                            old_line_num = hunk.old_range.start as usize;
                            let mut patch_lines = hunk.lines.into_iter();
                            while let Some(patch_line) = patch_lines.next() {
                                match patch_line {
                                    patch::Line::Remove(_) => {
                                        old_line_num += 1;
                                    }
                                    patch::Line::Add(add_line) => {
                                        new_lines.push(add_line.to_string());
                                    }
                                    patch::Line::Context(_) => {
                                        new_lines.push(old_lines[old_line_num - 1].to_string());
                                        old_line_num += 1;
                                    }
                                }
                            }
                        }
                        for i in old_line_num.. {
                            if i < old_lines.len() {
                                new_lines.push(old_lines[i].to_string());
                            } else {
                                break;
                            }
                        }
                        let new = new_lines.join(&options.line_ending);
                        match std::fs::write(
                            workdir
                                .join(
                                    patch
                                        .new
                                        .path
                                        .to_string()
                                        .split_ascii_whitespace()
                                        .next()
                                        .unwrap(),
                                )
                                .components()
                                .skip(options.strip_num.unwrap_or(0) as usize)
                                .collect::<PathBuf>(),
                            new,
                        ) {
                            Ok(_) => {}
                            Err(_) => return Err(PatchError::Unknown),
                        }
                    }
                    Err(err) => return Err(PatchError::IOError(err)),
                }
            }
            Ok(())
        }
        Err(err) => Err(PatchError::ParseError(err.to_string())),
    }
}
