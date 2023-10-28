use regex::Regex;
use std::borrow::Cow;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct VcsInfo {
    repo_url: String,
    branch: Option<String>,
    subpath: Option<String>,
}

impl FromStr for VcsInfo {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut s: Cow<str> = s.trim().into();
        let mut subpath: Option<String> = None;
        let branch: Option<String>;
        let repo_url: String;
        let re = Regex::new(r" \[([^] ]+)\]").unwrap();

        if let Some(ref m) = re.find(s.as_ref()) {
            subpath = Some(m.as_str()[2..m.as_str().len() - 1].to_string());
            s = Cow::Owned(vec![s[..m.start()].to_string(), s[m.end()..].to_string()].concat());
        }

        if let Some(index) = s.find(" -b ") {
            let (url, branch_str) = s.split_at(index);
            branch = Some(branch_str[4..].to_string());
            repo_url = url.to_string();
        } else {
            branch = None;
            repo_url = s.to_string();
        }

        Ok(VcsInfo {
            repo_url,
            branch,
            subpath,
        })
    }
}

impl ToString for VcsInfo {
    fn to_string(&self) -> String {
        let mut url = self.repo_url.clone();

        if let Some(branch) = &self.branch {
            url = format!("{} -b {}", url, branch);
        }

        if let Some(subpath) = &self.subpath {
            url = format!("{} [{}]", url, subpath);
        }

        url
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_vcs_info() {
        let vcs_info = VcsInfo::from_str("https://github.com/jelmer/example").unwrap();
        assert_eq!(vcs_info.repo_url, "https://github.com/jelmer/example");
        assert_eq!(vcs_info.branch, None);
        assert_eq!(vcs_info.subpath, None);
    }

    #[test]
    fn test_vcs_info_with_branch() {
        let vcs_info = VcsInfo::from_str("https://github.com/jelmer/example -b branch").unwrap();
        assert_eq!(vcs_info.repo_url, "https://github.com/jelmer/example");
        assert_eq!(vcs_info.branch, Some("branch".to_string()));
        assert_eq!(vcs_info.subpath, None);
    }

    #[test]
    fn test_vcs_info_with_subpath() {
        let vcs_info = VcsInfo::from_str("https://github.com/jelmer/example [subpath]").unwrap();
        assert_eq!(vcs_info.repo_url, "https://github.com/jelmer/example");
        assert_eq!(vcs_info.branch, None);
        assert_eq!(vcs_info.subpath, Some("subpath".to_string()));
    }

    #[test]
    fn test_vcs_info_with_branch_and_subpath() {
        let vcs_info =
            VcsInfo::from_str("https://github.com/jelmer/example -b branch [subpath]").unwrap();
        assert_eq!(vcs_info.repo_url, "https://github.com/jelmer/example");
        assert_eq!(vcs_info.branch, Some("branch".to_string()));
        assert_eq!(vcs_info.subpath, Some("subpath".to_string()));
    }
}