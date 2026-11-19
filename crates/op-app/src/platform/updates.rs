//! Native release metadata checks. Documents and preferences never enter requests.

use serde::Deserialize;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Duration;

pub const RELEASES_URL: &str = "https://github.com/linventif/open-process/releases";
const API_URL: &str = "https://api.github.com/repos/linventif/open-process/releases";
const RESPONSE_LIMIT: u64 = 2 * 1024 * 1024;

pub fn installed_release() -> Option<&'static str> {
    let tag = env!("OP_RELEASE_TAG");
    ReleaseId::parse(tag).map(|_| tag)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableUpdate {
    pub tag: String,
    pub release_url: String,
    pub download_url: String,
}

#[derive(Debug)]
enum ReleaseId {
    Version(semver::Version),
    Main { run: u64, commit: String },
}

impl ReleaseId {
    fn parse(tag: &str) -> Option<Self> {
        if tag.len() > 128 {
            return None;
        }
        if let Some(value) = tag.strip_prefix("main-") {
            let (run, commit) = value.split_once('-')?;
            if run.is_empty()
                || !run.bytes().all(|c| c.is_ascii_digit())
                || !(7..=40).contains(&commit.len())
                || !commit.bytes().all(|c| c.is_ascii_hexdigit())
            {
                return None;
            }
            return Some(Self::Main {
                run: run.parse().ok()?,
                commit: commit.to_ascii_lowercase(),
            });
        }
        semver::Version::parse(tag.strip_prefix('v')?).ok().map(Self::Version)
    }

    fn newer_than(&self, installed: &Self) -> bool {
        match (self, installed) {
            (Self::Version(candidate), Self::Version(current)) => {
                (!current.pre.is_empty() || candidate.pre.is_empty()) && candidate.cmp_precedence(current).is_gt()
            }
            (Self::Main { run, .. }, Self::Main { run: current, .. }) => run > current,
            _ => false,
        }
    }

    fn same_commit(&self, other: &Self) -> bool {
        matches!((self, other), (Self::Main { commit, .. }, Self::Main { commit: other, .. })
            if commit.starts_with(other) || other.starts_with(commit))
    }
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    state: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ReleaseFeed {
    List(Vec<Release>),
    Latest(Release),
}

fn asset_name(os: &str, arch: &str) -> Option<&'static str> {
    match (os, arch) {
        ("windows", "x86_64") => Some("open-process-x86_64-pc-windows-msvc.exe"),
        ("linux", "x86_64") => Some("open-process-x86_64-unknown-linux-gnu.tar.gz"),
        ("macos", "aarch64") => Some("open-process-aarch64-apple-darwin.tar.gz"),
        ("macos", "x86_64") => Some("open-process-x86_64-apple-darwin.tar.gz"),
        _ => None,
    }
}

fn select_update(json: &str, installed: &str, os: &str, arch: &str) -> Result<Option<AvailableUpdate>, String> {
    let current = ReleaseId::parse(installed).ok_or("Unrecognized installed release")?;
    let releases = match serde_json::from_str::<ReleaseFeed>(json).map_err(|e| e.to_string())? {
        ReleaseFeed::List(releases) => releases,
        ReleaseFeed::Latest(release) => vec![release],
    };
    let Some(asset) = asset_name(os, arch) else { return Ok(None) };
    let mut best: Option<(ReleaseId, String)> = None;
    for release in releases {
        if release.draft || !release.assets.iter().any(|a| a.name == asset && a.state == "uploaded") {
            continue;
        }
        let Some(id) = ReleaseId::parse(&release.tag_name) else { continue };
        if matches!(&current, ReleaseId::Version(v) if v.pre.is_empty()) && release.prerelease {
            continue;
        }
        if id.newer_than(&current) && !id.same_commit(&current) && best.as_ref().is_none_or(|(previous, _)| id.newer_than(previous)) {
            best = Some((id, release.tag_name));
        }
    }
    Ok(best.map(|(_, tag)| AvailableUpdate {
        release_url: format!("{RELEASES_URL}/tag/{tag}"),
        download_url: format!("{RELEASES_URL}/download/{tag}/{asset}"),
        tag,
    }))
}

fn fetch_update(installed: &str) -> Result<Option<AvailableUpdate>, String> {
    let stable = matches!(ReleaseId::parse(installed), Some(ReleaseId::Version(v)) if v.pre.is_empty());
    let url = format!("{API_URL}{}", if stable { "/latest" } else { "?per_page=30" });
    let config = ureq::Agent::config_builder()
        .https_only(true)
        .max_redirects(0)
        .timeout_global(Some(Duration::from_secs(8)))
        .build();
    let response = ureq::Agent::new_with_config(config)
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", concat!("Open-Process/", env!("CARGO_PKG_VERSION")))
        .call();
    let mut response = match response {
        Ok(response) => response,
        Err(ureq::Error::StatusCode(404)) => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    let json = response
        .body_mut()
        .with_config()
        .limit(RESPONSE_LIMIT)
        .read_to_string()
        .map_err(|e| e.to_string())?;
    select_update(&json, installed, std::env::consts::OS, std::env::consts::ARCH)
}

pub struct Check {
    result: Receiver<Result<Option<AvailableUpdate>, String>>,
}

impl Check {
    pub fn start(ctx: egui::Context, installed: &'static str) -> std::io::Result<Self> {
        let (sender, result) = mpsc::channel();
        std::thread::Builder::new().name("release-check".into()).spawn(move || {
            let _ = sender.send(fetch_update(installed));
            ctx.request_repaint();
        })?;
        Ok(Self { result })
    }

    pub fn poll(&self) -> Option<Result<Option<AvailableUpdate>, String>> {
        match self.result.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err("Release check stopped".into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn release(tag: &str) -> Value {
        let assets = [
            "open-process-x86_64-pc-windows-msvc.exe",
            "open-process-x86_64-unknown-linux-gnu.tar.gz",
            "open-process-aarch64-apple-darwin.tar.gz",
            "open-process-x86_64-apple-darwin.tar.gz",
        ]
        .map(|name| json!({"name": name, "state": "uploaded"}));
        json!({
            "tag_name": tag,
            "draft": false,
            "prerelease": tag.contains('-'),
            "assets": assets,
        })
    }

    fn select(values: &[Value], current: &str) -> Option<AvailableUpdate> {
        select_update(&serde_json::to_string(values).unwrap(), current, "windows", "x86_64").unwrap()
    }

    #[test]
    fn semantic_versions_compare_numerically_without_preview_or_channel_downgrades() {
        let releases = [
            release("v1.9.9"),
            release("v1.10.0"),
            release("v2.0.0-rc.1"),
            release("main-999-abcdef0"),
        ];
        assert_eq!(select(&releases, "v1.9.0").unwrap().tag, "v1.10.0");
        assert_eq!(select(&releases, "v1.10.0"), None);
        assert_eq!(select(&releases, "v3.0.0"), None);
        assert_eq!(select(&[release("v1.0.0+other")], "v1.0.0+installed"), None);
        assert_eq!(select(&[release("v2.0.0-rc.2")], "v2.0.0-rc.1").unwrap().tag, "v2.0.0-rc.2");
        assert_eq!(select(&[release("v2.0.0")], "v2.0.0-rc.2").unwrap().tag, "v2.0.0");
        let latest = select_update(&release("v1.10.0").to_string(), "v1.9.0", "windows", "x86_64").unwrap();
        assert_eq!(latest.unwrap().tag, "v1.10.0");
    }

    #[test]
    fn main_releases_use_run_numbers_and_ignore_republished_identical_commits() {
        let releases = [
            release("main-99-aaaaaaa"),
            release("main-100-bbbbbbb"),
            release("main-101-bbbbbbb"),
            release("main-999-1234567"),
            release("v99.0.0"),
        ];
        assert_eq!(select(&releases, "main-98-1234567").unwrap().tag, "main-101-bbbbbbb");
        assert_eq!(select(&[release("main-100-abcdef0")], "main-100-abcdef0"), None);
        assert_eq!(select(&releases, "main-1000-ccccccc"), None);
        assert_eq!(
            select(&[release("main-100-ABCDEF0")], "main-99-abcdef0123456789012345678901234567890123"),
            None
        );
    }

    #[test]
    fn drafts_missing_assets_and_incomplete_uploads_are_not_offered() {
        let mut draft = release("v4.0.0");
        draft["draft"] = json!(true);
        let mut missing = release("v5.0.0");
        missing["assets"] = json!([]);
        let mut uploading = release("v6.0.0");
        uploading["assets"][0]["state"] = json!("starter");
        let mut preview = release("v7.0.0");
        preview["prerelease"] = json!(true);
        assert_eq!(
            select(&[draft, missing, uploading, preview, release("v2.0.0")], "v1.0.0")
                .unwrap()
                .tag,
            "v2.0.0"
        );
    }

    #[test]
    fn download_links_match_each_platform_and_ignore_remote_link_fields() {
        let mut release = release("v2.0.0");
        release["html_url"] = json!("https://example.org/fake-release");
        for asset in release["assets"].as_array_mut().unwrap() {
            asset["browser_download_url"] = json!("javascript:alert(1)");
        }
        let json = release.to_string();
        for (os, arch, file) in [
            ("windows", "x86_64", "open-process-x86_64-pc-windows-msvc.exe"),
            ("linux", "x86_64", "open-process-x86_64-unknown-linux-gnu.tar.gz"),
            ("macos", "aarch64", "open-process-aarch64-apple-darwin.tar.gz"),
            ("macos", "x86_64", "open-process-x86_64-apple-darwin.tar.gz"),
        ] {
            let update = select_update(&json, "v1.0.0", os, arch).unwrap().unwrap();
            assert_eq!(update.release_url, format!("{RELEASES_URL}/tag/v2.0.0"));
            assert_eq!(update.download_url, format!("{RELEASES_URL}/download/v2.0.0/{file}"));
        }
        assert_eq!(select_update(&json, "v1.0.0", "linux", "aarch64").unwrap(), None);
    }

    #[test]
    fn malformed_metadata_and_unrecognized_tags_never_create_download_links() {
        for tag in [
            "",
            "local",
            "../../bad",
            "v2.0.0/path",
            "v2.0.0?redirect=bad",
            "main-+1-abcdef0",
            "main-999-not-a-hash",
        ] {
            assert!(ReleaseId::parse(tag).is_none(), "{tag}");
            assert_eq!(select(&[release(tag)], "v1.0.0"), None);
        }
        for json in ["", "null", "{}", "{bad json}", "{\"tag_name\":\"v2.0.0\"}"] {
            assert!(select_update(json, "v1.0.0", "windows", "x86_64").is_err());
        }
        assert_eq!(select(&[], "v1.0.0"), None);
        assert!(select_update("[]", "local", "windows", "x86_64").is_err());
    }

    #[test]
    fn worker_poll_is_nonblocking_and_reports_completion_or_disconnect() {
        let (sender, result) = mpsc::channel();
        let check = Check { result };
        assert!(check.poll().is_none());
        sender.send(Ok(None)).unwrap();
        assert_eq!(check.poll(), Some(Ok(None)));
        assert!(check.poll().is_none());
        sender.send(Err("offline".into())).unwrap();
        assert_eq!(check.poll(), Some(Err("offline".into())));
        drop(sender);
        assert!(check.poll().unwrap().is_err());
    }
}
