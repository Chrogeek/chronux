use crate::config::counter::Counters;

#[derive(Debug, Clone)]
pub enum PlaceholderText {
    Counter {
        counter: String,
        max_significant: usize,
        max_decimals: usize,
    },
}

impl PlaceholderText {
    pub fn parse(s: &str) -> Option<PlaceholderText> {
        let s = s.trim();
        if s.starts_with("counter(") && s.ends_with(')') {
            let inner = &s[8..s.len() - 1];
            let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
            if parts.len() == 3 {
                let counter = parts[0].to_string();
                let max_significant = parts[1].parse().ok()?;
                let max_decimals = parts[2].parse().ok()?;
                return Some(PlaceholderText::Counter {
                    counter,
                    max_significant,
                    max_decimals,
                });
            }
        }
        None
    }

    pub fn format(&self, counters: &Counters) -> String {
        match self {
            PlaceholderText::Counter {
                counter,
                max_significant,
                max_decimals,
            } => {
                let value = *counters.get(counter).unwrap_or(&0.0);

                if *max_decimals == 0 {
                    return format!("{value:.0}");
                }

                let formatted = format!("{:.1$}", value, *max_decimals);
                let mut parts = formatted.split('.');
                let int = parts.next().unwrap().to_string();
                let frac = parts.next().unwrap().to_string();

                if int.len() >= *max_significant {
                    int
                } else if frac.len() <= *max_significant - int.len() {
                    formatted
                } else {
                    format!(
                        "{}.{}",
                        int,
                        &frac[0..(*max_significant - int.len())]
                    )
                }
            }
        }
    }
}
