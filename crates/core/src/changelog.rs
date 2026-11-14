//! Release notes for an update: every version between the one installed and the new one,
//! taken from CHANGELOG.md (which the release workflow puts in `latest.json`), so someone a
//! few versions behind sees everything they are getting, not only the newest version's notes.

/// A version's numbers ("0.9.5", "v1.2" → [0, 9, 5], [1, 2]); `None` if it is not one.
fn parse(v: &str) -> Option<Vec<u64>> {
    let v = v.trim().trim_start_matches(['v', 'V']);
    let nums: Option<Vec<u64>> = v.split('.').map(|p| p.parse().ok()).collect();
    nums.filter(|n| !n.is_empty())
}

/// Compare versions number by number (missing numbers count as 0: 1.2 == 1.2.0).
fn cmp(a: &[u64], b: &[u64]) -> std::cmp::Ordering {
    let n = a.len().max(b.len());
    (0..n).map(|i| a.get(i).unwrap_or(&0).cmp(b.get(i).unwrap_or(&0))).find(|o| o.is_ne()).unwrap_or(std::cmp::Ordering::Equal)
}

/// The changelog's sections for versions newer than `current` up to and including `latest`,
/// newest first: (version, notes). Headings look like `## 0.9.5` (also `## [0.9.5] - date`,
/// `## v0.9.5`); "Unreleased" and anything that is not a version is skipped.
pub fn since(changelog: &str, current: &str, latest: &str) -> Vec<(String, String)> {
    let (Some(cur), Some(new)) = (parse(current), parse(latest)) else { return vec![] };
    let mut out: Vec<(Vec<u64>, String, String)> = vec![];
    let mut section: Option<(Vec<u64>, String, Vec<&str>)> = None;
    let finish = |s: Option<(Vec<u64>, String, Vec<&str>)>, out: &mut Vec<(Vec<u64>, String, String)>| {
        if let Some((v, name, lines)) = s {
            let notes = lines.join("\n").trim().to_string();
            if cmp(&v, &cur).is_gt() && cmp(&v, &new).is_le() && !notes.is_empty() {
                out.push((v, name, notes));
            }
        }
    };
    for line in changelog.lines() {
        if let Some(head) = line.strip_prefix("## ") {
            finish(section.take(), &mut out);
            // "0.9.5", "[0.9.5] - 2026-11-12", "v0.9.5 (date)": the first word, brackets off.
            let word = head.split_whitespace().next().unwrap_or("").trim_matches(['[', ']']);
            section = parse(word).map(|v| (v, word.trim_start_matches(['v', 'V']).to_string(), vec![]));
        } else if let Some((_, _, lines)) = section.as_mut() {
            lines.push(line);
        }
    }
    finish(section.take(), &mut out);
    out.sort_by(|a, b| cmp(&b.0, &a.0));
    out.dedup_by(|a, b| cmp(&a.0, &b.0).is_eq());
    out.into_iter().map(|(_, name, notes)| (name, notes)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOG: &str = "# Changelog\n\nIntro text.\n\n## Unreleased\n\n### New features\n- not out yet\n\n## 0.9.5\n\n### New features\n- fonts\n\n## [0.9.4] - 2026-11-12\n\n### New features\n- fuel\n\n### Fixes\n- a fix\n\n## v0.9.3\n- league import\n\n## 0.9.2\n- vr\n\n## 0.10.0\n- (out of order, newer than everything)\n";

    #[test]
    fn versions_after_the_installed_one() {
        let s = since(LOG, "0.9.2", "0.9.5");
        assert_eq!(s.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(), vec!["0.9.5", "0.9.4", "0.9.3"]);
        assert_eq!(s[1].1, "### New features\n- fuel\n\n### Fixes\n- a fix");
        // One version behind: just the newest. Unreleased never shows.
        assert_eq!(since(LOG, "0.9.4", "0.9.5").len(), 1);
        assert!(since(LOG, "0.9.5", "0.9.5").is_empty());
        // Numbers compare as numbers (0.10.0 is after 0.9.x), and only up to the update.
        assert_eq!(since(LOG, "0.9.4", "0.10.0").iter().map(|x| x.0.as_str()).collect::<Vec<_>>(), vec!["0.10.0", "0.9.5"]);
        assert!(since(LOG, "nonsense", "0.9.5").is_empty());
    }

    #[test]
    fn reads_the_real_changelog() {
        let real = include_str!("../../../CHANGELOG.md");
        let s = since(real, "0.9.2", "0.9.5");
        assert_eq!(s.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(), vec!["0.9.5", "0.9.4", "0.9.3"]);
        assert!(s.iter().all(|x| x.1.contains("### ")), "each version has its sections");
    }
}
