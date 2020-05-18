/// Attempt to pull a org name and repo name from a GitHub URL.
pub fn extract_gh_info(url: &str) -> Option<(String, String)> {
    let index = match url.find("github.com/") {
        Some(i) => i + 11,
        None => return None,
    };
    let rest: String = url.chars().skip(index).collect();

    let mut parts = rest.split('/');
    let org = match parts.next() {
        Some(s) => s,
        None => return None,
    };
    let repo = match parts.next() {
        Some(s) => s,
        None => return None,
    };
    Some((org.to_owned(), repo.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::extract_gh_info;

    #[test]
    fn test_extract_gh_info_valid() {
        let url = "https://github.com/Celeo/check_for_license/actions";
        let (org, repo) = extract_gh_info(url).unwrap();
        assert_eq!(org, "Celeo");
        assert_eq!(repo, "check_for_license");
    }

    #[test]
    fn test_extract_gh_info_invalid() {
        let url = "https://github.com/Celeo";
        let data = extract_gh_info(url);
        assert_eq!(data, None);
    }
}
