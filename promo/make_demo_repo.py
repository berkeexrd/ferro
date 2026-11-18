"""Builds the demo repository used for the promo footage (fictional project "aurora")."""
import os, subprocess, shutil, sys, textwrap

ROOT = sys.argv[1]
REPO = os.path.join(ROOT, "aurora")
REMOTE = os.path.join(ROOT, "aurora-remote.git")
for d in (REPO, REMOTE):
    shutil.rmtree(d, ignore_errors=True)
os.makedirs(REPO)
subprocess.run(["git", "init", "-q", "--bare", "-b", "main", REMOTE], check=True)

AUTH = {
    "elif": ("Elif Yilmaz", "elif@aurora.dev"),
    "mert": ("Mert Kaya", "mert@aurora.dev"),
    "deniz": ("Deniz Aksoy", "deniz@aurora.dev"),
}
day = [0]


def git(*a, who="elif", d=None, check=True):
    name, email = AUTH[who]
    env = dict(os.environ, GIT_AUTHOR_NAME=name, GIT_AUTHOR_EMAIL=email, GIT_COMMITTER_NAME=name, GIT_COMMITTER_EMAIL=email)
    if d is not None:
        stamp = f"2026-11-{d:02d}T{10 + (d * 7) % 8:02d}:{(d * 13) % 60:02d}:00+03:00"
        env["GIT_AUTHOR_DATE"] = stamp
        env["GIT_COMMITTER_DATE"] = stamp
    return subprocess.run(["git", "-C", REPO] + list(a), env=env, check=check, capture_output=True, text=True)


def write(path, text):
    p = os.path.join(REPO, path)
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, "w", encoding="utf-8", newline="\n") as f:
        f.write(textwrap.dedent(text).lstrip("\n"))


def commit(msg, who="elif", d=1):
    git("add", "-A")
    git("commit", "-q", "-m", msg, who=who, d=d)


STATUS_V1 = """
    use std::fs;
    use std::path::{Path, PathBuf};

    pub struct StatusEntry {
        pub path: PathBuf,
        pub kind: EntryKind,
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum EntryKind {
        Modified,
        Added,
        Deleted,
        Untracked,
        Unknown,
    }

    pub struct StatusScanner {
        index: Vec<PathBuf>,
        chunk: usize,
    }

    impl StatusScanner {
        pub fn new(index: Vec<PathBuf>) -> Self {
            Self { index, chunk: 256 }
        }

        pub fn chunk_size(&self) -> usize {
            self.chunk
        }

        pub fn scan(&self, root: &Path) -> Result<Vec<StatusEntry>, std::io::Error> {
            let paths = self.index.iter();
            let mut out = Vec::with_capacity(self.index.len());
            for path in paths {
                out.push(self.stat_one(root, path)?);
            }
            Ok(out)
        }

        fn stat_one(&self, root: &Path, path: &Path) -> Result<StatusEntry, std::io::Error> {
            let meta = fs::symlink_metadata(root.join(path))?;
            let kind = classify(&meta);
            Ok(StatusEntry { path: path.to_owned(), kind })
        }
    }

    fn classify(meta: &fs::Metadata) -> EntryKind {
        if meta.is_file() {
            EntryKind::Modified
        } else {
            EntryKind::Unknown
        }
    }
"""

THEME = """
    //! Design tokens.
    #[derive(Clone, Copy)]
    pub struct Rgb(pub u8, pub u8, pub u8);

    pub const BG: Rgb = Rgb(0x15, 0x14, 0x12);
    pub const ACCENT: Rgb = Rgb(0xE5, 0x8A, 0x55);
    pub const ADDED: Rgb = Rgb(0x7F, 0xD3, 0xA5);
    pub const DELETED: Rgb = Rgb(0xF2, 0x9B, 0x9B);

    pub fn blend(a: Rgb, b: Rgb, t: f32) -> Rgb {
        let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
        Rgb(m(a.0, b.0), m(a.1, b.1), m(a.2, b.2))
    }
"""

DIFF_V1 = """
    #[derive(Debug, Clone, PartialEq)]
    pub enum Line {
        Context(String),
        Added(String),
        Removed(String),
    }

    pub struct Hunk {
        pub header: String,
        pub lines: Vec<Line>,
    }

    impl Hunk {
        pub fn changed(&self) -> usize {
            self.lines.iter().filter(|l| !matches!(l, Line::Context(_))).count()
        }
    }

    pub fn parse(patch: &str) -> Vec<Hunk> {
        let mut hunks: Vec<Hunk> = Vec::new();
        for raw in patch.lines() {
            if raw.starts_with("@@") {
                hunks.push(Hunk { header: raw.to_string(), lines: Vec::new() });
            } else if let Some(h) = hunks.last_mut() {
                let line = match raw.chars().next() {
                    Some('+') => Line::Added(raw[1..].to_string()),
                    Some('-') => Line::Removed(raw[1..].to_string()),
                    _ => Line::Context(raw.get(1..).unwrap_or("").to_string()),
                };
                h.lines.push(line);
            }
        }
        hunks
    }
"""

GRAPH = """
    /// Assigns a lane to every commit so the history can be drawn as a graph.
    pub fn lanes(commits: &[(String, Vec<String>)]) -> Vec<usize> {
        let mut active: Vec<Option<String>> = Vec::new();
        let mut out = Vec::with_capacity(commits.len());
        for (sha, parents) in commits {
            let lane = active.iter().position(|l| l.as_deref() == Some(sha.as_str())).unwrap_or_else(|| {
                active.push(None);
                active.len() - 1
            });
            active[lane] = parents.first().cloned();
            out.push(lane);
        }
        out
    }
"""

MAIN = """
    mod diff;
    mod graph;
    mod status;
    mod theme;

    fn main() {
        let scanner = status::StatusScanner::new(Vec::new());
        println!("aurora ready, chunk size {}", scanner.chunk_size());
    }
"""

git("init", "-q", "-b", "main")
git("config", "core.autocrlf", "false")
write("Cargo.toml", '[package]\nname = "aurora"\nversion = "0.1.0"\nedition = "2021"\n')
write("README.md", "# aurora\n\nA tiny Git status engine.\n")
write(".gitignore", "target/\n")
commit("Project skeleton", "elif", 1)
write("src/main.rs", MAIN)
write("src/status.rs", STATUS_V1)
commit("Add the status scanner", "elif", 2)
write("src/theme.rs", THEME)
commit("Move colours into design tokens", "mert", 3)
write("src/diff.rs", DIFF_V1)
commit("Parse unified diffs into hunks", "elif", 4)
git("tag", "v0.1.0")

git("switch", "-q", "-c", "fix/path-separator")
write("src/status.rs", STATUS_V1.replace("root.join(path)", "root.join(path.components().collect::<PathBuf>())"))
commit("Normalise Windows path separators", "deniz", 5)
git("switch", "-q", "main")
git("merge", "-q", "--no-ff", "fix/path-separator", "-m", "Merge fix/path-separator", who="elif", d=6)

git("switch", "-q", "-c", "feature/commit-graph")
write("src/graph.rs", GRAPH)
commit("Lane assignment for the history graph", "mert", 7)
write("src/graph.rs", GRAPH + "\n    pub fn max_lanes(lanes: &[usize]) -> usize {\n        lanes.iter().copied().max().map_or(1, |m| m + 1)\n    }\n")
commit("Track the widest row", "mert", 8)
git("switch", "-q", "main")
write("README.md", "# aurora\n\nA tiny Git status engine.\n\n## Install\n\n    cargo install aurora\n")
commit("Document installation", "elif", 9)
git("merge", "-q", "--no-ff", "feature/commit-graph", "-m", "Merge feature/commit-graph", who="elif", d=10)
git("tag", "-a", "v0.2.0", "-m", "v0.2.0")

write("src/diff.rs", DIFF_V1.replace("pub fn changed", "pub fn changed_lines").replace(".changed()", ".changed_lines()"))
commit("Rename Hunk::changed to changed_lines", "deniz", 11)
write("src/theme.rs", THEME + "\n    pub const MODIFIED: Rgb = Rgb(0xE8, 0xBE, 0x55);\n")
commit("Add the modified colour", "mert", 12)
git("switch", "-q", "-c", "release/0.3")
write("Cargo.toml", '[package]\nname = "aurora"\nversion = "0.3.0"\nedition = "2021"\n')
commit("Bump version to 0.3.0", "elif", 13)
git("tag", "-a", "v0.3.0", "-m", "v0.3.0")
git("switch", "-q", "main")
write("src/main.rs", MAIN.replace("println!(\"aurora ready", "println!(\"aurora is ready"))
commit("Polish the start-up message", "elif", 14)

git("switch", "-q", "-c", "feature/parallel-scan")
git("remote", "add", "origin", REMOTE)
git("push", "-q", "-u", "origin", "main")
git("push", "-q", "origin", "feature/parallel-scan")
git("push", "-q", "origin", "--tags")
write("src/status.rs", STATUS_V1.replace("chunk: 256", "chunk: 128"))
commit("Smaller default chunk size", "elif", 15)
write("tests/status.rs", "#[test]\nfn chunk_size_is_positive() {\n    assert!(128 > 0);\n}\n")
commit("Add a smoke test for chunking", "elif", 16)

write("src/status.rs", STATUS_V1.replace("chunk: 256", "chunk: 1024"))
git("stash", "push", "-q", "-m", "experiment: bigger chunk")
git("checkout", "-q", "--", ".")
# ---- working tree: three hunks in status.rs, a new file, a deleted file, a small edit elsewhere
new_status = STATUS_V1.replace("chunk: 256", "chunk: 128")
new_status = new_status.replace(
    """        pub fn scan(&self, root: &Path) -> Result<Vec<StatusEntry>, std::io::Error> {
            let paths = self.index.iter();
            let mut out = Vec::with_capacity(self.index.len());
            for path in paths {
                out.push(self.stat_one(root, path)?);
            }
            Ok(out)
        }""",
    """        pub fn scan(&self, root: &Path) -> Result<Vec<StatusEntry>, std::io::Error> {
            // Split the index into chunks and stat them on worker threads.
            let chunks = self.index.chunks(self.chunk_size());
            let out = std::thread::scope(|s| -> Result<Vec<StatusEntry>, std::io::Error> {
                let handles: Vec<_> = chunks
                    .map(|c| s.spawn(move || self.stat_chunk(root, c)))
                    .collect();
                merge_results(handles)
            })?;
            Ok(out)
        }

        fn stat_chunk(&self, root: &Path, chunk: &[PathBuf]) -> Result<Vec<StatusEntry>, std::io::Error> {
            chunk.iter().map(|p| self.stat_one(root, p)).collect()
        }""",
)
new_status = new_status.replace("let kind = classify(&meta);", "let kind = classify(&meta).unwrap_or(EntryKind::Unknown);")
new_status = new_status.replace(
    """    fn classify(meta: &fs::Metadata) -> EntryKind {
        if meta.is_file() {
            EntryKind::Modified
        } else {
            EntryKind::Unknown
        }
    }""",
    """    fn classify(meta: &fs::Metadata) -> Option<EntryKind> {
        if meta.is_file() {
            Some(EntryKind::Modified)
        } else if meta.is_dir() {
            Some(EntryKind::Untracked)
        } else {
            None
        }
    }

    fn merge_results<T>(handles: Vec<std::thread::ScopedJoinHandle<'_, Result<Vec<T>, std::io::Error>>>) -> Result<Vec<T>, std::io::Error> {
        let mut out = Vec::new();
        for h in handles {
            out.extend(h.join().expect("worker panicked")?);
        }
        Ok(out)
    }""",
)
# the replacements above are textual on the dedented source; status.rs on disk has no indentation, so dedent them
write("src/status.rs", new_status)
write("src/stats.rs", "pub struct Stats {\n    pub files: usize,\n    pub millis: u128,\n}\n\nimpl Stats {\n    pub fn per_second(&self) -> f64 {\n        self.files as f64 / (self.millis.max(1) as f64 / 1000.0)\n    }\n}\n")
os.remove(os.path.join(REPO, "src", "graph.rs"))
write("README.md", "# aurora\n\nA tiny Git status engine.\n\n## Install\n\n    cargo install aurora\n\n## Speed\n\nStatus scans run in parallel.\n")
print(git("log", "--oneline", "--graph", "--all").stdout)
print(git("status", "--short").stdout)
