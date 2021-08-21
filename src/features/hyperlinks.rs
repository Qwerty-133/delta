use std::borrow::Cow;
use std::str::FromStr;

use lazy_static::lazy_static;
use regex::{Captures, Regex};

use crate::config::Config;
use crate::features::OptionValueFunction;
use crate::git_config::{GitConfig, GitConfigEntry, GitRemoteRepo};

pub fn make_feature() -> Vec<(String, OptionValueFunction)> {
    builtin_feature!([
        (
            "hyperlinks",
            bool,
            None,
            _opt => true
        )
    ])
}

pub fn format_commit_line_with_osc8_commit_hyperlink<'a>(
    line: &'a str,
    config: &Config,
) -> Cow<'a, str> {
    if let Some(commit_link_format) = &config.hyperlinks_commit_link_format {
        COMMIT_LINE_REGEX.replace(line, |captures: &Captures| {
            let commit = captures.get(2).unwrap().as_str();
            format_osc8_hyperlink(&commit_link_format.replace("{commit}", commit), commit)
        })
    } else if let Some(GitConfigEntry::GitRemote(GitRemoteRepo::GitHubRepo(repo))) =
        config.git_config.as_ref().and_then(get_remote_url)
    {
        COMMIT_LINE_REGEX.replace(line, |captures: &Captures| {
            format_commit_line_captures_with_osc8_commit_hyperlink(captures, &repo)
        })
    } else {
        Cow::from(line)
    }
}

fn get_remote_url(git_config: &GitConfig) -> Option<GitConfigEntry> {
    git_config
        .repo
        .as_ref()?
        .find_remote("origin")
        .ok()?
        .url()
        .and_then(|url| {
            GitRemoteRepo::from_str(url)
                .ok()
                .map(GitConfigEntry::GitRemote)
        })
}

/// Create a file hyperlink to `path`, displaying `text`.
pub fn format_osc8_file_hyperlink<'a>(
    relative_path: &'a str,
    line_number: Option<usize>,
    text: &str,
    config: &Config,
) -> Cow<'a, str> {
    if let Some(GitConfigEntry::Path(workdir)) = config.git_config_entries.get("delta.__workdir__")
    {
        let absolute_path = workdir.join(relative_path);
        let mut url = config
            .hyperlinks_file_link_format
            .replace("{path}", &absolute_path.to_string_lossy());
        if let Some(n) = line_number {
            url = url.replace("{line}", &format!("{}", n))
        } else {
            url = url.replace("{line}", "")
        };
        Cow::from(format_osc8_hyperlink(&url, text))
    } else {
        Cow::from(relative_path)
    }
}

fn format_osc8_hyperlink(url: &str, text: &str) -> String {
    format!(
        "{osc}8;;{url}{st}{text}{osc}8;;{st}",
        url = url,
        text = text,
        osc = "\x1b]",
        st = "\x1b\\"
    )
}

lazy_static! {
    static ref COMMIT_LINE_REGEX: Regex = Regex::new("(.* )([0-9a-f]{40})(.*)").unwrap();
}

fn format_commit_line_captures_with_osc8_commit_hyperlink(
    captures: &Captures,
    github_repo: &str,
) -> String {
    let commit = captures.get(2).unwrap().as_str();
    format!(
        "{prefix}{osc}8;;{url}{st}{commit}{osc}8;;{st}{suffix}",
        url = format_github_commit_url(commit, github_repo),
        commit = commit,
        prefix = captures.get(1).unwrap().as_str(),
        suffix = captures.get(3).unwrap().as_str(),
        osc = "\x1b]",
        st = "\x1b\\"
    )
}

fn format_github_commit_url(commit: &str, github_repo: &str) -> String {
    format!("https://github.com/{}/commit/{}", github_repo, commit)
}

#[cfg(test)]
mod tests {
    use std::fs::remove_file;

    use super::format_commit_line_with_osc8_commit_hyperlink;
    use crate::tests::integration_test_utils;

    #[test]
    fn test_commit_hyperlink_honors_insteadof() {
        let git_config_contents = br#"
[remote "origin"]                                                                                                                                                                                                                          
    url = github:dandavison/delta                                                                                                                                                                                                      
    fetch = +refs/heads/*:refs/remotes/origin/*
[url "https://github.com/"]
    insteadOf = github:
[url "ssh://git@github.com/"]
    pushInsteadOf = github:
    insteadOf = githubpriv:
"#;
        let git_config_path = "delta__test_commit_hyperlink_honors_insteadof";
        let config = integration_test_utils::make_config_from_args_and_git_config(
            &[],
            Some(git_config_contents),
            Some(git_config_path),
        );
        let hash = "342016d0a69d8361dc17396d9a441704416eb7bb";
        let line = format!("commit {}", hash);
        // TODO: This doesn't work because a git2::Repository is needed in order
        // to compute the remote URL but we do not actually have a repositoty
        // since the utility make_config_from_args_and_git_config creates a
        // GitConfig struct with a null Repository.
        let formatted = format_commit_line_with_osc8_commit_hyperlink(&line, &config);
        assert!(formatted.contains(&format!(
            "https://github.com/dandavison/delta/commit/{}",
            hash
        )));

        remove_file(git_config_path).unwrap();
    }
}
