//! The git commit a folder mod's files are checked out at: what a load report names beside the
//! mod, so a submodule that `git pull` left behind is visible (`docs/tools/mods.md`, "Which
//! files a checkout loads"). Read from the files git keeps, never by running `git`.

use std::path::{Path, PathBuf};

/// The short (7-digit) commit of the git checkout `dir` is in, or `None` outside one or when its
/// `HEAD` will not resolve.
#[must_use]
pub fn checkout_head(dir: &Path) -> Option<String> {
    let git_dir = dir.ancestors().find_map(git_dir_of)?;
    let head = std::fs::read_to_string(git_dir.join("HEAD")).ok()?;
    let head = head.trim();
    let full = match head.strip_prefix("ref: ") {
        Some(reference) => {
            let common = common_dir(&git_dir);
            std::fs::read_to_string(common.join(reference))
                .ok()
                .map(|hash| hash.trim().to_string())
                .or_else(|| packed_ref(&common, reference))?
        }
        None => head.to_string(),
    };
    let short = full.get(..7)?;
    short
        .bytes()
        .all(|b| b.is_ascii_hexdigit())
        .then(|| short.to_string())
}

/// `dir/.git` as a git directory: the directory itself, or the one a `gitdir:` file (a
/// submodule's, a worktree's) names, relative to `dir`.
fn git_dir_of(dir: &Path) -> Option<PathBuf> {
    let dot_git = dir.join(".git");
    if dot_git.is_dir() {
        return Some(dot_git);
    }
    let text = std::fs::read_to_string(&dot_git).ok()?;
    let target = Path::new(text.trim().strip_prefix("gitdir:")?.trim());
    Some(dir.join(target))
}

/// A worktree's shared git directory (its `commondir` file), where its branches live.
fn common_dir(git_dir: &Path) -> PathBuf {
    std::fs::read_to_string(git_dir.join("commondir"))
        .map_or_else(|_| git_dir.to_path_buf(), |c| git_dir.join(c.trim()))
}

/// `<hash> <ref>` per line, git's packed-refs format.
fn packed_ref(git_dir: &Path, reference: &str) -> Option<String> {
    let packed = std::fs::read_to_string(git_dir.join("packed-refs")).ok()?;
    packed.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let hash = parts.next()?;
        (parts.next()? == reference).then(|| hash.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("oag-revision-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_submodule_folder_names_the_commit_its_gitdir_file_points_at() {
        let root = scratch("submodule");
        let modules = root.join("super/.git/modules/official");
        std::fs::create_dir_all(&modules).unwrap();
        std::fs::write(
            modules.join("HEAD"),
            "0d2c1da0123456789abcdef0123456789abcdef0\n",
        )
        .unwrap();
        let checkout = root.join("super/assets/mods/official");
        std::fs::create_dir_all(checkout.join("slipstream")).unwrap();
        std::fs::write(
            checkout.join(".git"),
            "gitdir: ../../../.git/modules/official\n",
        )
        .unwrap();
        assert_eq!(
            checkout_head(&checkout.join("slipstream")).as_deref(),
            Some("0d2c1da")
        );
    }

    #[test]
    fn a_branch_resolves_through_its_ref_or_packed_refs() {
        let root = scratch("branch");
        let git = root.join(".git");
        std::fs::create_dir_all(git.join("refs/heads")).unwrap();
        std::fs::write(git.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        std::fs::write(
            git.join("packed-refs"),
            "# pack-refs with: peeled\nb71dfaf0123456789abcdef0123456789abcdef0 refs/heads/main\n",
        )
        .unwrap();
        assert_eq!(checkout_head(&root).as_deref(), Some("b71dfaf"));
        std::fs::write(
            git.join("refs/heads/main"),
            "c6cd38a0123456789abcdef0123456789abcdef0\n",
        )
        .unwrap();
        assert_eq!(checkout_head(&root).as_deref(), Some("c6cd38a"));
    }
}
