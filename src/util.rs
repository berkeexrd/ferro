//! Small helpers: relative time in Turkish, path splitting, ellipsis.

pub fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// Relative time ("2 sa önce", "dün", "3 gün önce").
pub fn rel(ts: i64) -> String {
    if ts <= 0 {
        return String::new();
    }
    let d = (now() - ts).max(0);
    if d < 60 {
        "az önce".into()
    } else if d < 3600 {
        format!("{} dk önce", d / 60)
    } else if d < 86400 {
        format!("{} sa önce", d / 3600)
    } else if d < 2 * 86400 {
        "dün".into()
    } else if d < 14 * 86400 {
        format!("{} gün önce", d / 86400)
    } else if d < 60 * 86400 {
        format!("{} hafta önce", d / (7 * 86400))
    } else if d < 700 * 86400 {
        format!("{} ay önce", d / (30 * 86400))
    } else {
        format!("{} yıl önce", d / (365 * 86400))
    }
}

pub fn rel_short(ts: i64) -> String {
    let s = rel(ts);
    s.replace(" önce", "").replace("az önce", "şimdi")
}

pub fn parse_ts(s: &str) -> i64 {
    s.trim().parse().unwrap_or(0)
}

/// "src/git/status.rs" -> ("src/git/", "status.rs")
pub fn split_path(p: &str) -> (&str, &str) {
    match p.rfind('/') {
        Some(i) => (&p[..=i], &p[i + 1..]),
        None => ("", p),
    }
}

pub fn short(sha: &str) -> &str {
    &sha[..sha.len().min(7)]
}

/// Local UTC offset in seconds (Windows time-zone API; 0 elsewhere).
pub fn local_offset() -> i64 {
    #[cfg(windows)]
    {
        #[repr(C)]
        struct SystemTime([u16; 8]);
        #[repr(C)]
        struct Tzi {
            bias: i32,
            std_name: [u16; 32],
            std_date: SystemTime,
            std_bias: i32,
            dst_name: [u16; 32],
            dst_date: SystemTime,
            dst_bias: i32,
        }
        extern "system" {
            fn GetTimeZoneInformation(t: *mut Tzi) -> u32;
        }
        static OFF: std::sync::OnceLock<i64> = std::sync::OnceLock::new();
        return *OFF.get_or_init(|| unsafe {
            let mut t: Tzi = std::mem::zeroed();
            let r = GetTimeZoneInformation(&mut t);
            let extra = if r == 2 { t.dst_bias } else { t.std_bias };
            -((t.bias + extra) as i64) * 60
        });
    }
    #[cfg(not(windows))]
    {
        0
    }
}

/// "08:41" in local time.
pub fn clock(ts: i64) -> String {
    let secs = (ts + local_offset()).rem_euclid(86400);
    format!("{:02}:{:02}", secs / 3600, (secs % 3600) / 60)
}

pub fn date_tr(ts: i64) -> String {
    let ts = ts + local_offset();
    let days = ts.div_euclid(86400);
    let secs = ts.rem_euclid(86400);
    let (y, m, d) = civil(days);
    const TR: [&str; 12] = ["Oca", "Şub", "Mar", "Nis", "May", "Haz", "Tem", "Ağu", "Eyl", "Eki", "Kas", "Ara"];
    const EN: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    if crate::i18n::is_en() {
        format!("{} {}, {}, {:02}:{:02}", EN[(m - 1) as usize], d, y, secs / 3600, (secs % 3600) / 60)
    } else {
        format!("{} {} {}, {:02}:{:02}", d, TR[(m - 1) as usize], y, secs / 3600, (secs % 3600) / 60)
    }
}

fn civil(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}
