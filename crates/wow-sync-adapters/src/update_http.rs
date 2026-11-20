//! The only network code (ops.md §4.4, D-24 P4-1): one anonymous GET of the GitHub Releases
//! "latest" document over the OS TLS stack. No proxy from the environment, no cookies, no
//! credentials, no "accept anyway" path. Only this file may name `ureq` or `native_tls`.

use std::collections::VecDeque;
use std::io::Read;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// A compile-time constant and the only request URL.
pub const API_URL: &str = "https://api.github.com/repos/hamiltonia/wow-sync/releases/latest";
pub const USER_AGENT: &str = "WoW-Sync";
pub const ACCEPT: &str = "application/vnd.github+json";
pub const MAX_RESPONSE_BYTES: usize = 256 * 1024;
pub const MAX_REDIRECTS: usize = 3;
const MAX_URL_BYTES: usize = 2048;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const TOTAL_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchError {
    Offline,
    Timeout,
    Tls,
    Redirect,
    Http(u16),
    TooLarge,
    Other,
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for FetchError {}

pub trait ReleaseSource: Send + Sync {
    /// The raw response body of the latest-release request.
    fn fetch_latest(&self) -> Result<Vec<u8>, FetchError>;
}

/// The request as it is built: exactly these headers, no query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestSpec {
    pub url: String,
    pub headers: Vec<(&'static str, &'static str)>,
}

pub fn request_spec(url: &str) -> RequestSpec {
    RequestSpec {
        url: url.to_string(),
        headers: vec![("User-Agent", USER_AGENT), ("Accept", ACCEPT)],
    }
}

struct Policy {
    hosts: Vec<String>,
    any_port: bool,
}

/// `(host, port, rest)` of an `https://` URL without userinfo.
fn split_https(url: &str) -> Option<(&str, Option<&str>, &str)> {
    let rest = url.strip_prefix("https://")?;
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (auth, tail) = rest.split_at(end);
    if auth.is_empty() || auth.contains('@') || auth.contains('[') {
        return None;
    }
    let (host, port) = match auth.rsplit_once(':') {
        Some((h, p)) => (h, Some(p)),
        None => (auth, None),
    };
    (!host.is_empty()).then_some((host, port, tail))
}

/// The URL to fetch next, or a failure. Nothing is fetched from a rejected target.
fn next_hop(base: &str, location: &str, policy: &Policy) -> Result<String, FetchError> {
    if location.len() > MAX_URL_BYTES || location.chars().any(|c| c.is_control() || c == ' ') {
        return Err(FetchError::Redirect);
    }
    let target = if location.starts_with('/') && !location.starts_with("//") {
        let (h, p, _) = split_https(base).ok_or(FetchError::Redirect)?;
        let port = p.map(|p| format!(":{p}")).unwrap_or_default();
        format!("https://{h}{port}{location}")
    } else {
        location.to_string()
    };
    let (host, port, _) = split_https(&target).ok_or(FetchError::Redirect)?;
    let host_ok = policy.hosts.iter().any(|h| h.eq_ignore_ascii_case(host));
    let port_ok = policy.any_port || matches!(port, None | Some("443"));
    if host_ok && port_ok {
        Ok(target)
    } else {
        Err(FetchError::Redirect)
    }
}

fn map_transport(t: &ureq::Transport) -> FetchError {
    let msg = format!("{t} {:?}", t.message()).to_lowercase();
    let tls = [
        "tls",
        "ssl",
        "certificate",
        "handshake",
        "schannel",
        "security",
        "native_tls",
    ];
    if tls.iter().any(|k| msg.contains(k)) {
        return FetchError::Tls;
    }
    if msg.contains("timed out") || msg.contains("timeout") {
        return FetchError::Timeout;
    }
    match t.kind() {
        ureq::ErrorKind::Dns | ureq::ErrorKind::ConnectionFailed => FetchError::Offline,
        _ => FetchError::Other,
    }
}

pub struct GithubReleases {
    agent: ureq::Agent,
    start_url: String,
    policy: Policy,
}

impl GithubReleases {
    pub fn new() -> Result<GithubReleases, FetchError> {
        let tls = native_tls::TlsConnector::builder()
            .min_protocol_version(Some(native_tls::Protocol::Tlsv12))
            .build()
            .map_err(|_| FetchError::Tls)?;
        Ok(GithubReleases {
            agent: agent(tls, false),
            start_url: API_URL.to_string(),
            policy: Policy {
                hosts: vec!["api.github.com".into(), "github.com".into()],
                any_port: false,
            },
        })
    }

    /// Test-only: a local endpoint on `localhost`, trusting the given DER roots.
    #[cfg(test)]
    fn for_test(url: &str, roots: &[&[u8]]) -> GithubReleases {
        let mut b = native_tls::TlsConnector::builder();
        for r in roots {
            b.add_root_certificate(native_tls::Certificate::from_der(r).unwrap());
        }
        GithubReleases {
            agent: agent(b.build().unwrap(), false),
            start_url: url.to_string(),
            policy: Policy {
                hosts: vec!["localhost".into()],
                any_port: true,
            },
        }
    }
}

fn agent(tls: native_tls::TlsConnector, env_proxy: bool) -> ureq::Agent {
    ureq::AgentBuilder::new()
        .try_proxy_from_env(env_proxy)
        .redirects(0)
        .tls_connector(Arc::new(tls))
        .timeout_connect(CONNECT_TIMEOUT)
        .timeout(TOTAL_TIMEOUT)
        .user_agent(USER_AGENT)
        .build()
}

impl ReleaseSource for GithubReleases {
    fn fetch_latest(&self) -> Result<Vec<u8>, FetchError> {
        let mut url = self.start_url.clone();
        for hop in 0..=MAX_REDIRECTS {
            let spec = request_spec(&url);
            let resp = match self.agent.get(&spec.url).set("Accept", ACCEPT).call() {
                Ok(r) => r,
                Err(ureq::Error::Status(code, _)) => return Err(FetchError::Http(code)),
                Err(ureq::Error::Transport(t)) => return Err(map_transport(&t)),
            };
            let status = resp.status();
            if (300..400).contains(&status) {
                if hop == MAX_REDIRECTS {
                    return Err(FetchError::Redirect);
                }
                let loc = resp.header("Location").ok_or(FetchError::Redirect)?;
                url = next_hop(&url, loc, &self.policy)?;
                continue;
            }
            if status != 200 {
                return Err(FetchError::Http(status));
            }
            let mut body = Vec::new();
            resp.into_reader()
                .take(MAX_RESPONSE_BYTES as u64 + 1)
                .read_to_end(&mut body)
                .map_err(|e| {
                    if e.kind() == std::io::ErrorKind::TimedOut {
                        FetchError::Timeout
                    } else {
                        FetchError::Other
                    }
                })?;
            if body.len() > MAX_RESPONSE_BYTES {
                return Err(FetchError::TooLarge);
            }
            return Ok(body);
        }
        Err(FetchError::Redirect)
    }
}

type Hook = Box<dyn Fn(u32) + Send + Sync>;

/// Scripted source for tests. `hook(call_number)` runs before each result and may block or
/// panic to model a worker dying mid-request.
#[cfg(any(test, feature = "testkit"))]
pub struct FakeReleases {
    script: Mutex<VecDeque<Result<Vec<u8>, FetchError>>>,
    calls: AtomicU32,
    hook: Mutex<Option<Arc<Hook>>>,
}

#[cfg(any(test, feature = "testkit"))]
impl FakeReleases {
    pub fn new(script: Vec<Result<Vec<u8>, FetchError>>) -> FakeReleases {
        FakeReleases {
            script: Mutex::new(script.into()),
            calls: AtomicU32::new(0),
            hook: Mutex::new(None),
        }
    }

    pub fn set_hook(&self, hook: impl Fn(u32) + Send + Sync + 'static) {
        *self.hook.lock().unwrap() = Some(Arc::new(Box::new(hook)));
    }

    pub fn calls(&self) -> u32 {
        self.calls.load(Ordering::SeqCst)
    }
}

#[cfg(any(test, feature = "testkit"))]
impl ReleaseSource for FakeReleases {
    fn fetch_latest(&self) -> Result<Vec<u8>, FetchError> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        let hook = self.hook.lock().unwrap().clone();
        if let Some(h) = hook {
            h(n);
        }
        self.script
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(Err(FetchError::Offline))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn policy() -> Policy {
        Policy {
            hosts: vec!["api.github.com".into(), "github.com".into()],
            any_port: false,
        }
    }

    #[test]
    fn request_has_exactly_the_documented_headers_and_no_query() {
        let s = request_spec(API_URL);
        assert_eq!(
            s.headers,
            vec![
                ("User-Agent", "WoW-Sync"),
                ("Accept", "application/vnd.github+json")
            ]
        );
        assert!(!s.url.contains('?') && !s.url.contains('@') && !s.url.contains('#'));
        assert_eq!(s.url, API_URL);
        assert!(USER_AGENT
            .chars()
            .all(|c| c.is_ascii_alphabetic() || c == '-'));
    }

    #[test]
    fn redirects_follow_only_https_on_the_two_hosts() {
        let base = API_URL;
        let p = policy();
        for ok in [
            "https://github.com/hamiltonia/wow-sync/releases/latest",
            "https://api.github.com/x",
            "https://API.GITHUB.COM/x",
            "https://github.com:443/x",
            "/other/path",
        ] {
            assert!(next_hop(base, ok, &p).is_ok(), "{ok}");
        }
        assert_eq!(
            next_hop(base, "/other", &p).unwrap(),
            "https://api.github.com/other"
        );
        for bad in [
            "http://github.com/x",
            "https://evil.example/x",
            "https://github.com.evil.example/x",
            "https://evil.example@github.com/x",
            "https://github.com@evil.example/x",
            "https://github.com:8443/x",
            "//evil.example/x",
            "ftp://github.com/x",
            "HTTPS://github.com/x",
            "https://github.com/a b",
            "https://github.com/a\r\nX: y",
            "https://127.0.0.1/x",
            "https://[::1]/x",
            "",
        ] {
            assert_eq!(
                next_hop(base, bad, &p),
                Err(FetchError::Redirect),
                "{bad:?}"
            );
        }
        let long = format!("https://github.com/{}", "a".repeat(MAX_URL_BYTES));
        assert_eq!(next_hop(base, &long, &p), Err(FetchError::Redirect));
    }

    #[test]
    fn fake_runs_its_hook_counts_calls_and_defaults_to_offline() {
        let f = FakeReleases::new(vec![Ok(b"x".to_vec()), Err(FetchError::Timeout)]);
        let seen = Arc::new(Mutex::new(Vec::new()));
        let s2 = seen.clone();
        f.set_hook(move |n| s2.lock().unwrap().push(n));
        assert_eq!(f.fetch_latest(), Ok(b"x".to_vec()));
        assert_eq!(f.fetch_latest(), Err(FetchError::Timeout));
        assert_eq!(f.fetch_latest(), Err(FetchError::Offline));
        assert_eq!(f.calls(), 3);
        assert_eq!(*seen.lock().unwrap(), vec![1, 2, 3]);
    }

    fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if p.is_dir() {
                if !matches!(name.as_str(), "target" | ".git" | "node_modules") {
                    rs_files(&p, out);
                }
            } else if matches!(p.extension().and_then(|e| e.to_str()), Some("rs" | "toml")) {
                out.push(p);
            }
        }
    }

    /// Network-code violations in `files` (path, text). `allowed_dep_manifest` is the one
    /// manifest that may name the transport crates.
    fn network_violations(root: &Path, files: &[(PathBuf, String)]) -> Vec<String> {
        let transport = [
            concat!("ure", "q::"),
            concat!("use ure", "q"),
            concat!("native", "_tls::"),
            concat!("use native", "_tls"),
        ];
        let mut bad = Vec::new();
        for (p, text) in files {
            let rel = p.strip_prefix(root).unwrap_or(p);
            let rel_s = rel.to_string_lossy().replace('\\', "/");
            let is_http = rel_s == "crates/wow-sync-adapters/src/update_http.rs";
            let is_xtask = rel_s.starts_with("xtask/");
            let is_toml = rel_s.ends_with(".toml");
            for (n, line) in text.lines().enumerate() {
                let loc = format!("{rel_s}:{}", n + 1);
                if is_toml {
                    let manifest = rel_s == "crates/wow-sync-adapters/Cargo.toml";
                    let names = line.contains(concat!("ure", "q ="))
                        || line.contains(concat!("native", "-tls ="));
                    if names && !manifest {
                        bad.push(format!(
                            "{loc}: transport dependency outside the adapters manifest"
                        ));
                    }
                    continue;
                }
                if !is_http && transport.iter().any(|t| line.contains(t)) {
                    bad.push(format!(
                        "{loc}: transport crate used outside update_http.rs"
                    ));
                }
                if !is_http
                    && !is_xtask
                    && line.contains("https://")
                    && !line.contains("github.com/hamiltonia/wow-sync/")
                {
                    bad.push(format!("{loc}: unlisted https:// literal"));
                }
            }
        }
        bad
    }

    fn workspace_files() -> (PathBuf, Vec<(PathBuf, String)>) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
        let root = root.canonicalize().unwrap();
        let mut paths = Vec::new();
        rs_files(&root, &mut paths);
        let files = paths
            .into_iter()
            .map(|p| {
                let t = std::fs::read_to_string(&p).unwrap_or_default();
                (p, t)
            })
            .collect();
        (root, files)
    }

    #[test]
    fn only_update_http_names_the_transport_or_an_unlisted_url() {
        let (root, mut files) = workspace_files();
        assert!(files.len() > 30, "scan found the workspace");
        assert_eq!(network_violations(&root, &files), Vec::<String>::new());

        // Control: each kind of plant is caught.
        let src = root
            .join("crates")
            .join("wow-sync-core")
            .join("src")
            .join("lib.rs");
        for plant in [
            concat!("fn f() { ure", "q::get(\"x\"); }"),
            concat!("use native", "_tls::TlsConnector;"),
            "const U: &str = \"https://example.com/a\";",
        ] {
            let mut f = files.clone();
            f.push((src.clone(), plant.to_string()));
            assert_eq!(network_violations(&root, &f).len(), 1, "{plant}");
        }
        let manifest = root.join("crates").join("wow-sync-core").join("Cargo.toml");
        files.push((manifest, concat!("ure", "q = \"2\"").to_string()));
        assert_eq!(network_violations(&root, &files).len(), 1);
    }
}

#[cfg(all(test, windows))]
mod tls_tests {
    use super::*;
    use std::io::Write;
    use std::net::{TcpListener, TcpStream};
    use std::sync::atomic::AtomicBool;

    fn b64(s: &str) -> Vec<u8> {
        let mut out = Vec::new();
        let (mut acc, mut bits) = (0u32, 0);
        for c in s.bytes().filter(|c| *c != b'=') {
            let v = match c {
                b'A'..=b'Z' => c - b'A',
                b'a'..=b'z' => c - b'a' + 26,
                b'0'..=b'9' => c - b'0' + 52,
                b'+' => 62,
                b'/' => 63,
                _ => panic!("bad base64"),
            } as u32;
            acc = (acc << 6) | v;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                out.push((acc >> bits) as u8);
                acc &= (1 << bits) - 1;
            }
        }
        out
    }

    // Synthetic self-signed ECDSA P-256 certificates generated for these tests only.
    const GOOD_PFX: &str = "MIIEfwIBAzCCBDsGCSqGSIb3DQEHAaCCBCwEggQoMIIEJDCCAc0GCSqGSIb3DQEHAaCCAb4EggG6MIIBtjCCAbIGCyqGSIb3DQEMCgECoIHMMIHJMBwGCiqGSIb3DQEMAQMwDgQIetb+bHgMD7ICAgfQBIGon4m27kfArfqmAcxM2Uu8ZkTzfNYIz5QpnUEaVPzXbWPcf+6ZC/TRQL+DetDX6atWr/tycIQntL+vdJ6RX8rYfUpzLwvixY7F12Cnb1Bw53ME5gmqhd4nWuQogV8cBosf2MdJKQUltKJmE+0wjTcuk6HqRECcqfEcm24LxtglldTHpe2gonpTTP7DZjbK6F/A28+6YHzNPE3liIlfCm7X7OlJMgb2GO7JMYHTMBMGCSqGSIb3DQEJFTEGBAQBAAAAMF0GCSqGSIb3DQEJFDFQHk4AdABlAC0ANQBiAGIANwBlADQAYgA3AC0AYwA3ADEAMAAtADQANgBjADUALQA4AGMAMwBiAC0AZQBkADEAYwBkADQAMQBlAGEAMQA5ADgwXQYJKwYBBAGCNxEBMVAeTgBNAGkAYwByAG8AcwBvAGYAdAAgAFMAbwBmAHQAdwBhAHIAZQAgAEsAZQB5ACAAUwB0AG8AcgBhAGcAZQAgAFAAcgBvAHYAaQBkAGUAcjCCAk8GCSqGSIb3DQEHBqCCAkAwggI8AgEAMIICNQYJKoZIhvcNAQcBMBwGCiqGSIb3DQEMAQMwDgQI/WgWpEMf7AwCAgfQgIICCKEwJZ+ywLkBEactrA/cs4Fuv62WxfFFVmZc7OBVP7JVZ+uqe8RXbSP6y3BeHFUFwQhKLcpzp4ou2vvLE+iS+wo9/4+tbvNlPHNHwaBHmVtg0O5gIk+yFdQjBGWgSE9+9ZLBPgiyg9Whjsno+e2cLEjJIhZSkg7T8+cVAxuv7MnmbdqlA0SnoqvxyLxpeTl3CX/bDD/bY+1hX02d32Che6TXFscWLM5C70Flha6tnFXNFHTiFGoOAtzzUyN3/nMpKHNkmWdCwVYxQ145u0AccwVPb6zyGTMVpcVJT2cygjsUu0LKb6vrGBRquUhM7XupUG5BRFLQNH/Ah92PqdndXvn5y6p3J3v7brRG/E8M5idAglijvh1p8Z2GCMXFfK7GpqMC1YtLcAi7OWWV3aj0nkkG01xTRGbEMs928NDPf+DGye5mqPikqvcbM4QZUTLfKC4/hXa6Qou4CDs1+xapyjWg2Ubpdk/RjiC3RcU9gObtHsUwP1OQA4W9Pm/ZJ4qaLlmC3UTKKlVzjrJ0TONt4NBGN0GGxLYt/wfB6TXLlXmZ+GDGsbovlPhjEHWj4ZNDowRog6DG91H6l5tepR1EO+TkpJyObbxbMg/DkH78Hz5hdPXYlTSASeA1EQ2EGMaU16BJica62tjPVWlp08x5pcwxQccEyCvcl8/mWbX8iEUNt68CeluW3LowOzAfMAcGBSsOAwIaBBTxAeRjzy9iTwBnmaTbZ12Htw1w4wQUC/17oCRrMT/C7eSqF1u/UvNJ028CAgfQ";
    const GOOD_DER: &str = "MIIBjjCCATSgAwIBAgIQIpjhf5rdiZpGsqERUUl+ZDAKBggqhkjOPQQDAjAUMRIwEAYDVQQDDAlsb2NhbGhvc3QwIBcNMjYxMTE5MjAxNDM0WhgPMjEyNjExMjAyMDE0MzRaMBQxEjAQBgNVBAMMCWxvY2FsaG9zdDBZMBMGByqGSM49AgEGCCqGSM49AwEHA0IABPmYcNafNoHNgQSThi0wAdQkQzx6oDhiReo50/s50HZLx63J2KV7dEofTLdSzCj1Z4c+khzXRrbeghwHmz5KsEejZjBkMA4GA1UdDwEB/wQEAwIFoDAdBgNVHSUEFjAUBggrBgEFBQcDAgYIKwYBBQUHAwEwFAYDVR0RBA0wC4IJbG9jYWxob3N0MB0GA1UdDgQWBBRkY55UOkDYmegQGFZoEvJ9A8oJnDAKBggqhkjOPQQDAgNIADBFAiAaltSAoO8SbSi2L/6osFHCydlA5KAp4uBy6R3zRHxStwIhAJftVtFxrVtN1K+iWuAAdg1I72QlVGP3xzgXBQhuIC7Z";
    const EXPIRED_PFX: &str = "MIIEfwIBAzCCBDsGCSqGSIb3DQEHAaCCBCwEggQoMIIEJDCCAc0GCSqGSIb3DQEHAaCCAb4EggG6MIIBtjCCAbIGCyqGSIb3DQEMCgECoIHMMIHJMBwGCiqGSIb3DQEMAQMwDgQI3i9Yq9McPDYCAgfQBIGox+XW4BSDPTwM0M0HL7B4tJWL8TBRZLZW+dLLLdcTj156SN2u6n7Ak696p0KTRDuauf8OMgxXi1ffQVf5xQM4xrnbfkkd00MSF5Fwo5FvWGSHnD7IiSHuj8c2mC/ZY7fnJmBDzvUjYYvHRrc8TivKtaXbi52TTj88a7oinbKIm3PnJMq6BhWl77FFtHwCCjxvPzCmH02HUZgi/FG55JoC9syRQMc1mCgNMYHTMBMGCSqGSIb3DQEJFTEGBAQBAAAAMF0GCSqGSIb3DQEJFDFQHk4AdABlAC0AMwBmAGQAYQAwADQAZQA0AC0AOABlADUAOQAtADQAMQBmAGIALQBhAGEAZAA5AC0AMwAzADQAMwBiADUAOQA1ADAANwAwAGUwXQYJKwYBBAGCNxEBMVAeTgBNAGkAYwByAG8AcwBvAGYAdAAgAFMAbwBmAHQAdwBhAHIAZQAgAEsAZQB5ACAAUwB0AG8AcgBhAGcAZQAgAFAAcgBvAHYAaQBkAGUAcjCCAk8GCSqGSIb3DQEHBqCCAkAwggI8AgEAMIICNQYJKoZIhvcNAQcBMBwGCiqGSIb3DQEMAQMwDgQIcnqwZFeV/mUCAgfQgIICCFv0YQSRx2FZ6BLvLWgMbvDLxZMRpouAvfgtj59LpeVExPNh02IxqVLSXOnQRKngy2jCY+ihoGQPPgl5ctTfUDcS8XinXXfEcjDZz2/xM8HPQ3gnHgXeEcIPSvb2R8Blznei4+V23KjtXxBAWSJCiPxVZjMqBOV+WFQd1FlB8Df0RUNp2GOknRal/P5FO6ozVKmEU1e242xbFu5MQb9TL1AaIRgzcB9HYDN6dkefDtwY6uZ3DV6F5YsFtC3QowR7qCilzEqDltD+CwwME4WLbxvOmQJrKP5bEGXPVio1PLIPMJ64Gu6VGYNZIEPKx7EsX+2/u7R4ESW4troQ+pN0/s2cumZ2UdtAZ8jHc8TO6ULHfVu233Gk58ADmHnuZUwzN0rHSiopNJs+QKBl1PriKSwxwGJB+9VzdZL0NKSu8Sl9wzstrqq8lf0SU2f6PydazQvnTpbmsnjzdcE4E26gtXSWJto91ZAftePT+/GPAmQfrzAFYFt2tidn6F+zqlZYM/Ix66MBuul/r7wvQmjRCc9b2C0zCOfrb5Ep6JBAk14jB5uV0L4En6XB1bQ8mhuyxEPY3hjBp9rouCfvBQzIRrfmuZOQXgSOMAroQkBHRVUKuV4ECAP4gN7IlWxuH3kxl27pTE68oP7atsQazj5RWg3oRjJpzJk0ayc3sy6+GFoY5H4mOvw09eAwOzAfMAcGBSsOAwIaBBQDieJNjgHVtA+ZwlGjtLlUZLSU+AQUK+IiAdA68KNL40yunr59fmPsAJ8CAgfQ";
    const EXPIRED_DER: &str = "MIIBjDCCATKgAwIBAgIQUic/CeSuTYNJEBI6W7KrxjAKBggqhkjOPQQDAjAUMRIwEAYDVQQDDAlsb2NhbGhvc3QwHhcNMjMxMTIwMjAxNDM0WhcNMjQxMTIwMjAxNDM0WjAUMRIwEAYDVQQDDAlsb2NhbGhvc3QwWTATBgcqhkjOPQIBBggqhkjOPQMBBwNCAAQ2nJcnw9UsdcOLX/SqB+y1DyXArldTCg2wtbFaZGm+Eo7dw4PKsCV+bNEkfNKdpaBKcgXBiZzVepCfCHkvbe7po2YwZDAOBgNVHQ8BAf8EBAMCBaAwHQYDVR0lBBYwFAYIKwYBBQUHAwIGCCsGAQUFBwMBMBQGA1UdEQQNMAuCCWxvY2FsaG9zdDAdBgNVHQ4EFgQUC1ArkKpE7LpztgYhuOIVuxsMnxEwCgYIKoZIzj0EAwIDSAAwRQIhANnukbJu04hWuz7asIm17RubA5QcLTwNRkks504cucBwAiBPxOZBYa1OhERSvqItj8gf9n6NP9CEpK0fSiaA9IPvdQ==";
    const WRONG_PFX: &str = "MIIElwIBAzCCBFMGCSqGSIb3DQEHAaCCBEQEggRAMIIEPDCCAc0GCSqGSIb3DQEHAaCCAb4EggG6MIIBtjCCAbIGCyqGSIb3DQEMCgECoIHMMIHJMBwGCiqGSIb3DQEMAQMwDgQIs6sYBYIJwtcCAgfQBIGo23Y9zem590mi1FZGONaI5jATZcHaA0iLzhSg9gBi2/1btBD+BGlMT/CMtnPSj4odwBrfPhJs9hosE5Ex6hmkUE+9zKwZkYbfCwsMPt7r8yDRulxaYdXVqTEULR91ye2GveJzpkrVbpzICRoOfiBo8yj6V8+B0qMSR5maxVX1wgo6LXleVMkQ9Dlz+bzBEsSsc6wooFsgvHWYqyNm7PGs0QDHapgWc9l9MYHTMBMGCSqGSIb3DQEJFTEGBAQBAAAAMF0GCSqGSIb3DQEJFDFQHk4AdABlAC0AZgAzAGMAYgA0ADEANwA4AC0AMgAyAGYANQAtADQAMgA1ADAALQA4AGMAZABmAC0AZABkAGUAZgA1ADUANwA1AGMAYgAyADkwXQYJKwYBBAGCNxEBMVAeTgBNAGkAYwByAG8AcwBvAGYAdAAgAFMAbwBmAHQAdwBhAHIAZQAgAEsAZQB5ACAAUwB0AG8AcgBhAGcAZQAgAFAAcgBvAHYAaQBkAGUAcjCCAmcGCSqGSIb3DQEHBqCCAlgwggJUAgEAMIICTQYJKoZIhvcNAQcBMBwGCiqGSIb3DQEMAQMwDgQIQgYW6BlxBIACAgfQgIICINw/Q3hZ5kopk5MuDScKNomgnPQMYz3Wc3iczcC2X0PuXNLo/dozZQlKdpeknTIXNaLBq3l5AX1fsmQHkz73LpYg3qd7/IS45h7wpU7vdvs4cH6+GTrrT74NHwj0q3UKUbyADWajKD9Wkqo2yx7Uj6UQZjRwkqc80BNOupv1hkesIP8ykdrbqATOFYxPUyDqEpIfYisWDQKE/R9zUiTkog9egsZf+1IC51dkp0mwryyI2utjqlpe1q1OLoqRbxJ5cTcgEJmr9mVLgdqASzWmK+x2V6itpufnOtGPOoq5JLLRc9IlZnQD/JoirghW86kqj4tQm0Nq2lVE63BtFLETRxk4BjnSSIhXD8vDODO1++h/bY0jLjfH/wrJnz9qwAJ8kWaFNL692dZ61TFapuRABb6jd0c3BHQlHkK7zNlpQlHy18cJWIzv2el9A1BiLy4JiUOKfCvwoCM1P7GpGMTIYab1f+jsUOIb8e/TIscEkkJkPGEi2C92a+exTwd81Ur8s7GK8OMGIaj0nhtJZ6hhkQIUAouk058pBrGjv03P2nxkp9Zn7EfKcJPZ32kNxcfVbsqNPH6+f/CpC2aRw2IYaIYBPfCLzMyGBsHb61D5h1PoHnhl45Eux0HaP7SE5qjJBzxChqsoU7vLsXWp7fao/XTI/F7ir+1pEOQneNzp4MD14zpx4nSz51iM4As1ZIORIJvJ0NVJ6P1rp5vncdY4MRUwOzAfMAcGBSsOAwIaBBSSGbIu8IM3vfFOXbWVXZHHblDmPQQUZ032d0AmW9PyR4w0iru4R3Sn47oCAgfQ";
    const WRONG_DER: &str = "MIIBpzCCAUygAwIBAgIQS0YNYS5cRqdPKQGTyyQWTzAKBggqhkjOPQQDAjAcMRowGAYDVQQDDBF3cm9uZ2hvc3QuaW52YWxpZDAgFw0yNjExMTkyMDE0MzRaGA8yMTI2MTEyMDIwMTQzNFowHDEaMBgGA1UEAwwRd3Jvbmdob3N0LmludmFsaWQwWTATBgcqhkjOPQIBBggqhkjOPQMBBwNCAASoWMcnc2Y0nS9prf6HAVo+tKZ5YA2N6j+cq2TIF4gxRX4e45/voFkWjkUMg5F0JEovYvBJgFbOonYQNxl0gLrUo24wbDAOBgNVHQ8BAf8EBAMCBaAwHQYDVR0lBBYwFAYIKwYBBQUHAwIGCCsGAQUFBwMBMBwGA1UdEQQVMBOCEXdyb25naG9zdC5pbnZhbGlkMB0GA1UdDgQWBBT3qKM7Z9tRa68agmLSMdMSxm+88zAKBggqhkjOPQQDAgNJADBGAiEA8hUpWwNyFqA1R84nUH3jRg/T+8pQpfNIdesRY/atDP4CIQDUNISvJjdXQnQwmTcME8BphOomkF+6YHJcDp+IPyQoAg==";

    struct TestServer {
        port: u16,
        conns: Arc<AtomicU32>,
        requests: Arc<Mutex<Vec<String>>>,
        stop: Arc<AtomicBool>,
        handle: Option<std::thread::JoinHandle<()>>,
    }

    /// `handler(request_number, request_head)` returns the full response bytes.
    fn start(pfx: &str, handler: impl Fn(u32, &str) -> Vec<u8> + Send + 'static) -> TestServer {
        let identity = native_tls::Identity::from_pkcs12(&b64(pfx), "t").unwrap();
        let acceptor = native_tls::TlsAcceptor::new(identity).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let conns = Arc::new(AtomicU32::new(0));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let (c2, r2, s2) = (conns.clone(), requests.clone(), stop.clone());
        let handle = std::thread::spawn(move || {
            for stream in listener.incoming() {
                if s2.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(stream) = stream else { continue };
                c2.fetch_add(1, Ordering::SeqCst);
                let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                let Ok(mut tls) = acceptor.accept(stream) else {
                    continue;
                };
                let mut head = Vec::new();
                let mut buf = [0u8; 1024];
                while !head.windows(4).any(|w| w == b"\r\n\r\n") {
                    match tls.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => head.extend_from_slice(&buf[..n]),
                    }
                }
                let head = String::from_utf8_lossy(&head).to_string();
                if head.is_empty() {
                    continue;
                }
                let n = {
                    let mut r = r2.lock().unwrap();
                    r.push(head.clone());
                    r.len() as u32
                };
                let _ = tls.write_all(&handler(n, &head));
                let _ = tls.flush();
                let _ = tls.shutdown();
            }
        });
        TestServer {
            port,
            conns,
            requests,
            stop,
            handle: Some(handle),
        }
    }

    impl TestServer {
        fn url(&self, path: &str) -> String {
            format!("https://localhost:{}{path}", self.port)
        }
        fn conns(&self) -> u32 {
            self.conns.load(Ordering::SeqCst)
        }
        fn requests(&self) -> Vec<String> {
            self.requests.lock().unwrap().clone()
        }
    }

    impl Drop for TestServer {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
            let _ = TcpStream::connect(("127.0.0.1", self.port));
            if let Some(h) = self.handle.take() {
                let _ = h.join();
            }
        }
    }

    fn respond(status: &str, extra: &str, body: &[u8]) -> Vec<u8> {
        let mut v = format!(
            "HTTP/1.1 {status}\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .into_bytes();
        v.extend_from_slice(body);
        v
    }

    fn ok(body: &str) -> Vec<u8> {
        respond(
            "200 OK",
            "Content-Type: application/json\r\n",
            body.as_bytes(),
        )
    }

    fn redirect(to: &str) -> Vec<u8> {
        respond("302 Found", &format!("Location: {to}\r\n"), b"")
    }

    /// A TCP listener that only counts connections.
    struct Counter {
        port: u16,
        n: Arc<AtomicU32>,
        stop: Arc<AtomicBool>,
        handle: Option<std::thread::JoinHandle<()>>,
    }

    fn counter() -> Counter {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let n = Arc::new(AtomicU32::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let (n2, s2) = (n.clone(), stop.clone());
        let handle = std::thread::spawn(move || {
            for s in l.incoming() {
                if s2.load(Ordering::SeqCst) {
                    break;
                }
                if s.is_ok() {
                    n2.fetch_add(1, Ordering::SeqCst);
                }
            }
        });
        Counter {
            port,
            n,
            stop,
            handle: Some(handle),
        }
    }

    impl Counter {
        fn count(&self) -> u32 {
            self.n.load(Ordering::SeqCst)
        }
    }

    impl Drop for Counter {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
            let _ = TcpStream::connect(("127.0.0.1", self.port));
            if let Some(h) = self.handle.take() {
                let _ = h.join();
            }
        }
    }

    #[test]
    fn a_trusted_valid_certificate_succeeds_and_sends_only_the_documented_request() {
        let s = start(GOOD_PFX, |_, _| ok("{\"tag_name\":\"v1.0.0\"}"));
        let g = GithubReleases::for_test(&s.url("/repos/x/y/releases/latest"), &[&b64(GOOD_DER)]);
        assert_eq!(g.fetch_latest().unwrap(), b"{\"tag_name\":\"v1.0.0\"}");
        let reqs = s.requests();
        assert_eq!(reqs.len(), 1);
        let mut lines = reqs[0].lines();
        assert_eq!(
            lines.next(),
            Some("GET /repos/x/y/releases/latest HTTP/1.1")
        );
        let mut names: Vec<String> = lines
            .filter(|l| !l.is_empty())
            .map(|l| l.split(':').next().unwrap().to_ascii_lowercase())
            .collect();
        names.sort();
        assert_eq!(names, vec!["accept", "host", "user-agent"], "{}", reqs[0]);
        assert!(reqs[0].contains("User-Agent: WoW-Sync\r\n"));
        assert!(reqs[0].contains("Accept: application/vnd.github+json\r\n"));
    }

    #[test]
    fn untrusted_expired_and_wrong_host_certificates_fail_before_any_request() {
        let cases: [(&str, Vec<Vec<u8>>, &str); 3] = [
            (GOOD_PFX, vec![], "self-signed, not trusted"),
            (EXPIRED_PFX, vec![b64(EXPIRED_DER)], "expired"),
            (WRONG_PFX, vec![b64(WRONG_DER)], "wrong host"),
        ];
        for (pfx, roots, why) in cases {
            let s = start(pfx, |_, _| ok("{}"));
            let roots: Vec<&[u8]> = roots.iter().map(|r| r.as_slice()).collect();
            let g = GithubReleases::for_test(&s.url("/x"), &roots);
            assert_eq!(g.fetch_latest(), Err(FetchError::Tls), "{why}");
            assert!(s.conns() >= 1, "{why}: the client did connect");
            assert!(s.requests().is_empty(), "{why}: nothing was sent");
        }
    }

    #[test]
    fn redirect_to_http_to_another_host_and_a_fourth_redirect_fail_without_fetching() {
        let roots = b64(GOOD_DER);

        let plain = counter();
        let to = format!("http://localhost:{}/x", plain.port);
        let s = start(GOOD_PFX, move |_, _| redirect(&to));
        let g = GithubReleases::for_test(&s.url("/a"), &[&roots]);
        assert_eq!(g.fetch_latest(), Err(FetchError::Redirect));
        assert_eq!(plain.count(), 0, "http target never contacted");
        drop(s);

        let other = counter();
        let to = format!("https://127.0.0.1:{}/x", other.port);
        let s = start(GOOD_PFX, move |_, _| redirect(&to));
        let g = GithubReleases::for_test(&s.url("/a"), &[&roots]);
        assert_eq!(g.fetch_latest(), Err(FetchError::Redirect));
        assert_eq!(other.count(), 0, "other host never contacted");
        drop(s);

        // Three redirects are followed; the fourth fails.
        let port_cell = Arc::new(Mutex::new(0u16));
        let pc = port_cell.clone();
        let s = start(GOOD_PFX, move |n, _| {
            let port = *pc.lock().unwrap();
            redirect(&format!("https://localhost:{port}/hop{n}"))
        });
        *port_cell.lock().unwrap() = s.port;
        let g = GithubReleases::for_test(&s.url("/a"), &[&roots]);
        assert_eq!(g.fetch_latest(), Err(FetchError::Redirect));
        assert_eq!(
            s.requests().len(),
            4,
            "initial request plus three redirects"
        );
        drop(s);

        // Positive control: two redirects then success.
        let port_cell = Arc::new(Mutex::new(0u16));
        let pc = port_cell.clone();
        let s = start(GOOD_PFX, move |n, _| {
            let port = *pc.lock().unwrap();
            if n < 3 {
                redirect(&format!("https://localhost:{port}/hop{n}"))
            } else {
                ok("{\"done\":true}")
            }
        });
        *port_cell.lock().unwrap() = s.port;
        let g = GithubReleases::for_test(&s.url("/a"), &[&roots]);
        assert_eq!(g.fetch_latest().unwrap(), b"{\"done\":true}");
    }

    #[test]
    fn http_errors_and_oversized_bodies_fail() {
        let roots = b64(GOOD_DER);
        let s = start(GOOD_PFX, |_, _| {
            respond("403 Forbidden", "", b"rate limited")
        });
        let g = GithubReleases::for_test(&s.url("/a"), &[&roots]);
        assert_eq!(g.fetch_latest(), Err(FetchError::Http(403)));
        drop(s);
        let s = start(GOOD_PFX, |_, _| {
            respond("200 OK", "", &vec![b'a'; MAX_RESPONSE_BYTES + 1])
        });
        let g = GithubReleases::for_test(&s.url("/a"), &[&roots]);
        assert_eq!(g.fetch_latest(), Err(FetchError::TooLarge));
        drop(s);
        let s = start(GOOD_PFX, |_, _| {
            respond("200 OK", "", &vec![b'a'; MAX_RESPONSE_BYTES])
        });
        let g = GithubReleases::for_test(&s.url("/a"), &[&roots]);
        assert_eq!(g.fetch_latest().unwrap().len(), MAX_RESPONSE_BYTES);
    }

    #[test]
    fn an_unreachable_endpoint_is_offline() {
        let port = {
            let l = TcpListener::bind("127.0.0.1:0").unwrap();
            l.local_addr().unwrap().port()
        };
        let g = GithubReleases::for_test(&format!("https://localhost:{port}/x"), &[]);
        assert_eq!(g.fetch_latest(), Err(FetchError::Offline));
    }

    #[test]
    fn environment_proxy_settings_are_ignored() {
        let roots = b64(GOOD_DER);
        let proxy = counter();
        let addr = format!("http://127.0.0.1:{}", proxy.port);
        let s = start(GOOD_PFX, |_, _| ok("{}"));
        let host = s.url("/x");

        let saved: Vec<(&str, Option<String>)> = [
            "HTTPS_PROXY",
            "ALL_PROXY",
            "https_proxy",
            "all_proxy",
            "NO_PROXY",
            "no_proxy",
        ]
        .iter()
        .map(|k| (*k, std::env::var(k).ok()))
        .collect();
        for k in ["NO_PROXY", "no_proxy"] {
            std::env::remove_var(k);
        }
        for k in ["HTTPS_PROXY", "ALL_PROXY", "https_proxy", "all_proxy"] {
            std::env::set_var(k, &addr);
        }

        let g = GithubReleases::for_test(&host, &[&roots]);
        let r = g.fetch_latest();

        // Control: an agent that does read the environment does reach the proxy.
        let tls = native_tls::TlsConnector::new().unwrap();
        let _ = agent(tls, true).get(&host).call();
        let control = proxy.count();

        for (k, v) in saved {
            match v {
                Some(v) => std::env::set_var(k, v),
                None => std::env::remove_var(k),
            }
        }
        assert_eq!(r.unwrap(), b"{}", "direct connection succeeded");
        assert!(control >= 1, "control: env-aware agent used the proxy");
        assert_eq!(control, 1, "only the control agent touched the proxy");
    }
}
