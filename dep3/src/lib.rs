use deb822_lossless::Paragraph;
use std::str::FromStr;

pub struct PatchHeader(Paragraph);

#[derive(Debug, PartialEq, Eq)]
pub enum Forwarded {
    No,
    NotNeeded,
    Yes(String)
}

impl std::str::FromStr for Forwarded {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "no" => Ok(Forwarded::No),
            "not-needed" => Ok(Forwarded::NotNeeded),
            s => Ok(Forwarded::Yes(s.to_string()))
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum OriginCategory {
    /// an upstream patch that had to be modified to apply on the current version
    Backport,
    /// a patch created by Debian or another distribution vendor
    Vendor,
    /// a patch cherry-picked from the upstream VCS
    Upstream,
    Other
}

#[derive(Debug, PartialEq, Eq)]
pub enum Origin {
    Commit(String),
    Other(String)
}

#[derive(Debug, PartialEq, Eq)]
pub enum AppliedUpstream {
    Commit(String),
    Other(String)
}

impl std::str::FromStr for AppliedUpstream {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(rest) = s.strip_prefix("commit:") {
            Ok(AppliedUpstream::Commit(rest.to_string()))
        } else {
            Ok(AppliedUpstream::Other(s.to_string()))
        }
    }
}

pub fn parse_origin(s: &str) -> (Option<OriginCategory>, Origin) {
    // if origin starts with "<category>, " then it is a category

    let mut parts = s.splitn(2, ", ");
    let (category, s) = match parts.next() {
        Some("backport") => (Some(OriginCategory::Backport), parts.next().unwrap_or("")),
        Some("vendor") => (Some(OriginCategory::Vendor), parts.next().unwrap_or("")),
        Some("upstream") => (Some(OriginCategory::Upstream), parts.next().unwrap_or("")),
        Some("other") => (Some(OriginCategory::Other), parts.next().unwrap_or("")),
        None | Some(_) => (None, s),
    };

    if let Some(rest) = s.strip_prefix("commit:") {
        (category, Origin::Commit(rest.to_string()))
    } else {
        (category, Origin::Other(s.to_string()))
    }
}


impl PatchHeader {
    pub fn new() -> Self {
        PatchHeader(Paragraph::new())
    }

    pub fn origin(&self) -> Option<(Option<OriginCategory>, Origin)> {
        self.0.get("Origin").as_deref().map(parse_origin)
    }

    pub fn forwarded(&self) -> Option<Forwarded> {
        self.0.get("Forwarded").as_deref().map(|s| s.parse().unwrap())
    }

    pub fn author(&self) -> Option<String> {
        self.0.get("Author").or_else(|| self.0.get("From"))
    }

    pub fn reviewed_by(&self) -> Vec<String> {
        self.0.get_all("Reviewed-By").collect()
    }

    pub fn last_update(&self) -> Option<chrono::NaiveDate> {
        self.0.get("Last-Update").as_deref().and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
    }

    pub fn applied_upstream(&self) -> Option<AppliedUpstream> {
        self.0.get("Applied-Upstream").as_deref().map(|s| s.parse().unwrap())
    }

    pub fn bugs(&self) -> impl Iterator<Item = (Option<String>, String)> + '_ {
        self.0.items().filter_map(|(k, v)| {
            if k.starts_with("Bug-") {
                Some((Some(k.strip_prefix("Bug-").unwrap().to_string()), v))
            } else if k == "Bug" {
                Some((None, v))
            } else {
                None
            }
        })
    }

    fn description_field(&self) -> Option<String> {
        self.0.get("Description").or_else(|| self.0.get("Subject"))
    }

    pub fn description(&self) -> Option<String> {
        self.description_field().as_deref().map(|s| s.split('\n').next().unwrap_or(s).to_string())
    }

    pub fn long_description(&self) -> Option<String> {
        self.description_field().as_deref().map(|s| s.split_once('\n').map(|x| x.1).unwrap_or("").to_string())
    }
}

impl Default for PatchHeader {
    fn default() -> Self {
        Self::new()
    }
}

impl std::str::FromStr for PatchHeader {
    type Err = deb822_lossless::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(PatchHeader(Paragraph::from_str(s)?))
    }
}

#[cfg(test)]
mod tests {
    use super::PatchHeader;
    use std::str::FromStr;

    #[test]
    fn test_upstream() {
        let text = r#"From: Ulrich Drepper <drepper@redhat.com>
Subject: Fix regex problems with some multi-bytes characters
 
 * posix/bug-regex17.c: Add testcases.
 * posix/regcomp.c (re_compile_fastmap_iter): Rewrite COMPLEX_BRACKET
   handling.
 
Origin: upstream, http://sourceware.org/git/?p=glibc.git;a=commitdiff;h=bdb56bac
Bug: http://sourceware.org/bugzilla/show_bug.cgi?id=9697
Bug-Debian: http://bugs.debian.org/510219
"#;

        let header = PatchHeader::from_str(text).unwrap();

        assert_eq!(header.origin(), Some((Some(super::OriginCategory::Upstream), super::Origin::Other("http://sourceware.org/git/?p=glibc.git;a=commitdiff;h=bdb56bac".to_string()))));
        assert_eq!(header.forwarded(), None);
        assert_eq!(header.author(), Some("Ulrich Drepper <drepper@redhat.com>".to_string()));
        assert_eq!(header.reviewed_by(), Vec::<&str>::new());
        assert_eq!(header.last_update(), None);
        assert_eq!(header.applied_upstream(), None);
        assert_eq!(header.bugs().collect::<Vec<_>>(), vec![
            (None, "http://sourceware.org/bugzilla/show_bug.cgi?id=9697".to_string()),
            (Some("Debian".to_string()), "http://bugs.debian.org/510219".to_string()),
        ]);
        assert_eq!(header.description(), Some("Fix regex problems with some multi-bytes characters".to_string()));
    }

    #[test]
    fn test_forwarded() {
        let text = r#"Description: Use FHS compliant paths by default
 Upstream is not interested in switching to those paths.
 .
 But we will continue using them in Debian nevertheless to comply with
 our policy.
Forwarded: http://lists.example.com/oct-2006/1234.html
Author: John Doe <johndoe-guest@users.alioth.debian.org>
Last-Update: 2006-12-21
"#;
        let header = PatchHeader::from_str(text).unwrap();

        assert_eq!(header.origin(), None);
        assert_eq!(header.forwarded(), Some(super::Forwarded::Yes("http://lists.example.com/oct-2006/1234.html".to_string())));
        assert_eq!(header.author(), Some("John Doe <johndoe-guest@users.alioth.debian.org>".to_string()));
        assert_eq!(header.reviewed_by(), Vec::<&str>::new());
        assert_eq!(header.last_update(), Some(chrono::NaiveDate::from_ymd(2006, 12, 21)));
        assert_eq!(header.applied_upstream(), None);
        assert_eq!(header.bugs().collect::<Vec<_>>(), vec![]);
        assert_eq!(header.description(), Some("Use FHS compliant paths by default".to_string()));
    }

    #[test]
    fn test_not_forwarded() {
        let text = r#"Description: Workaround for broken symbol resolving on mips/mipsel
 The correct fix will be done in etch and it will require toolchain
 fixes.
Forwarded: not-needed
Origin: vendor, http://bugs.debian.org/cgi-bin/bugreport.cgi?msg=80;bug=265678
Bug-Debian: http://bugs.debian.org/265678
Author: Thiemo Seufer <ths@debian.org>
"#;

        let header = PatchHeader::from_str(text).unwrap();

        assert_eq!(header.origin(), Some((Some(super::OriginCategory::Vendor), super::Origin::Other("http://bugs.debian.org/cgi-bin/bugreport.cgi?msg=80;bug=265678".to_string()))));
        assert_eq!(header.forwarded(), Some(super::Forwarded::NotNeeded));
        assert_eq!(header.author(), Some("Thiemo Seufer <ths@debian.org>".to_string()));
        assert_eq!(header.reviewed_by(), Vec::<&str>::new());
        assert_eq!(header.last_update(), None);
        assert_eq!(header.applied_upstream(), None);
        assert_eq!(header.bugs().collect::<Vec<_>>(), vec![
            (Some("Debian".to_string()), "http://bugs.debian.org/265678".to_string()),
        ]);

        assert_eq!(header.description(), Some("Workaround for broken symbol resolving on mips/mipsel".to_string()));
    }

    #[test]
    fn test_applied_upstream() {
        let text = r#"Description: Fix widget frobnication speeds
 Frobnicating widgets too quickly tended to cause explosions.
Forwarded: http://lists.example.com/2010/03/1234.html
Author: John Doe <johndoe-guest@users.alioth.debian.org>
Applied-Upstream: 1.2, http://bzr.example.com/frobnicator/trunk/revision/123
Last-Update: 2010-03-29
"#;
        let header = PatchHeader::from_str(text).unwrap();

        assert_eq!(header.origin(), None);
        assert_eq!(header.forwarded(), Some(super::Forwarded::Yes("http://lists.example.com/2010/03/1234.html".to_string())));
        assert_eq!(header.author(), Some("John Doe <johndoe-guest@users.alioth.debian.org>".to_string()));
        assert_eq!(header.reviewed_by(), Vec::<&str>::new());
        assert_eq!(header.last_update(), Some(chrono::NaiveDate::from_ymd(2010, 3, 29)));
        assert_eq!(header.applied_upstream(), Some(super::AppliedUpstream::Other("1.2, http://bzr.example.com/frobnicator/trunk/revision/123".to_string())));
        assert_eq!(header.bugs().collect::<Vec<_>>(), vec![]);
        assert_eq!(header.description(), Some("Fix widget frobnication speeds".to_string()));
    }
}