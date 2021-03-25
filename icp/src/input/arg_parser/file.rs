// Copyright 2021 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::env::current_dir;
use std::fs::metadata;
use std::io;
use std::path::{Component, Path, PathBuf};

use super::{ArgParseRes, ContextFreeArgParser};

/// See [`file()`] and [`file_for_current_dir()`] for details.
#[derive(PartialEq, Clone, Debug)]
pub struct FileArgParser {
    base: PathBuf,
    hint: String,
}

/// Parses input as a file path.  If the input is a relative path, then it is
/// parsed as relative to `base`.  If the input is an absolute path, then
/// `base` value is disregarded.
#[cfg(test)]
pub fn file<Base, Hint>(base: Base, hint: Hint) -> FileArgParser
where
    Base: Into<PathBuf>,
    Hint: ToString,
{
    FileArgParser {
        base: base.into(),
        hint: hint.to_string(),
    }
}

/// Parses input as a file path.  Works similarly to the [`file()`] constructor,
/// except that `base` is automatically set to the current working directory.
pub fn file_for_current_dir<Hint>(hint: Hint) -> io::Result<FileArgParser>
where
    Hint: ToString,
{
    let base = current_dir()?;
    Ok(FileArgParser {
        base,
        hint: hint.to_string(),
    })
}

enum ParsedInput {
    InvalidPath {
        error: io::Error,
    },
    EntryPrefix {
        parent: PathBuf,
        prefix: String,
        parsed_up_to: usize,
    },
    FileEntry {
        file: PathBuf,
    },
}

/// Removes the last component from a file path given as a string.  `Path` has a
/// similar functionality but it normalizes the input first, which we do not
/// want.  See the usage location.
fn cut_last_component(mut input: &str) -> &str {
    // Skip any number of trailing '/'es.
    input = match input.rfind(|c| c != '/') {
        Some(i) => &input[0..=i],
        None => input,
    };
    // And now skip the very last chunk of non-'/'es.
    match input.rfind(|c| c == '/') {
        Some(i) => &input[0..=i],
        None => "",
    }
}

/// Parses `input` as a file bath relative to `base`.  If `input` is an absolute
/// path, then `base` value is ignored, as per [`Path::push()`].
fn parse_input(mut input: &str, base: &Path) -> ParsedInput {
    // "/." is special as `Path` will normalize it by completely removing both
    // the forward slash and the dot.  I, on the other hand, want to treat it as
    // just another entry.
    if input.ends_with("/.") {
        input = &input[0..(input.len() - 1)];

        let mut path = base.to_path_buf();
        path.push(Path::new(&input[0..(input.len() - 1)]));

        return ParsedInput::EntryPrefix {
            parent: path,
            prefix: ".".into(),
            parsed_up_to: input.len(),
        };
    }

    let show_dir_content = input.is_empty() || input.ends_with('/');

    let mut path = base.to_path_buf();
    path.push(Path::new(input));

    if path.exists() {
        if !path.is_dir() {
            return ParsedInput::FileEntry { file: path };
        } else if show_dir_content {
            return ParsedInput::EntryPrefix {
                parent: path,
                prefix: "".into(),
                parsed_up_to: input.len(),
            };
        }
        // If it exists, is a directory but the user did not end the input with
        // a forward slash we will treat it as a prefix match below.
        //
        // NOTE: An empty statement keeps `cargo fmt` from attaching this
        // comment to the `else if` statement and shifting it to the left.
        ;
    } else if show_dir_content {
        return ParsedInput::InvalidPath {
            error: metadata(path).expect_err(
                "`path.exists()` is false, so `metadata()` is expected to fail",
            ),
        };
    }

    match path.parent() {
        Some(parent) => {
            if parent.exists() && parent.is_dir() {
                let entry = match path.components().next_back().expect(
                    "As the path has a non-empty parent it must contain at \
                     least two components",
                ) {
                    Component::Prefix(prefix) => panic!(
                        "`Component::Prefix` should only occur on Windows. \
                         Got: {:?}",
                        prefix
                    ),
                    Component::RootDir => panic!(
                        "`Component::RootDir` is unexpected in a \
                         non-existing path with existing parent."
                    ),
                    Component::CurDir => ".",
                    Component::ParentDir => "..",
                    Component::Normal(entry) => entry.to_str().expect(
                        "As the input path is a String, it should end up a \
                         valid Unicode sequence after all the transformations",
                    ),
                };
                let prefix = entry.to_string();
                // It is annoying, but I could not figure out how to remove just
                // the last component from the path.  `Path` will do
                // normalization and will not work correctly, at least, for path
                // that end with '/.'.  `prefix` may also end up not been the
                // last part of the input, for example when the user types
                // something line 'abc///'.  So we remove the last component
                // here "manually".
                let parsed_up_to = cut_last_component(input).len();

                ParsedInput::EntryPrefix {
                    parent: parent.to_path_buf(),
                    prefix,
                    parsed_up_to,
                }
            } else {
                ParsedInput::InvalidPath {
                    error: metadata(path).expect_err(
                        "`path.exists()` is false, so `metadata()` is expected \
                         to fail",
                    ),
                }
            }
        }
        None => ParsedInput::InvalidPath {
            error: metadata(path).expect_err(
                "`path.exists()` is false, so `metadata()` is expected to fail",
            ),
        },
    }
}

fn find_matching(dir: &Path, prefix: &str) -> Vec<String> {
    let entries = match dir.read_dir() {
        Ok(entries) => entries,
        Err(_) => return vec![],
    };

    let mut res = vec![];
    for entry in entries {
        // If we fail to read an entry, we just ignore the error.  As an
        // alternative we might consider switching the argument parsers to
        // produce results/suggestions/hints as an atomic item.  In which case
        // we would be able to return the error.  For now, there is no path for
        // error reporting.
        if let Ok(entry) = entry {
            if let Ok(mut name) = entry.file_name().into_string() {
                if name.starts_with(&prefix) {
                    match entry.file_type() {
                        Ok(file_type) if file_type.is_dir() => {
                            name.push('/');
                            res.push(name);
                        }
                        Ok(_) | Err(_) => {
                            res.push(name);
                        }
                    }
                }
            }
        }
    }

    // Make sure our tests are deterministic and the user sees things in a
    // sorted order.
    res.sort_unstable();
    res
}

impl ContextFreeArgParser<PathBuf> for FileArgParser {
    fn parse(&self, input: &str) -> ArgParseRes<PathBuf> {
        match parse_input(input, &self.base) {
            ParsedInput::InvalidPath { error } => ArgParseRes::Failed {
                parsed_up_to: input.len(),
                reason: vec![error.to_string()],
            },
            ParsedInput::EntryPrefix { parsed_up_to, .. } => {
                ArgParseRes::Failed {
                    parsed_up_to,
                    reason: vec![],
                }
            }
            ParsedInput::FileEntry { file } => ArgParseRes::Parsed(file),
        }
    }

    fn suggestion(&self, input_prefix: &str) -> Vec<String> {
        match parse_input(input_prefix, &self.base) {
            ParsedInput::InvalidPath { error } => vec![error.to_string()],
            ParsedInput::EntryPrefix {
                parent,
                prefix,
                parsed_up_to: _,
            } => find_matching(&parent, &prefix),
            ParsedInput::FileEntry { file } => {
                let name = file
                    .components()
                    .next_back()
                    .expect(
                        "As we got a path it should have at least one \
                         component.",
                    )
                    .as_os_str()
                    .to_string_lossy()
                    .into_owned();

                match file.parent() {
                    Some(parent) => find_matching(parent, &name),
                    None => vec![name],
                }
            }
        }
    }

    fn hint(&self) -> Vec<String> {
        vec![self.hint.clone()]
    }
}

#[cfg(test)]
mod tests {
    use super::{cut_last_component, file};

    use crate::input::arg_parser::test_utils::build_cf_parse_checkers;

    use std::fs::{create_dir, File};

    use tempfile::tempdir;

    #[test]
    fn basic_cut_last_component() {
        assert_eq!(cut_last_component(""), "");
        assert_eq!(cut_last_component("name"), "");
        assert_eq!(cut_last_component("/in-root"), "/");
        assert_eq!(cut_last_component("dir1/dir2"), "dir1/");
        assert_eq!(cut_last_component("dir1/dir2/"), "dir1/");
        assert_eq!(cut_last_component("dir1/dir2///"), "dir1/");
    }

    #[test]
    fn simple() {
        let temp_dir = tempdir().unwrap();

        let temp_dir_name = temp_dir
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned()
            + "/";

        create_dir(temp_dir.path().join("dir1")).unwrap();
        create_dir(temp_dir.path().join("dir2")).unwrap();
        let _ = File::create(temp_dir.path().join("dir1/file1.isv")).unwrap();
        let _ = File::create(temp_dir.path().join("dir1/file2.isv")).unwrap();
        let _ = File::create(temp_dir.path().join("dir2/file3.isv")).unwrap();
        let _ = File::create(temp_dir.path().join("dir2/file3")).unwrap();

        let parser = file(temp_dir.path(), "path arg");

        let (check_hint, check_suggestions, check_parse, check_failure) =
            build_cf_parse_checkers("parser", parser);

        let check_parse = |input: &str, expected_path: &str| {
            let mut full_path = temp_dir.path().to_path_buf();
            full_path.push(expected_path);

            check_parse(input, full_path);
        };

        check_hint(&["path arg"]);

        check_parse("dir1/file1.isv", "dir1/file1.isv");
        check_parse("dir1/file2.isv", "dir1/file2.isv");
        check_parse("dir2/file3.isv", "dir2/file3.isv");

        // Directories are not valid targets, so they all must fail.
        check_failure(".", 0, &[]);
        check_failure("dir1", 0, &[]);
        check_failure("dir1/.", 5, &[]);
        check_failure("dir1/./", 7, &[]);
        check_failure("dir1/..", 5, &[]);
        check_failure("dir1/../", 8, &[]);
        check_failure("dir2", 0, &[]);

        check_failure("dir", 0, &[]);
        check_failure("dir1/f", 5, &[]);
        check_failure("dir1/fil", 5, &[]);
        check_failure("dir1/wrong", 5, &[]);
        check_failure("dir2/", 5, &[]);
        check_failure("dir2/file", 5, &[]);

        check_failure("nope", 0, &[]);
        check_failure("dir3", 0, &[]);

        check_suggestions("./", &["dir1/", "dir2/"]);
        check_suggestions("", &["dir1/", "dir2/"]);
        check_suggestions(".", &[&temp_dir_name]);
        check_suggestions("d", &["dir1/", "dir2/"]);
        check_suggestions("a", &[]);
        check_suggestions("dir1", &["dir1/"]);
        check_suggestions("dir1/.", &[]);
        check_suggestions("dir1/./", &["file1.isv", "file2.isv"]);
        check_suggestions("dir1/..", &[]);
        check_suggestions("dir1/../", &["dir1/", "dir2/"]);
        check_suggestions("dir12", &[]);
        check_suggestions("dir1/", &["file1.isv", "file2.isv"]);
        check_suggestions("dir1/f", &["file1.isv", "file2.isv"]);
        check_suggestions("dir1/file", &["file1.isv", "file2.isv"]);
        check_suggestions("dir1/file1", &["file1.isv"]);
        check_suggestions("dir1/file1.isv", &["file1.isv"]);
        check_suggestions("dir1/file1.isv.", &[]);
        check_suggestions("dir2", &["dir2/"]);
        check_suggestions("dir2/", &["file3", "file3.isv"]);
        check_suggestions("dir2/f", &["file3", "file3.isv"]);
        check_suggestions("dir2/file", &["file3", "file3.isv"]);
        check_suggestions("dir2/file3", &["file3", "file3.isv"]);
        check_suggestions("dir2/file3.", &["file3.isv"]);
        check_suggestions("dir2/file3.isv", &["file3.isv"]);
        check_suggestions("dir2/file3.isvz", &[]);
        check_suggestions("dir2/file4", &[]);
    }
}
