//! Unified diff parsing, hunk/line patch building and conflict-marker parsing.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LineKind {
    Ctx,
    Add,
    Del,
}

#[derive(Clone, Debug)]
pub struct DLine {
    pub kind: LineKind,
    pub text: String,
    pub old_no: Option<u32>,
    pub new_no: Option<u32>,
    pub no_newline: bool,
}

#[derive(Clone, Debug)]
pub struct Hunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub section: String,
    pub lines: Vec<DLine>,
}

impl Hunk {
    pub fn changed(&self) -> usize {
        self.lines.iter().filter(|l| l.kind != LineKind::Ctx).count()
    }
}

#[derive(Clone, Debug, Default)]
pub struct FileDiff {
    pub header: Vec<String>,
    pub hunks: Vec<Hunk>,
    pub binary: bool,
    pub is_new: bool,
    pub is_deleted: bool,
    pub rename_from: Option<String>,
    pub rename_to: Option<String>,
    pub additions: u32,
    pub deletions: u32,
}

fn parse_hunk_header(l: &str) -> Option<(u32, u32, u32, u32, String)> {
    let rest = l.strip_prefix("@@ -")?;
    let (old, rest) = rest.split_once(" +")?;
    let (new, section) = rest.split_once(" @@")?;
    let pr = |s: &str| -> Option<(u32, u32)> {
        match s.split_once(',') {
            Some((a, b)) => Some((a.parse().ok()?, b.parse().ok()?)),
            None => Some((s.parse().ok()?, 1)),
        }
    };
    let (os, ol) = pr(old)?;
    let (ns, nl) = pr(new)?;
    Some((os, ol, ns, nl, section.trim().to_string()))
}

pub fn parse_diff(raw: &str) -> FileDiff {
    let mut fd = FileDiff::default();
    let mut cur: Option<Hunk> = None;
    let (mut old_no, mut new_no) = (0u32, 0u32);
    for l in raw.split('\n') {
        if let Some((os, ol, ns, nl, sec)) = parse_hunk_header(l) {
            if let Some(h) = cur.take() {
                fd.hunks.push(h);
            }
            cur = Some(Hunk { old_start: os, old_lines: ol, new_start: ns, new_lines: nl, section: sec, lines: vec![] });
            old_no = os;
            new_no = ns;
            continue;
        }
        match cur.as_mut() {
            None => {
                if l.is_empty() {
                    continue;
                }
                fd.header.push(l.to_string());
                if l.starts_with("new file mode") {
                    fd.is_new = true;
                } else if l.starts_with("deleted file mode") {
                    fd.is_deleted = true;
                } else if l.starts_with("Binary files") || l.starts_with("GIT binary patch") {
                    fd.binary = true;
                } else if let Some(r) = l.strip_prefix("rename from ") {
                    fd.rename_from = Some(r.to_string());
                } else if let Some(r) = l.strip_prefix("rename to ") {
                    fd.rename_to = Some(r.to_string());
                }
            }
            Some(h) => {
                let mut ch = l.chars();
                match ch.next() {
                    Some('+') => {
                        h.lines.push(DLine { kind: LineKind::Add, text: l[1..].to_string(), old_no: None, new_no: Some(new_no), no_newline: false });
                        new_no += 1;
                        fd.additions += 1;
                    }
                    Some('-') => {
                        h.lines.push(DLine { kind: LineKind::Del, text: l[1..].to_string(), old_no: Some(old_no), new_no: None, no_newline: false });
                        old_no += 1;
                        fd.deletions += 1;
                    }
                    Some('\\') => {
                        if let Some(p) = h.lines.last_mut() {
                            p.no_newline = true;
                        }
                    }
                    Some(' ') => {
                        h.lines.push(DLine { kind: LineKind::Ctx, text: l[1..].to_string(), old_no: Some(old_no), new_no: Some(new_no), no_newline: false });
                        old_no += 1;
                        new_no += 1;
                    }
                    _ => {} // trailing empty line
                }
            }
        }
    }
    if let Some(h) = cur.take() {
        fd.hunks.push(h);
    }
    fd
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PatchMode {
    /// stage: apply to the index
    Stage,
    /// unstage: reverse-apply to the index
    Unstage,
}

/// Patch for ONE hunk containing only `selected` changed lines (indexes into `hunk.lines`).
pub fn build_patch(fd: &FileDiff, hunk_idx: usize, selected: &[usize], mode: PatchMode) -> Option<String> {
    let h = fd.hunks.get(hunk_idx)?;
    let mut body: Vec<String> = vec![];
    let (mut o, mut n) = (0u32, 0u32);
    let mut changed = false;
    let mut last_kept;
    for (i, l) in h.lines.iter().enumerate() {
        let sel = selected.contains(&i);
        match l.kind {
            LineKind::Ctx => {
                body.push(format!(" {}", l.text));
                o += 1;
                n += 1;
                last_kept = true;
            }
            LineKind::Del => {
                if sel {
                    body.push(format!("-{}", l.text));
                    o += 1;
                    changed = true;
                    last_kept = true;
                } else if mode == PatchMode::Stage {
                    // keep as context: it still exists in the index
                    body.push(format!(" {}", l.text));
                    o += 1;
                    n += 1;
                    last_kept = true;
                } else {
                    last_kept = false; // absent from the index: omit entirely
                }
            }
            LineKind::Add => {
                if sel {
                    body.push(format!("+{}", l.text));
                    n += 1;
                    changed = true;
                    last_kept = true;
                } else if mode == PatchMode::Unstage {
                    // stays in the index: context
                    body.push(format!(" {}", l.text));
                    o += 1;
                    n += 1;
                    last_kept = true;
                } else {
                    last_kept = false;
                }
            }
        }
        if l.no_newline && last_kept {
            body.push("\\ No newline at end of file".to_string());
        }
    }
    if !changed {
        return None;
    }
    let old_start = h.old_start;
    let new_start = h.new_start;
    let mut out: Vec<String> = fd.header.clone();
    out.push(format!("@@ -{},{} +{},{} @@{}", old_start, o, new_start, n, if h.section.is_empty() { String::new() } else { format!(" {}", h.section) }));
    out.extend(body);
    Some(out.join("\n") + "\n")
}

pub fn all_lines(h: &Hunk) -> Vec<usize> {
    h.lines.iter().enumerate().filter(|(_, l)| l.kind != LineKind::Ctx).map(|(i, _)| i).collect()
}

/// Changed middle part of a removed/added pair, in char indexes.
pub fn intra_range(a: &str, b: &str) -> (Option<(usize, usize)>, Option<(usize, usize)>) {
    if a == b {
        return (None, None);
    }
    let ac: Vec<char> = a.chars().collect();
    let bc: Vec<char> = b.chars().collect();
    let max = ac.len().min(bc.len());
    let mut s = 0;
    while s < max && ac[s] == bc[s] {
        s += 1;
    }
    let (mut ea, mut eb) = (ac.len(), bc.len());
    while ea > s && eb > s && ac[ea - 1] == bc[eb - 1] {
        ea -= 1;
        eb -= 1;
    }
    let matched = s + (ac.len() - ea);
    if (matched as f32) < (ac.len().min(bc.len()) as f32) * 0.25 {
        return (None, None);
    }
    (if ea > s { Some((s, ea)) } else { None }, if eb > s { Some((s, eb)) } else { None })
}

#[derive(Clone, Debug)]
pub enum Seg {
    Text(String),
    Conflict(Conflict),
}

#[derive(Clone, Debug, Default)]
pub struct Conflict {
    pub ours: String,
    pub theirs: String,
    pub base: Option<String>,
    /// resolved text, when the user decided
    pub result: Option<String>,
    pub choice: Option<char>, // 'o' 't' 'b' 'm'
}

pub fn parse_conflicts(src: &str) -> Vec<Seg> {
    let lines: Vec<&str> = src.split('\n').collect();
    let mut segs: Vec<Seg> = vec![];
    let mut buf: Vec<&str> = vec![];
    let mut i = 0;
    while i < lines.len() {
        let l = lines[i].trim_end_matches('\r');
        if l.starts_with("<<<<<<< ") {
            if !buf.is_empty() {
                segs.push(Seg::Text(buf.join("\n") + "\n"));
                buf.clear();
            }
            let mut c = Conflict::default();
            let (mut ours, mut base, mut theirs): (Vec<&str>, Vec<&str>, Vec<&str>) = (vec![], vec![], vec![]);
            let mut mode = 0;
            i += 1;
            while i < lines.len() {
                let x = lines[i];
                let xt = x.trim_end_matches('\r');
                if xt.starts_with("||||||| ") {
                    mode = 1;
                } else if xt == "=======" {
                    mode = 2;
                } else if xt.starts_with(">>>>>>> ") {
                    break;
                } else if mode == 0 {
                    ours.push(x);
                } else if mode == 1 {
                    base.push(x);
                } else {
                    theirs.push(x);
                }
                i += 1;
            }
            let j = |v: &Vec<&str>| if v.is_empty() { String::new() } else { v.join("\n") + "\n" };
            c.ours = j(&ours);
            c.theirs = j(&theirs);
            c.base = if base.is_empty() { None } else { Some(j(&base)) };
            segs.push(Seg::Conflict(c));
            i += 1;
            continue;
        }
        buf.push(lines[i]);
        i += 1;
    }
    if !buf.is_empty() {
        segs.push(Seg::Text(buf.join("\n")));
    }
    segs
}

pub fn compose_conflicts(segs: &[Seg]) -> String {
    segs.iter()
        .map(|s| match s {
            Seg::Text(t) => t.clone(),
            Seg::Conflict(c) => match (&c.result, c.choice) {
                (Some(r), _) => {
                    if r.is_empty() || r.ends_with('\n') {
                        r.clone()
                    } else {
                        format!("{r}\n")
                    }
                }
                (None, Some('o')) => c.ours.clone(),
                (None, Some('t')) => c.theirs.clone(),
                (None, Some('b')) => format!("{}{}", c.ours, c.theirs),
                _ => c.ours.clone(),
            },
        })
        .collect()
}
