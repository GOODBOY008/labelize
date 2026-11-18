use chrono::{DateTime, Datelike, Duration, Locale, Months, NaiveDate, Timelike, Utc};

#[derive(Default)]
pub(super) struct FieldClock {
    adjustment: Duration,
    start: Option<DateTime<Utc>>,
    time_now: bool,
    language: u8,
    offsets: [[i32; 6]; 2],
}

impl FieldClock {
    pub fn changes_per_copy(&self) -> bool {
        self.time_now
    }

    pub fn start_format(&mut self) {
        self.start = Some(Utc::now() + self.adjustment);
    }

    pub fn configure(&mut self, command: &str, parts: &[&str]) {
        let number = |i: usize| parts.get(i).and_then(|p| p.trim().parse::<i32>().ok());
        match command {
            "^SL" => {
                self.time_now = parts.first().is_some_and(|s| s.eq_ignore_ascii_case("T"))
                    || number(0).is_some();
                self.language = number(1).unwrap_or(1) as u8;
            }
            "^SO" => {
                if let Some(n @ 2..=3) = number(0) {
                    self.offsets[n as usize - 2] =
                        std::array::from_fn(|i| number(i + 1).unwrap_or(0).clamp(-32000, 32000));
                }
            }
            "^ST" => {
                let now = Utc::now() + self.adjustment;
                let year = number(2).unwrap_or(now.year());
                let Ok(month) = u32::try_from(number(0).unwrap_or(now.month() as i32)) else {
                    return;
                };
                let Ok(day) = u32::try_from(number(1).unwrap_or(now.day() as i32)) else {
                    return;
                };
                let Some(date) = NaiveDate::from_ymd_opt(year, month, day) else {
                    return;
                };
                let mut hour = number(3).unwrap_or(now.hour() as i32);
                if let Some(fmt) = parts.get(6) {
                    if (1..=12).contains(&hour)
                        && (fmt.eq_ignore_ascii_case("A") || fmt.eq_ignore_ascii_case("P"))
                    {
                        hour = hour % 12 + if fmt.eq_ignore_ascii_case("P") { 12 } else { 0 };
                    }
                }
                if let Some(dt) = date.and_hms_opt(
                    hour as u32,
                    number(4).unwrap_or(now.minute() as i32) as u32,
                    number(5).unwrap_or(now.second() as i32) as u32,
                ) {
                    let dt = dt.and_utc();
                    self.adjustment = dt - Utc::now();
                    self.start = Some(dt);
                }
            }
            _ => {}
        }
    }

    pub fn expand(&self, text: &str, markers: [Option<char>; 3]) -> String {
        let now = if self.time_now { None } else { self.start }
            .unwrap_or_else(|| Utc::now() + self.adjustment);
        let mut dates = [now; 3];
        for (date, &[month, day, year, hour, min, sec]) in dates[1..].iter_mut().zip(&self.offsets)
        {
            let months = month + year * 12;
            let shifted = if months >= 0 {
                now.checked_add_months(Months::new(months as u32))
            } else {
                now.checked_sub_months(Months::new(months.unsigned_abs()))
            };
            if let Some(time) = shifted.and_then(|s| {
                s.checked_add_signed(Duration::seconds(
                    i64::from(day) * 86400
                        + i64::from(hour) * 3600
                        + i64::from(min) * 60
                        + i64::from(sec),
                ))
            }) {
                *date = time;
            }
        }
        let locale = match self.language {
            2 | 10 => Locale::es_ES,
            3 => Locale::fr_FR,
            4 => Locale::de_DE,
            5 => Locale::it_IT,
            6 => Locale::nb_NO,
            7 => Locale::pt_PT,
            8 => Locale::sv_SE,
            9 => Locale::da_DK,
            11 => Locale::nl_NL,
            12 => Locale::fi_FI,
            13 => Locale::ja_JP,
            14 => Locale::ko_KR,
            15 => Locale::zh_CN,
            16 => Locale::zh_TW,
            17 => Locale::ru_RU,
            18 => Locale::pl_PL,
            _ => Locale::en_US,
        };
        let mut result = String::new();
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            if let (Some(i), Some(&key)) =
                (markers.iter().position(|m| *m == Some(c)), chars.peek())
            {
                if key == c {
                    result.push(c);
                    chars.next();
                    continue;
                }
                if "aAbBdhHIjmMpSUWwyY".contains(key) {
                    chars.next();
                    if key == 'w' {
                        result
                            .push_str(&format!("{:02}", dates[i].weekday().num_days_from_sunday()));
                    } else {
                        result.push_str(
                            &dates[i]
                                .format_localized(&format!("%{key}"), locale)
                                .to_string(),
                        );
                    }
                    continue;
                }
            }
            result.push(c);
        }
        result
    }
}
