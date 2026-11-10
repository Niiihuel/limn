//! Shared, filesystem-free file/folder/language artwork from Material Icon Theme.
//!
//! SVGs and filename associations are pinned and embedded at compile time. A host
//! rasterizes [`FileIcon::svg`] once per physical icon size, retaining its colors.
//! Resolution uses basename → longest extension → generic file; folders never
//! infer their kind from an extension. Both slash styles and ASCII case work on
//! every platform, including paths received from tool activity and remote hosts.
use std::borrow::Cow;

use crate::Icon;

#[path = "file_icon_data.rs"]
mod data;

/// Valid catalog index. Construct through the resolver or [`FileIcon::catalog`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FileIcon(u16);

impl FileIcon {
    /// Unmodified upstream SVG; immutable and embedded, with no filesystem IO.
    pub const fn svg(self) -> &'static str {
        data::ASSETS[self.0 as usize].1
    }

    /// Stable upstream name, useful for accessibility descriptions and exports.
    pub const fn name(self) -> &'static str {
        data::ASSETS[self.0 as usize].0
    }

    /// Every vendored icon, primarily for asset validation and visual catalogs.
    pub fn catalog() -> impl ExactSizeIterator<Item = Self> {
        (0..data::ASSETS.len()).map(|index| Self(index as u16))
    }
}

fn basename(path: &str) -> &str {
    path.trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or_default()
}

fn folded(value: &str) -> Cow<'_, str> {
    if value.bytes().any(|byte| byte.is_ascii_uppercase()) {
        Cow::Owned(value.to_ascii_lowercase())
    } else {
        Cow::Borrowed(value)
    }
}

fn lookup(table: &[(&str, u16)], value: &str) -> Option<u16> {
    table
        .binary_search_by_key(&value, |(key, _)| *key)
        .ok()
        .map(|index| table[index].1)
}

/// Filename takes precedence over the longest recognized compound extension.
/// Unknown, empty and extensionless names have the catalog's generic file icon.
pub fn file_icon(path: &str) -> Icon {
    let name = folded(basename(path));
    if let Some(index) = lookup(data::FILE_NAMES, &name) {
        return Icon::FileType(FileIcon(index));
    }
    for (offset, _) in name.match_indices('.') {
        if let Some(index) = lookup(data::EXTENSIONS, &name[offset + 1..]) {
            return Icon::FileType(FileIcon(index));
        }
    }
    Icon::FileType(FileIcon(data::DEFAULT_FILE))
}

/// Folder state is explicit; this does not read the directory or follow links.
pub fn folder_icon(path: &str, expanded: bool) -> Icon {
    let name = folded(basename(path));
    let index = data::FOLDERS
        .binary_search_by_key(&name.as_ref(), |(key, _)| *key)
        .ok()
        .map(|index| {
            let (closed, open) = data::FOLDERS[index].1;
            if expanded {
                open
            } else {
                closed
            }
        })
        .unwrap_or(if expanded {
            data::DEFAULT_FOLDER_OPEN
        } else {
            data::DEFAULT_FOLDER
        });
    Icon::FileType(FileIcon(index))
}

/// Language IDs, common code-fence aliases, and extension aliases share artwork.
pub fn language_icon(alias: &str) -> Icon {
    let alias = folded(alias.trim().trim_start_matches('.'));
    let canonical = match alias.as_ref() {
        "c++" => "cpp",
        "c#" => "csharp",
        "js" | "mjs" | "cjs" => "javascript",
        "ts" => "typescript",
        "tsx" => "typescriptreact",
        "jsx" => "javascriptreact",
        "rs" => "rust",
        "py" => "python",
        "sh" | "bash" | "zsh" | "shell" => "shellscript",
        "ps1" | "pwsh" => "powershell",
        "md" => "markdown",
        "yml" => "yaml",
        other => other,
    };
    let index = lookup(data::LANGUAGES, canonical)
        .or_else(|| lookup(data::EXTENSIONS, &alias))
        .unwrap_or(data::DEFAULT_FILE);
    Icon::FileType(FileIcon(index))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(icon: Icon) -> &'static str {
        let Icon::FileType(icon) = icon else {
            panic!("file catalog icon expected")
        };
        icon.name()
    }

    #[test]
    fn filenames_win_over_extensions_and_longest_suffix_is_preserved() {
        assert_eq!(name(file_icon("/repo/package.json")), "nodejs");
        assert_eq!(name(file_icon("tsconfig.json")), "tsconfig");
        assert_eq!(name(file_icon("README.md")), "readme");
        assert_eq!(name(file_icon(".gitignore")), "git");
        assert_eq!(name(file_icon("module.d.ts")), "typescript-def");
        assert_eq!(name(file_icon("component.test.ts")), "test-ts");
        assert_eq!(name(file_icon("component.tsx")), "react_ts");
        assert_eq!(name(file_icon("Cargo.toml")), "toml");
        assert_eq!(name(file_icon("Cargo.lock")), "lock");
    }

    #[test]
    fn platform_paths_case_unicode_and_unknown_inputs_are_deterministic() {
        for path in ["src/main.rs", "C:\\repo\\SRC\\MAIN.RS", "./café/東京.rs"] {
            assert_eq!(name(file_icon(path)), "rust", "{path}");
        }
        assert_eq!(file_icon("README.MD"), file_icon("readme.md"));
        for path in [
            "",
            "README-unknown",
            ".unrecognized",
            "unknown.not-an-extension",
        ] {
            assert_eq!(name(file_icon(path)), "file");
        }
        // A parent folder or an earlier extension must not classify a different leaf.
        assert_eq!(name(file_icon("main.rs/unknown")), "file");
        assert_eq!(name(file_icon("main.rs.unknown")), "file");
    }

    #[test]
    fn folders_and_language_aliases_use_the_same_catalog_without_path_io() {
        assert_eq!(name(folder_icon("C:\\repo\\SRC\\", false)), "folder-src");
        assert_eq!(name(folder_icon("/repo/src/", true)), "folder-src-open");
        assert_eq!(name(folder_icon("/repo/unknown.rs", false)), "folder");
        assert_eq!(name(folder_icon("/", true)), "folder-open");
        for (alias, path) in [
            ("RS", "main.rs"),
            ("c++", "main.cpp"),
            ("c#", "Main.cs"),
            ("tsx", "index.tsx"),
            ("BASH", "run.sh"),
            ("python", "main.py"),
            (".json", "data.json"),
        ] {
            assert_eq!(language_icon(alias), file_icon(path), "{alias}");
        }
        assert_eq!(name(language_icon("unsupported language")), "file");
    }

    #[test]
    fn generated_associations_are_sorted_valid_and_have_embedded_svg_sources() {
        for table in [data::FILE_NAMES, data::EXTENSIONS, data::LANGUAGES] {
            assert!(table.windows(2).all(|pair| pair[0].0 < pair[1].0));
            assert!(table
                .iter()
                .all(|(_, index)| (*index as usize) < data::ASSETS.len()));
        }
        assert!(data::FOLDERS.windows(2).all(|pair| pair[0].0 < pair[1].0));
        for (_, (closed, open)) in data::FOLDERS {
            assert_ne!(closed, open);
            assert!((*closed as usize) < data::ASSETS.len());
            assert!((*open as usize) < data::ASSETS.len());
        }
        for icon in FileIcon::catalog() {
            assert!(icon.svg().starts_with("<svg"), "{}", icon.name());
            assert!(icon.svg().contains("viewBox="), "{}", icon.name());
            assert!(!icon.svg().contains("<script"));
            assert!(!icon.svg().contains("<image"));
            assert!(!icon.svg().contains("href="));
        }
    }
}
