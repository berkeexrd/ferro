//! Turkish ⇄ English. The UI is written in Turkish; English is produced at draw time by exact-match
//! and `{}`-pattern lookup (placeholders may be written `{}` or `{name}`). Results are memoised.

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize, Default)]
pub enum Lang {
    #[default]
    Auto,
    Tr,
    En,
}

static CUR: AtomicU8 = AtomicU8::new(0); // 0 = Tr, 1 = En
static GEN: AtomicU8 = AtomicU8::new(0);

pub fn is_en() -> bool {
    CUR.load(Ordering::Relaxed) == 1
}

/// Resolve `Auto` from the Windows display language.
pub fn detect() -> Lang {
    #[cfg(windows)]
    {
        extern "system" {
            fn GetUserDefaultLocaleName(buf: *mut u16, len: i32) -> i32;
        }
        let mut b = [0u16; 85];
        let n = unsafe { GetUserDefaultLocaleName(b.as_mut_ptr(), 85) };
        if n > 0 {
            let s = String::from_utf16_lossy(&b[..(n as usize).saturating_sub(1)]).to_lowercase();
            return if s.starts_with("tr") { Lang::Tr } else { Lang::En };
        }
    }
    let l = std::env::var("LANG").unwrap_or_default().to_lowercase();
    if l.starts_with("tr") { Lang::Tr } else { Lang::En }
}

pub fn set(l: Lang) {
    let eff = if l == Lang::Auto { detect() } else { l };
    CUR.store(if eff == Lang::En { 1 } else { 0 }, Ordering::Relaxed);
    GEN.fetch_add(1, Ordering::Relaxed);
}

struct Pat {
    segs: Vec<String>,
    en: Vec<String>,
}

struct Tables {
    exact: HashMap<String, &'static str>,
    pats: Vec<Pat>,
}

fn norm_placeholders(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '{' {
            let mut j = String::new();
            let mut closed = false;
            for d in it.by_ref() {
                if d == '}' {
                    closed = true;
                    break;
                }
                j.push(d);
            }
            if closed && j.chars().all(|x| x.is_alphanumeric() || x == '_' || x == '.' ) {
                out.push_str("{}");
            } else {
                out.push('{');
                out.push_str(&j);
                if closed {
                    out.push('}');
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn build() -> Tables {
    let mut exact = HashMap::new();
    let mut pats: Vec<Pat> = vec![];
    for (tr, en) in TABLE {
        let n = norm_placeholders(tr);
        if n.contains("{}") {
            pats.push(Pat { segs: n.split("{}").map(|x| x.to_string()).collect(), en: norm_placeholders(en).split("{}").map(|x| x.to_string()).collect() });
        } else {
            exact.insert(n, *en);
        }
    }
    pats.sort_by_key(|p| std::cmp::Reverse(p.segs.iter().map(|s| s.chars().count()).sum::<usize>()));
    Tables { exact, pats }
}

thread_local! {
    static TBL: Tables = build();
    static MEMO: RefCell<(u8, HashMap<String, String>)> = RefCell::new((0, HashMap::new()));
}

fn match_pat(p: &Pat, s: &str, tb: &Tables, depth: u8) -> Option<String> {
    let first = &p.segs[0];
    let last = p.segs.last().unwrap();
    if !s.starts_with(first.as_str()) || s.len() < first.len() + last.len() || !s.ends_with(last.as_str()) {
        return None;
    }
    let mut caps: Vec<&str> = vec![];
    let mut pos = first.len();
    let end = s.len() - last.len();
    for seg in &p.segs[1..p.segs.len() - 1] {
        let rel = if seg.is_empty() { Some(0) } else { s[pos..end].find(seg.as_str()) }?;
        caps.push(&s[pos..pos + rel]);
        pos += rel + seg.len();
    }
    if pos > end {
        return None;
    }
    caps.push(&s[pos..end]);
    if caps.len() + 1 != p.segs.len() || caps.len() + 1 != p.en.len() {
        return None;
    }
    let mut out = p.en[0].clone();
    for (i, c) in caps.iter().enumerate() {
        out.push_str(&if depth < 3 { lookup(c, tb, depth + 1) } else { c.to_string() });
        out.push_str(&p.en[i + 1]);
    }
    Some(out)
}

fn lookup(s: &str, tb: &Tables, depth: u8) -> String {
    if s.is_empty() {
        return String::new();
    }
    if let Some(e) = tb.exact.get(s) {
        return e.to_string();
    }
    for p in &tb.pats {
        if let Some(r) = match_pat(p, s, tb, depth) {
            return r;
        }
    }
    s.to_string()
}

/// Translate a UI string for the current language (identity for Turkish).
pub fn t(s: &str) -> Cow<'_, str> {
    if !is_en() || s.is_empty() {
        return Cow::Borrowed(s);
    }
    let gen = GEN.load(Ordering::Relaxed);
    MEMO.with(|m| {
        let mut m = m.borrow_mut();
        if m.0 != gen || m.1.len() > 6000 {
            m.0 = gen;
            m.1.clear();
        }
        if let Some(v) = m.1.get(s) {
            return Cow::Owned(v.clone());
        }
        let r = fix_plural(&TBL.with(|tb| lookup(s, tb, 0)));
        m.1.insert(s.to_string(), r.clone());
        Cow::Owned(r)
    })
}

/// "1 files" → "1 file", "3 file(s)" → "3 files".
fn fix_plural(s: &str) -> String {
    if !(s.contains("(s)") || s.contains("1 ")) {
        return s.to_string();
    }
    let words: Vec<&str> = s.split(' ').collect();
    let mut out: Vec<String> = Vec::with_capacity(words.len());
    for (i, w) in words.iter().enumerate() {
        let prev_one = i > 0 && words[i - 1] == "1";
        let mut w = w.to_string();
        if w.contains("(s)") {
            w = w.replace("(s)", if prev_one { "" } else { "s" });
        } else if prev_one {
            let (core, tail) = match w.find(|c: char| !c.is_alphabetic()) {
                Some(k) => (w[..k].to_string(), w[k..].to_string()),
                None => (w.clone(), String::new()),
            };
            let sing = match core.as_str() {
                "files" => "file",
                "commits" => "commit",
                "entries" => "entry",
                "operations" => "operation",
                "lines" => "line",
                "hunks" => "hunk",
                "conflicts" => "conflict",
                "repositories" => "repository",
                "tags" => "tag",
                _ => "",
            };
            if !sing.is_empty() {
                w = format!("{sing}{tail}");
            }
        }
        out.push(w);
    }
    out.join(" ")
}

pub fn rt(s: impl AsRef<str>) -> eframe::egui::RichText {
    eframe::egui::RichText::new(t(s.as_ref()).into_owned())
}

pub trait PainterT {
    fn text_t(&self, pos: eframe::egui::Pos2, anchor: eframe::egui::Align2, text: impl ToString, font: eframe::egui::FontId, color: eframe::egui::Color32) -> eframe::egui::Rect;
}

impl PainterT for eframe::egui::Painter {
    fn text_t(&self, pos: eframe::egui::Pos2, anchor: eframe::egui::Align2, text: impl ToString, font: eframe::egui::FontId, color: eframe::egui::Color32) -> eframe::egui::Rect {
        let s = text.to_string();
        self.text(pos, anchor, t(&s), font, color)
    }
}

pub trait RespT {
    fn on_hover_text_t(self, text: &str) -> Self;
}

impl RespT for eframe::egui::Response {
    fn on_hover_text_t(self, text: &str) -> Self {
        self.on_hover_text(t(text).into_owned())
    }
}

/// (Turkish, English). `{}` / `{name}` are placeholders.
const TABLE: &[(&str, &str)] = &[
    (" · henüz gönderilmedi", " · not pushed yet"),
    (" · {n_partial} kısmen", " · {} partial"),
    ("(boş)", "(empty)"),
    ("+ Yardımcı yazar ekle", "+ Add co-author"),
    ("− Yardımcı yazar", "− Co-author"),
    (", {hunks} hunk", ", {} hunk(s)"),
    ("1 hunk hazırlandı", "1 hunk staged"),
    ("1 hunk hazırlıktan çıkarıldı", "1 hunk unstaged"),
    ("Ad Soyad", "Full name"),
    ("Ad Soyad <e-posta>", "Full Name <email>"),
    ("Amend geri alınamaz; reflog'da durur", "Amend can't be undone; it stays in the reflog"),
    ("Anlık görüntü alınamadı", "Snapshot unavailable"),
    ("Ara", "Search"),
    ("Arayüz ölçeği", "Interface scale"),
    ("Ayarlar", "Settings"),
    ("Ayarlar (Ctrl ,)", "Settings (Ctrl ,)"),
    ("Ayrıntı için bir PR seç", "Select a PR for details"),
    ("Ayrıntı için bir commit seç", "Select a commit for details"),
    ("Ayrıntı için bir stash seç", "Select a stash for details"),
    ("Açık", "Light"),
    ("Açık / koyu temayı değiştir", "Toggle light / dark theme"),
    ("Açık pull request yok", "No open pull requests"),
    ("Açıklama (isteğe bağlı)", "Description (optional)"),
    ("Açıklama yok.", "No description."),
    ("Bağlı hesap yok. Token girmen gerekmez: tarayıcıda onaylarsın.", "No account connected. No token needed: just approve in your browser."),
    ("Bilgisayarındaki bir klasörü seç", "Pick a folder on this computer"),
    ("Bir commit özeti yaz", "Write a commit summary"),
    ("Bir depo aç ve çalışmaya başla. Her işlem geri alınabilir, her işlemin git karşılığı görünür.", "Open a repository and get to work. Every action is undoable and every git command is visible."),
    ("Bir dosya seç", "Select a file"),
    ("Bir dosyayı değiştirince burada görünür", "Changed files will show up here"),
    ("Birleştir: {b}", "Merge: {}"),
    ("Birleştirildi: {name}", "Merged: {}"),
    ("Birleştirilmemiş commit'ler varsa git silmeyi reddeder.", "Git refuses to delete the branch if it has unmerged commits."),
    ("Birleştirme çatışma verdi; çözüp tamamla", "The merge hit conflicts; resolve them and finish"),
    ("Bizim", "Ours"),
    ("Bizimki", "Ours"),
    ("Boş klasörde git init", "git init in an empty folder"),
    ("Branch'i yayınla", "Publish branch"),
    ("Bu depoda uzak depo yok", "This repository has no remote"),
    ("Bu deponun GitHub uzak adresi yok.", "This repository has no GitHub remote."),
    ("Bu filtrede dosya yok", "No files in this filter"),
    ("Bu hunk'taki değişiklikler çalışma alanından kaldırılır. İşlem günlüğünden geri alınamaz.", "The changes in this hunk are removed from your working tree. This can't be undone from the operation log."),
    ("Bu noktaya dön", "Return to this point"),
    ("Bu noktaya dönülemez: araya geri alınamayan bir işlem girmiş", "Can't return here: a non-undoable operation is in between"),
    ("Bu oturumda henüz işlem yok", "No operations in this session yet"),
    ("Buradan dal aç", "Branch from here"),
    ("Buraya dön", "Return here"),
    ("Commit oluşturuldu · {}", "Commit created · {}"),
    ("Commit ve push", "Commit and push"),
    ("Commit · hazır dosya yok", "Commit · nothing staged"),
    ("Commit · {} dosya{}", "Commit · {} file(s){}"),
    ("Commit özeti", "Commit summary"),
    ("Commit'i düzelt", "Amend commit"),
    ("Commit işlemi: {k}", "Commit action: {}"),
    ("DAL", "BRANCH"),
    ("DALLAR", "BRANCHES"),
    ("DEPO", "REPO"),
    ("DEPOLAR", "REPOSITORIES"),
    ("DOSYALAR", "FILES"),
    ("KOMUTLAR", "COMMANDS"),
    ("Dal ara", "Search branches"),
    ("Dal ara veya yeni dal adı yaz", "Search a branch or type a new name"),
    ("Dal bulunamadı", "No branch found"),
    ("Dal değiştirildi: {prev} → {name}", "Switched branch: {} → {}"),
    ("Dal değiştir…", "Switch branch…"),
    ("Dal oluşturuldu: {name}", "Branch created: {}"),
    ("Dal silindi", "Branch deleted"),
    ("Dala geç: {b}", "Switch to: {}"),
    ("Dallar", "Branches"),
    ("Dallar sekmesine git", "Go to Branches"),
    ("Dalı al", "Check out branch"),
    ("Dalı sil", "Delete branch"),
    ("Dalı sil: {b}", "Delete branch: {}"),
    ("Depo adresi", "Repository URL"),
    ("Depo ara", "Search repositories"),
    ("Depo klasörünü seç", "Choose the repository folder"),
    ("Depo klonla", "Clone repository"),
    ("Depo klonla…", "Clone repository…"),
    ("Depo seç", "Select repo"),
    ("Depolarının listesi", "Your repositories"),
    ("Depoyu aç: {p}", "Open repository: {}"),
    ("Depoyu kapat", "Close repository"),
    ("Desteklenen editör bulunamadı", "No supported editor found"),
    ("Değişiklik yok; doğrudan geçilir", "No changes; switches directly"),
    ("Değişiklikler", "Changes"),
    ("Değişiklikler atıldı", "Changes discarded"),
    ("Değişiklikler gönderildi", "Changes pushed"),
    ("Değişiklikler sekmesine git", "Go to Changes"),
    ("Değişiklikler stash'lendi", "Changes stashed"),
    ("Değişiklikleri at", "Discard changes"),
    ("Değişiklikleri at: {f}", "Discard changes: {}"),
    ("Değişiklikleri at…", "Discard changes…"),
    ("Değişiklikleri stash'le", "Stash changes"),
    ("Değişiklikleri taşı", "Bring my changes"),
    ("Diff bağlam satırı", "Diff context lines"),
    ("Dosya Gezgini'nde göster", "Show in File Explorer"),
    ("Dosya yeniden adlandırıldı, içerik değişmedi", "File renamed, contents unchanged"),
    ("Dosya çözüldü olarak işaretlendi", "File marked as resolved"),
    ("Dosyanın tamamını bağlam olarak göster", "Show the whole file as context"),
    ("Dosyayı göster: {f}", "Show file: {}"),
    ("Dosyayı hazırlıktan çıkar", "Unstage file"),
    ("Düzelt", "Amend"),
    ("Düzenle", "Edit"),
    ("ETİKETLER · {}", "TAGS · {}"),
    ("Editörde aç", "Open in editor"),
    ("Esc", "Esc"),
    ("Etiket oluştur", "Create tag"),
    ("Etiket oluşturuldu: {name}", "Tag created: {}"),
    ("Etiketle", "Tag"),
    ("Etkin yap", "Make active"),
    ("Eşleşen commit yok", "No matching commits"),
    ("Fark görmek için bir dosya seç", "Select a file to see its diff"),
    ("Ferro'ya hoş geldin", "Welcome to Ferro"),
    ("Fetch tamamlandı", "Fetch complete"),
    ("Force push (güvenli)", "Force push (safe)"),
    ("Force push yapılsın mı?", "Force push?"),
    ("Force push öncesi onay iste", "Confirm before force push"),
    ("Geri al", "Undo"),
    ("Geri al (revert)", "Revert"),
    ("Geri alınacak işlem yok", "Nothing to undo"),
    ("Geri alınamadı: {e}", "Couldn't undo: {}"),
    ("Geri alındı (revert): {short}", "Reverted: {}"),
    ("Geri alındı: {title}", "Undone: {}"),
    ("Geri yükle (pop)", "Restore (pop)"),
    ("Geç", "Switch"),
    ("Geçmiş", "History"),
    ("Geçmiş sekmesine git", "Go to History"),
    ("Giriş reddedildi.", "Sign-in was denied."),
    ("Git kimliği (genel)", "Git identity (global)"),
    ("Git kimliği ayarlı değil — Ayarlar > Git bölümünden ekle", "Git identity not set — add it in Settings > Git"),
    ("Git kimliği kaydedildi", "Git identity saved"),
    ("Git kimliği ~/.gitconfig dosyasından okundu: {} · {}", "Git identity read from ~/.gitconfig: {} · {}"),
    ("GitHub hesabı bağlı değil", "No GitHub account connected"),
    ("GitHub hesabı bağlı değil (Ayarlar > Hesaplar).", "No GitHub account connected (Settings > Accounts)."),
    ("GitHub hesapları", "GitHub accounts"),
    ("GitHub ile giriş yap", "Sign in with GitHub"),
    ("GitHub · {} bağlı", "GitHub · {} connected"),
    ("Görünüm", "Appearance"),
    ("Gösterilecek metin değişikliği yok", "No text changes to show"),
    ("Gözat", "Browse"),
    ("Güvenlik", "Safety"),
    ("Hard reset", "Hard reset"),
    ("Hard reset yapılsın mı?", "Hard reset?"),
    ("Hard reset: reflog'dan kurtarılabilir", "Hard reset: recoverable from the reflog"),
    ("Hard — tüm değişiklikleri sil", "Hard — discard all changes"),
    ("Hazır", "Ready"),
    ("Hazırla", "Stage"),
    ("Hazırla: {f}", "Stage: {}"),
    ("Hazırlananları geri çek", "Unstage everything"),
    ("Hazırlandı", "Staged"),
    ("Hazırlıktan çıkar", "Unstage"),
    ("Hazırlığı kaldır", "Unstage"),
    ("Henüz commit yok", "No commits yet"),
    ("Her yazma işlemi anlık görüntüyle saklanır; uzağa giden işlemler geri alınamaz.", "Every write is saved with a snapshot; operations that reach the remote can't be undone."),
    ("Hesap ekle", "Add account"),
    ("Hesaplar", "Accounts"),
    ("Hunk atıldı", "Hunk discarded"),
    ("Hunk atılsın mı?", "Discard hunk?"),
    ("Hunk'ı at", "Discard hunk"),
    ("Hunk'ı hazırla", "Stage hunk"),
    ("J / K ile dosyalar arasında gez", "Use J / K to move between files"),
    ("Kapat", "Close"),
    ("Kimliği kaydet", "Save identity"),
    ("Klonla", "Clone"),
    ("Klonlama tamamlandı", "Clone complete"),
    ("Klonlanacak klasörü seç", "Choose the clone destination"),
    ("Klonlanıyor · {}", "Cloning · {}"),
    ("Klonlanıyor…", "Cloning…"),
    ("Kod isteniyor…", "Requesting code…"),
    ("Kodu kopyala", "Copy code"),
    ("Kodun süresi doldu; yeniden dene.", "The code expired; try again."),
    ("Komut kopyalandı", "Command copied"),
    ("Komut paleti", "Command palette"),
    ("Komut, dosya, dal ve commit ara", "Search commands, files, branches and commits"),
    ("Komut, dosya, dal veya depo ara", "Search commands, files, branches or repositories"),
    ("Koyu", "Dark"),
    ("Liste yoğunluğu", "List density"),
    ("Mesaj, hash veya yazar ara", "Search message, hash or author"),
    ("Mixed — değişiklikleri çalışma alanında tut", "Mixed — keep changes in the working tree"),
    ("Oluştur", "Create"),
    ("Onaylayınca bu pencere kendiliğinden kapanır.", "This window closes by itself once you approve."),
    ("Onların", "Theirs"),
    ("Onlarınki", "Theirs"),
    ("Pull request'ler", "Pull requests"),
    ("Pull request'lere git", "Go to Pull requests"),
    ("Pull tamamlandı", "Pull complete"),
    ("Pull: çatışmalar var", "Pull: there are conflicts"),
    ("Rahat", "Comfortable"),
    ("SON DEPOLAR", "RECENT REPOSITORIES"),
    ("Satır içi", "Inline"),
    ("Seçilenleri hazırla ({sel_n})", "Stage selected ({})"),
    ("Seçili commit'ten sonraki commit'ler ve tüm yerel değişiklikler silinir.", "Commits after the selected one and all local changes will be lost."),
    ("Seçili dosyayı hazırla / çıkar", "Stage / unstage selected file"),
    ("Sil", "Delete"),
    ("Sistem", "System"),
    ("Soft — değişiklikleri hazırda tut", "Soft — keep changes staged"),
    ("Son commit'i düzelt (amend)", "Amend last commit"),
    ("Son git komutunu kopyala", "Copy last git command"),
    ("Son işlem geri alınamaz", "The last operation can't be undone"),
    ("Son işlem geri alınamaz: {w}", "The last operation can't be undone: {}"),
    ("Son işlem: {}", "Last: {}"),
    ("Son işlemi geri al", "Undo last operation"),
    ("Sonraki dosya", "Next file"),
    ("Sonraki hunk", "Next hunk"),
    ("Sonuç yok", "No results"),
    ("Stash boş", "Stash is empty"),
    ("Stash geri yüklendi", "Stash restored"),
    ("Stash sayfasından geri yükle", "Restore from the Stash page"),
    ("Stash silindi", "Stash dropped"),
    ("Stash uygulandı", "Stash applied"),
    ("Stash çatışma verdi", "Stash hit conflicts"),
    ("Stash'i geri yükle: {n}", "Restore stash: {}"),
    ("Stash'le ve geç", "Stash and switch"),
    ("Stash'lere git", "Go to Stash"),
    ("Sıfırla", "Reset"),
    ("Sıfırlandı", "Reset done"),
    ("Sıfırlandı ({mode}) → {short}", "Reset ({}) → {}"),
    ("Sıkışık", "Compact"),
    ("Takip edilen dosyalar kısa süreliğine geri alınabilir; yeni dosyalar Geri Dönüşüm Kutusu'na taşınır.", "Tracked files can be restored for a short while; new files are moved to the Recycle Bin."),
    ("Takipsiz", "Untracked"),
    ("Tarayıcıda aç", "Open in browser"),
    ("Tarayıcıda açılan sayfaya aşağıdaki kodu gir. Token kopyalamana gerek yok.", "Enter the code below on the page that opened in your browser. No token to copy."),
    ("Tarayıcıyı aç", "Open browser"),
    ("Taslak", "Draft"),
    ("Tema", "Theme"),
    ("Terminalde aç", "Open in terminal"),
    ("Tüm dosya", "Whole file"),
    ("Tüm dosyayı hazırla", "Stage whole file"),
    ("Tüm çatışmalar çözüldü", "All conflicts resolved"),
    ("Tümü", "All"),
    ("Tümünü hazırla", "Stage all"),
    ("URL ile ya da hesabından seç", "Use a URL or pick from your account"),
    ("UZAK", "REMOTE"),
    ("Uygula", "Apply"),
    ("Uzak daldaki geçmiş yerel geçmişinle değiştirilir. --force-with-lease kullanılır: başkası araya commit eklediyse işlem reddedilir.", "The remote history is replaced with your local history. --force-with-lease is used: it's refused if someone pushed in between."),
    ("Varsayılan editör", "Default editor"),
    ("Vazgeç", "Cancel"),
    ("YEREL", "LOCAL"),
    ("Ya da bir klasörü pencereye sürükle", "Or drag a folder into the window"),
    ("Yan yana", "Side by side"),
    ("Yedek bulunamadı", "Backup not found"),
    ("Yeni dal", "New branch"),
    ("Yeni dal oluştur: {q}", "Create new branch: {}"),
    ("Yeni dal oluştur…", "Create new branch…"),
    ("Yeni depo oluştur", "Create new repository"),
    ("Yeni depo oluştur…", "Create new repository…"),
    ("Yeni deponun klasörünü seç", "Choose the new repository's folder"),
    ("Yeniden dene", "Try again"),
    ("Yenile", "Refresh"),
    ("Yerel depo aç…", "Open local repository…"),
    ("Yerel depo ekle", "Add local repository"),
    ("Yerel klasör", "Local folder"),
    ("Yinele", "Redo"),
    ("Yinelendi: {t2}", "Redone: {}"),
    ("Yinelenecek işlem yok", "Nothing to redo"),
    ("Yinelenemedi: {e}", "Couldn't redo: {}"),
    ("Yol kopyalandı", "Path copied"),
    ("Yolu kopyala", "Copy path"),
    ("Yolu kopyala: {f}", "Copy path: {}"),
    ("Yükleniyor…", "Loading…"),
    ("araya geri alınamaz işlem girmiş", "a non-undoable operation is in between"),
    ("az önce", "just now"),
    ("bağlanıyor…", "connecting…"),
    ("değişen dosya", "changed file"),
    ("değişiklik yok", "no changes"),
    ("dün", "yesterday"),
    ("ebeveyn ", "parent "),
    ("geri alınamaz", "can't be undone"),
    ("uzak durum, geri alınmaz", "remote state, not undoable"),
    ("uzak · {subject}", "remote · {}"),
    ("uzağa gönderildi, geri alınmaz", "sent to the remote, not undoable"),
    ("yeni", "new"),
    ("mevcut", "current"),
    ("etkin", "active"),
    ("{base} dalından oluşturulur ve geçilir.", "Created from {} and checked out."),
    ("{conflicts} çatışma", "{} conflict(s)"),
    ("{current}  ·  {note}", "{}  ·  {}"),
    ("{cur} içine birleştir", "Merge into {}"),
    ("{k} işlem", "{} operations"),
    ("{l} sürüyor", "{} in progress"),
    ("{n_all} dosya · {n_staged} hazır", "{} files · {} staged"),
    ("{n_all} dosya · {n_staged} hazır · {n_partial} kısmen", "{} files · {} staged · {} partial"),
    ("{n_changed} / {n_changed} satır hazır", "{} / {} lines staged"),
    ("{n_changes} değişiklik var; geçişte seçenek sorulur", "{} changes; you'll be asked how to switch"),
    ("{n_changes} kaydedilmemiş değişiklik var", "{} uncommitted changes"),
    ("{n_un} commit henüz origin'e gönderilmedi (içi boş daireler)", "{} commit(s) not pushed to origin yet (hollow circles)"),
    ("{name} birleştirildi", "{} merged"),
    ("{name} dalına geçildi", "Switched to {}"),
    ("{name} oluşturuldu ve geçildi", "{} created and checked out"),
    ("{n} dosya hazırlandı", "{} file(s) staged"),
    ("{n} dosya hazırlıktan çıkarıldı", "{} file(s) unstaged"),
    ("{n} satır hazırlandı", "{} line(s) staged"),
    ("{n} satır hazırlıktan çıkarıldı", "{} line(s) unstaged"),
    ("{open} / {total} çatışma bekliyor", "{} / {} conflicts remaining"),
    ("{sel_n} / {n_changed} satır seçili", "{} / {} lines selected"),
    ("{short} commit'inden", "from commit {}"),
    ("{} açık", "{} open"),
    ("{} commit", "{} commits"),
    ("{} dosya", "{} files"),
    ("{} dosya değişti", "{} files changed"),
    ("{} hesabı bağlandı", "{} account connected"),
    ("{} kayıt", "{} entries"),
    ("{} yerel · {} uzak · {} etiket", "{} local · {} remote · {} tags"),
    ("{} · {} güncelledi", "{} · updated {}"),
    ("{}+ commit · sanal liste", "{}+ commits · virtual list"),
    ("Çalışma alanı temiz", "Working tree is clean"),
    ("Çalışıyor", "Working"),
    ("Çatışma {ci}", "Conflict {}"),
    ("Çatışma çözümüne git", "Go to conflict resolution"),
    ("Çatışma çıktı; çözüp devam et", "Conflict; resolve it and continue"),
    ("Çatışmalar", "Conflicts"),
    ("Çözüldü olarak işaretle", "Mark as resolved"),
    ("Çözümlerin atılır ve depo işlem öncesi duruma döner.", "Your resolutions are dropped and the repository returns to its state before the operation."),
    ("Çıkış", "Sign out"),
    ("Önce GitHub hesabı bağla (Ayarlar > Hesaplar)", "Connect a GitHub account first (Settings > Accounts)"),
    ("Önce commit'e girecek değişiklikleri hazırla", "Stage the changes to commit first"),
    ("Önce çatışmaları çöz", "Resolve the conflicts first"),
    ("Önceki dosya", "Previous file"),
    ("Önceki hunk", "Previous hunk"),
    ("İkili dosya", "Binary file"),
    ("İkili dosya — fark gösterilemiyor", "Binary file — no diff to show"),
    ("İkisi de", "Both"),
    ("İptal", "Cancel"),
    ("İşlem günlüğü", "Operation log"),
    ("İşlem günlüğünü aç", "Open operation log"),
    ("İşlem iptal edildi", "Operation aborted"),
    ("İşlem iptal edilsin mi?", "Abort the operation?"),
    ("İşlem tamamlandı", "Operation complete"),
    ("İşleme devam et", "Continue operation"),
    ("İşlemi iptal et", "Abort operation"),
    ("“{b}” dalı silinsin mi?", "Delete branch “{}”?"),
    ("“{cur}” için PR oluştur", "Create PR for “{}”"),
    ("“{target}” dalına geçmeden önce ne yapılsın?", "What should happen before switching to “{}”?"),
    ("“{}” değişiklikleri atılsın mı?", "Discard changes to “{}”?"),
    ("‹adres›", "‹url›"),
    ("↑↓ gezin   ↵ geç", "↑↓ navigate   ↵ switch"),
    ("↑↓ gezin   ↵ çalıştır", "↑↓ navigate   ↵ run"),
    ("⋯ {} satır değişmedi", "⋯ {} lines unchanged"),
    ("⏎ satır sonu yok", "⏎ no newline at end"),
    ("üstteki işlem önce geri alınmalı", "undo the operation above first"),
    ("Branch'i yayınla", "Publish branch"),
    ("Tümü {}", "All {}"),
    ("Hazır {}", "Staged {}"),
    ("Takipsiz {}", "Untracked {}"),
    ("{} sürüyor", "{} in progress"),
    ("{} sayfası", "{} page"),
    // time
    ("{} dk önce", "{}m ago"),
    ("{} sa önce", "{}h ago"),
    ("{} gün önce", "{}d ago"),
    ("{} hafta önce", "{}w ago"),
    ("{} ay önce", "{}mo ago"),
    ("{} yıl önce", "{}y ago"),
    ("{} dk", "{}m"),
    ("{} sa", "{}h"),
    ("{} gün", "{}d"),
    ("{} hafta", "{}w"),
    ("{} ay", "{}mo"),
    ("{} yıl", "{}y"),
    ("şimdi", "now"),
    // backend messages
    ("Klasör bulunamadı", "Folder not found"),
    ("Bu klasör bir Git deposu değil", "This folder is not a Git repository"),
    ("Depo bulunamadı", "Repository not found"),
    // welcome / clone extras
    ("Depo adresi gerekli", "Repository URL required"),
    ("Bu bilgisayardaki depoları bul", "Find repositories on this computer"),
    ("Taranıyor…", "Scanning…"),
    ("{} depo bulundu", "{} repositories found"),
    ("Depo bulunamadı; elle ekleyebilirsin", "No repositories found; you can add one manually"),
    ("Dil", "Language"),
    ("Hesap", "Account"),
    ("Burada yeni bir depo oluşturulsun mu? (git init)", "Create a new repository here? (git init)"),
    ("Depo oluştur", "Create repository"),
    ("Depoyu yayınla", "Publish repository"),
    ("Depoyu GitHub'da yayınla", "Publish repository to GitHub"),
    ("Depo GitHub hesabında oluşturulur ve geçerli dal gönderilir.", "The repository is created on your GitHub account and the current branch is pushed."),
    ("Özel depo", "Private repository"),
    ("Yayınla", "Publish"),
    ("Yayın", "Publish"),
    ("Yayın…", "Publishing…"),
    ("Önce GitHub'a giriş yap", "Sign in to GitHub first"),
    ("Depo yayınlandı", "Repository published"),
    ("Depo oluşturuldu; ilk commit'ten sonra Push'la", "Repository created; push after your first commit"),
    ("Kopyalandı", "Copied"),
    ("Satırı kopyala", "Copy line"),
    ("Seçili satırları kopyala", "Copy selected lines"),
    ("Hunk'ı kopyala", "Copy hunk"),
    ("Kopyala", "Copy"),
    ("Tarayıcıda GitHub giriş sayfası açıldı. Orada onayla; bu pencere kendiliğinden kapanır.", "A GitHub sign-in page opened in your browser. Approve it there; this window closes by itself."),
    ("Giriş, git ile aynı yerde saklanan Git Credential Manager hesabıyla yapılır. Başka bir hesap için önce tarayıcıda o hesaba geç.", "Sign-in uses Git Credential Manager, the same store git uses. For another account, switch to it in your browser first."),
    ("Git Credential Manager bulunamadı. Git for Windows'u “Git Credential Manager” seçeneğiyle kur.", "Git Credential Manager not found. Reinstall Git for Windows with the “Git Credential Manager” option."),
    ("Hesap bulundu ama erişim anahtarı okunamadı", "Account found, but its access token could not be read"),
    ("Giriş tamamlanamadı", "Sign-in did not complete"),
    ("BU BİLGİSAYARDA BULUNANLAR", "FOUND ON THIS COMPUTER"),
    ("Klasör tara…", "Scan a folder…"),
    ("Taranacak klasörü seç", "Choose a folder to scan"),
    ("Kısayollar", "Shortcuts"),
    ("Otomatik", "Automatic"),
    ("Türkçe", "Türkçe"),
];
