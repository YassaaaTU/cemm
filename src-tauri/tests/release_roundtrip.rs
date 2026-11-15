//! The release workflow, end to end, through the compiled binary.
//!
//! Everything here goes through the same process the app spawns as its
//! sidecar, over the same protocol, and that process talks HTTP to a stand-in
//! for GitHub and the CurseForge CDN running in this test. Nothing is stubbed
//! inside CEMM: capture reads real files, publish makes the real sequence of
//! Git Data API calls, fetch and install download and verify real bytes, and
//! the profiles are real directories — throwaway ones under the system temp
//! folder.
//!
//! The stand-in is reachable only because a development build honours
//! `CEMM_DEV_GITHUB_API`, `CEMM_DEV_GITHUB_RAW` and `CEMM_DEV_ADDON_ORIGIN`.
//! A release build never reads them.

use base64::Engine;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

// ---------------------------------------------------------------------------
// A stand-in for GitHub's Git Data API, raw file host and the addon CDN
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Store {
    blobs: HashMap<String, Vec<u8>>,
    trees: HashMap<String, BTreeMap<String, String>>,
    commits: HashMap<String, String>,
    head: String,
    /// Files the "CDN" serves, by name.
    cdn: HashMap<String, Vec<u8>>,
    /// Every request, as `METHOD path`.
    log: Vec<String>,
    /// Refuse the next branch update, as GitHub does when the branch moved.
    reject_next_ref_update: bool,
    /// Cut the connection partway through any response whose path ends so.
    truncate_path_suffix: Option<String>,
    /// Answer this many upcoming blob uploads with a rate-limit refusal.
    rate_limit_next_blobs: u32,
}

impl Store {
    fn id(kind: &str, bytes: &[u8]) -> String {
        sha256(&[kind.as_bytes(), bytes].concat())[..40].to_string()
    }

    fn commit_tree(&mut self, tree: BTreeMap<String, String>, parent: &str) -> String {
        let tree_id = Self::id("tree", &serde_json::to_vec(&tree).unwrap());
        self.trees.insert(tree_id.clone(), tree);
        let commit_id = Self::id("commit", format!("{tree_id}{parent}").as_bytes());
        self.commits.insert(commit_id.clone(), tree_id);
        commit_id
    }

    fn tree_at(&self, reference: &str) -> Option<&BTreeMap<String, String>> {
        let commit = if reference == "main" {
            &self.head
        } else {
            reference
        };
        self.trees.get(self.commits.get(commit)?)
    }

    fn file_at(&self, reference: &str, path: &str) -> Option<&Vec<u8>> {
        self.blobs.get(self.tree_at(reference)?.get(path)?)
    }

    fn head_paths(&self) -> Vec<String> {
        self.tree_at("main")
            .map(|tree| tree.keys().cloned().collect())
            .unwrap_or_default()
    }
}

struct Storage {
    store: Arc<Mutex<Store>>,
    origin: String,
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(value) = u8::from_str_radix(&text[index + 1..index + 3], 16) {
                out.push(value);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn respond(stream: &mut TcpStream, status: &str, content_type: &str, body: &[u8], truncate: bool) {
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n{}\r\n",
        body.len(),
        if truncate {
            "Connection: close\r\n"
        } else {
            ""
        }
    );
    let _ = stream.write_all(header.as_bytes());
    if truncate {
        // Promise the whole body, deliver half, hang up.
        let _ = stream.write_all(&body[..body.len() / 2]);
    } else {
        let _ = stream.write_all(body);
    }
    let _ = stream.flush();
}

/// Serves one connection for as long as the client keeps it open.
///
/// Keep-alive matters here for a mundane reason: a first release of a real
/// pack is several thousand uploads in a row, and a connection apiece runs a
/// Windows machine out of ephemeral ports partway through.
fn handle(mut stream: TcpStream, store: &Arc<Mutex<Store>>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(60)));
    let mut buffer: Vec<u8> = Vec::new();
    let mut chunk = vec![0u8; 64 * 1024];
    loop {
        let header_end = loop {
            if let Some(position) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
                break position + 4;
            }
            match stream.read(&mut chunk) {
                Ok(0) | Err(_) => return,
                Ok(read) => buffer.extend_from_slice(&chunk[..read]),
            }
        };
        let head = String::from_utf8_lossy(&buffer[..header_end]).into_owned();
        let mut lines = head.lines();
        let mut request_line = lines.next().unwrap_or_default().split(' ');
        let method = request_line.next().unwrap_or_default().to_string();
        let target = request_line.next().unwrap_or_default().to_string();
        let content_length = lines
            .filter_map(|line| line.split_once(':'))
            .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
            .and_then(|(_, value)| value.trim().parse::<usize>().ok())
            .unwrap_or(0);
        while buffer.len() < header_end + content_length {
            match stream.read(&mut chunk) {
                Ok(0) | Err(_) => return,
                Ok(read) => buffer.extend_from_slice(&chunk[..read]),
            }
        }
        let body = buffer[header_end..header_end + content_length].to_vec();
        buffer.drain(..header_end + content_length);

        if !serve(&mut stream, method, target, body, store) {
            return;
        }
    }
}

/// Answers one request. Returns whether the connection is still usable.
fn serve(
    stream: &mut TcpStream,
    method: String,
    target: String,
    body: Vec<u8>,
    store: &Arc<Mutex<Store>>,
) -> bool {
    let (raw_path, query) = target.split_once('?').unwrap_or((&target, ""));
    let path = percent_decode(raw_path);
    let mut store = store.lock().unwrap();
    store.log.push(format!("{method} {path}"));
    let truncate = store
        .truncate_path_suffix
        .as_deref()
        .is_some_and(|suffix| path.ends_with(suffix));
    let json_body: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    let not_found = json!({ "message": "Not Found" }).to_string();
    let ok = |stream: &mut TcpStream, value: Value| {
        respond(
            stream,
            "200 OK",
            "application/json",
            value.to_string().as_bytes(),
            false,
        )
    };

    const API: &str = "/api/repos/o/r/";
    if let Some(rest) = path.strip_prefix("/raw/o/r/") {
        let (reference, file) = rest.split_once('/').unwrap_or((rest, ""));
        match store.file_at(reference, file) {
            Some(bytes) => respond(
                stream,
                "200 OK",
                "application/octet-stream",
                bytes,
                truncate,
            ),
            None => respond(
                stream,
                "404 Not Found",
                "text/plain",
                b"404: Not Found",
                false,
            ),
        }
    } else if let Some(name) = path.strip_prefix("/cdn/") {
        match store.cdn.get(name) {
            Some(bytes) => respond(
                stream,
                "200 OK",
                "application/java-archive",
                bytes,
                truncate,
            ),
            None => respond(stream, "404 Not Found", "text/plain", b"missing", false),
        }
    } else if let Some(rest) = path.strip_prefix(API) {
        match (method.as_str(), rest) {
            ("GET", "git/refs/heads/main") => {
                let head = store.head.clone();
                ok(stream, json!({ "object": { "sha": head } }))
            }
            ("GET", "commits/main") => {
                let head = store.head.clone();
                respond(
                    stream,
                    "200 OK",
                    "application/vnd.github.sha",
                    head.as_bytes(),
                    false,
                )
            }
            ("POST", "git/blobs") if store.rate_limit_next_blobs > 0 => {
                store.rate_limit_next_blobs -= 1;
                let refusal = json!({ "message": "You have exceeded a secondary rate limit." });
                let body = refusal.to_string();
                let header = format!(
                    "HTTP/1.1 429 Too Many Requests
Content-Type: application/json
Retry-After: 0
Content-Length: {}

",
                    body.len()
                );
                let _ = stream.write_all(header.as_bytes());
                let _ = stream.write_all(body.as_bytes());
                let _ = stream.flush();
            }
            ("POST", "git/blobs") => {
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(json_body["content"].as_str().unwrap_or_default())
                    .unwrap_or_default();
                let id = Store::id("blob", &bytes);
                store.blobs.insert(id.clone(), bytes);
                respond(
                    stream,
                    "201 Created",
                    "application/json",
                    json!({ "sha": id }).to_string().as_bytes(),
                    false,
                )
            }
            ("POST", "git/trees") => {
                let mut tree = json_body["base_tree"]
                    .as_str()
                    .and_then(|base| store.trees.get(base).cloned())
                    .unwrap_or_default();
                for item in json_body["tree"].as_array().into_iter().flatten() {
                    tree.insert(
                        item["path"].as_str().unwrap_or_default().to_string(),
                        item["sha"].as_str().unwrap_or_default().to_string(),
                    );
                }
                let id = Store::id("tree", &serde_json::to_vec(&tree).unwrap());
                store.trees.insert(id.clone(), tree);
                respond(
                    stream,
                    "201 Created",
                    "application/json",
                    json!({ "sha": id }).to_string().as_bytes(),
                    false,
                )
            }
            ("POST", "git/commits") => {
                let tree = json_body["tree"].as_str().unwrap_or_default().to_string();
                let parent = json_body["parents"][0].as_str().unwrap_or_default();
                let id = Store::id("commit", format!("{tree}{parent}").as_bytes());
                store.commits.insert(id.clone(), tree);
                respond(
                    stream,
                    "201 Created",
                    "application/json",
                    json!({ "sha": id }).to_string().as_bytes(),
                    false,
                )
            }
            ("PATCH", "git/refs/heads/main") => {
                if std::mem::take(&mut store.reject_next_ref_update) {
                    respond(
                        stream,
                        "422 Unprocessable Entity",
                        "application/json",
                        json!({ "message": "Update is not a fast forward" })
                            .to_string()
                            .as_bytes(),
                        false,
                    );
                } else {
                    store.head = json_body["sha"].as_str().unwrap_or_default().to_string();
                    let head = store.head.clone();
                    ok(stream, json!({ "object": { "sha": head } }))
                }
            }
            ("GET", rest) if rest.starts_with("git/commits/") => {
                match store.commits.get(&rest["git/commits/".len()..]) {
                    Some(tree) => {
                        let tree = tree.clone();
                        ok(stream, json!({ "tree": { "sha": tree } }))
                    }
                    None => respond(
                        stream,
                        "404 Not Found",
                        "application/json",
                        not_found.as_bytes(),
                        false,
                    ),
                }
            }
            ("GET", rest) if rest.starts_with("git/trees/") => {
                assert!(query.contains("recursive=1"));
                let entries: Vec<Value> = store
                    .trees
                    .get(&rest["git/trees/".len()..])
                    .into_iter()
                    .flatten()
                    .map(|(path, sha)| json!({ "path": path, "type": "blob", "sha": sha }))
                    .collect();
                ok(stream, json!({ "tree": entries, "truncated": false }))
            }
            ("GET", rest) if rest.starts_with("contents/") => {
                let wanted = &rest["contents/".len()..];
                let prefix = format!("{wanted}/");
                let exists = store
                    .head_paths()
                    .iter()
                    .any(|path| path == wanted || path.starts_with(&prefix));
                if exists {
                    ok(stream, json!([{ "name": "something" }]))
                } else {
                    respond(
                        stream,
                        "404 Not Found",
                        "application/json",
                        not_found.as_bytes(),
                        false,
                    )
                }
            }
            _ => respond(
                stream,
                "404 Not Found",
                "application/json",
                not_found.as_bytes(),
                false,
            ),
        }
    } else {
        respond(
            stream,
            "404 Not Found",
            "text/plain",
            b"unknown route",
            false,
        );
    }
    !truncate
}

impl Storage {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test storage");
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let mut initial = Store::default();
        // A repository that already has a README on `main`, like a real one.
        let readme = Store::id("blob", b"# updates");
        initial.blobs.insert(readme.clone(), b"# updates".to_vec());
        initial.head =
            initial.commit_tree(BTreeMap::from([("README.md".to_string(), readme)]), "root");
        let store = Arc::new(Mutex::new(initial));

        let accepting = Arc::clone(&store);
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let store = Arc::clone(&accepting);
                std::thread::spawn(move || handle(stream, &store));
            }
        });
        Self { store, origin }
    }

    fn with<T>(&self, action: impl FnOnce(&mut Store) -> T) -> T {
        action(&mut self.store.lock().unwrap())
    }

    fn cdn_url(&self, name: &str) -> String {
        format!("{}/cdn/{name}", self.origin)
    }

    fn serve(&self, name: &str, content: &str) {
        self.with(|store| {
            store
                .cdn
                .insert(name.to_string(), content.as_bytes().to_vec())
        });
    }

    fn count(&self, needle: &str) -> usize {
        self.with(|store| {
            store
                .log
                .iter()
                .filter(|line| line.contains(needle))
                .count()
        })
    }
}

// ---------------------------------------------------------------------------
// The compiled binary, in sidecar mode
// ---------------------------------------------------------------------------

struct Sidecar {
    child: Child,
    stdin: ChildStdin,
    lines: mpsc::Receiver<String>,
    next_id: u64,
    /// How long one answer may take. Generous where a whole pack is hashed.
    timeout: Duration,
}

impl Sidecar {
    fn spawn(executable: &Path, storage: Option<&Storage>) -> Self {
        let environment = storage
            .map(|storage| {
                vec![
                    ("CEMM_DEV_GITHUB_API", format!("{}/api", storage.origin)),
                    ("CEMM_DEV_GITHUB_RAW", format!("{}/raw", storage.origin)),
                    ("CEMM_DEV_ADDON_ORIGIN", storage.origin.clone()),
                ]
            })
            .unwrap_or_default();
        Self::spawn_with(executable, &environment, Duration::from_secs(120))
    }

    fn spawn_with(executable: &Path, environment: &[(&str, String)], timeout: Duration) -> Self {
        let mut command = Command::new(executable);
        command
            .arg("--cemm-sidecar-service")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        for (name, value) in environment {
            command.env(name, value);
        }
        let mut child = command.spawn().expect("the CEMM binary should start");
        let stdin = child.stdin.take().expect("stdin");
        let stdout = child.stdout.take().expect("stdout");
        let (sender, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if sender.send(line).is_err() {
                    break;
                }
            }
        });
        let mut sidecar = Self {
            child,
            stdin,
            lines,
            next_id: 0,
            timeout,
        };
        let ready = sidecar.next_message();
        assert_eq!(ready["type"], "ready", "unexpected first message: {ready}");
        sidecar
    }

    fn next_message(&mut self) -> Value {
        let line = self
            .lines
            .recv_timeout(self.timeout)
            .expect("the sidecar should answer");
        serde_json::from_str(&line).expect("sidecar messages are JSON")
    }

    fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        self.next_id += 1;
        let id = self.next_id;
        let request = json!({ "id": id, "method": method, "params": params });
        writeln!(self.stdin, "{request}").expect("write request");
        self.stdin.flush().expect("flush");
        loop {
            let message = self.next_message();
            if message["id"] != id {
                continue;
            }
            match message["type"].as_str() {
                Some("response") => return Ok(message["result"].clone()),
                Some("error") => {
                    return Err(message["error"].as_str().unwrap_or_default().to_string())
                }
                _ => continue, // progress events
            }
        }
    }
}

impl Drop for Sidecar {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

// ---------------------------------------------------------------------------
// Throwaway profiles
// ---------------------------------------------------------------------------

const BRIDGE: u64 = 1530014;
const REPO: &str = "o/r";

struct Profile {
    root: PathBuf,
}

impl Profile {
    fn write(&self, relative: &str, content: &str) {
        let path = self.root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    fn read(&self, relative: &str) -> String {
        std::fs::read_to_string(self.root.join(relative))
            .unwrap_or_else(|error| panic!("{relative}: {error}"))
    }

    fn exists(&self, relative: &str) -> bool {
        self.root.join(relative).exists()
    }

    fn remove(&self, relative: &str) {
        std::fs::remove_file(self.root.join(relative)).unwrap();
    }

    fn path(&self) -> String {
        self.root.to_string_lossy().into_owned()
    }

    fn record(&self, storage: &Storage, project: u64, file: u64, name: &str) -> Value {
        let folder = format!("{}\\mods", self.root.display());
        json!({
            "addonID": project,
            "name": format!("Addon {project}"),
            "modFolderPath": folder,
            "fileNameOnDisk": name,
            "filePaths": [format!("{folder}\\{name}")],
            "isEnabled": true,
            "isLocked": false,
            "webSiteURL": format!("https://www.curseforge.com/minecraft/mc-mods/addon-{project}"),
            "launcherOnly": { "left": "alone" },
            "installedFile": { "id": file, "fileName": name, "downloadUrl": storage.cdn_url(name) },
        })
    }

    fn set_instance(&self, name: &str, addons: Vec<Value>) {
        self.write(
            "minecraftinstance.json",
            &serde_json::to_string_pretty(&json!({
                "baseModLoader": { "name": "neoforge-21.1.265" },
                "gameVersion": "1.21.1",
                "name": name,
                "allocatedMemory": 8192,
                "javaArgsOverride": "-Xmx8G -XX:+UseZGC",
                "installedAddons": addons,
            }))
            .unwrap(),
        );
    }

    fn instance_record(&self, project: u64) -> Option<Value> {
        let instance: Value = serde_json::from_str(&self.read("minecraftinstance.json")).unwrap();
        instance["installedAddons"]
            .as_array()
            .unwrap()
            .iter()
            .find(|addon| addon["addonID"] == project)
            .cloned()
    }

    /// Every file by content, optionally only under some folders.
    fn snapshot(&self, only: &[&str]) -> BTreeMap<String, String> {
        let mut files = BTreeMap::new();
        let mut pending = vec![self.root.clone()];
        while let Some(directory) = pending.pop() {
            for entry in std::fs::read_dir(&directory).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    pending.push(path);
                    continue;
                }
                let relative = path
                    .strip_prefix(&self.root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                if only.is_empty() || only.iter().any(|prefix| relative.starts_with(prefix)) {
                    files.insert(relative, sha256(&std::fs::read(&path).unwrap()));
                }
            }
        }
        files
    }
}

const ADMIN_TWEAKS: &str = "{\n  \"general\": {\n    \"compatibility\": {\n      \"sableCompat\": true\n    }\n  },\n  \"keyActions\": {\n    \"reload\": \"key.keyboard.r\"\n  }\n}\n";
const FRIEND_TWEAKS: &str = "{\n  \"general\": {\n    \"compatibility\": {\n      \"sableCompat\": true\n    }\n  },\n  \"keyActions\": {\n    \"reload\": \"key.mouse.5\",\n    \"melee\": \"key.keyboard.v\"\n  }\n}\n";

fn spec(with_setting: bool) -> String {
    let mut spec = json!({
        "lineage": "test-fusion",
        "name": "Test Fusion",
        "base": { "project_id": 1356598, "file_id": 9571494, "name": "All the Mons 2.0.1" },
        "server_release": { "name": "TestFusion-Server-1", "notes": "matching dedicated server build" },
        "pins": [{ "project_id": BRIDGE, "file_id": 8420412, "reason": "1.2.0 must not be taken" }],
        "content": [
            { "path": "kubejs" },
            { "path": "config/ftbquests/quests" },
            { "path": "mods/atmons-ore-hammer-fix-1.0.0.jar" },
            { "path": "config/tacztweaks.json", "policy": "default" }
        ],
        "known_issues": ["Scorched Guns per-round reload exploit is not fixed."],
        "manual_steps": ["Back up your world before the first launch."]
    });
    if with_setting {
        spec["settings"] = json!([{
            "path": "config/tacztweaks.json",
            "pointer": "/general/compatibility/sableCompat",
            "value": false,
            "reason": "TaCZ handles Sable natively"
        }]);
    }
    serde_json::to_string_pretty(&spec).unwrap()
}

/// What the pack consists of at its first release, written into `profile`.
fn write_first_release_content(profile: &Profile, tweaks: &str) {
    profile.write("mods/alpha-1.jar", "alpha one");
    profile.write("mods/beta-1.jar", "beta one");
    profile.write("mods/bridge-1.1.1.jar", "bridge 1.1.1");
    profile.write("mods/atmons-ore-hammer-fix-1.0.0.jar", "hammer fix");
    profile.write("kubejs/server_scripts/recipes.js", "recipes v1");
    profile.write("kubejs/server_scripts/old.js", "old v1");
    profile.write("kubejs/startup_scripts/items.js", "items v1");
    profile.write(
        "config/ftbquests/quests/chapters/guns.snbt",
        "{ id: \"1A2B\" }",
    );
    profile.write("config/tacztweaks.json", tweaks);
}

fn first_release_records(profile: &Profile, storage: &Storage) -> Vec<Value> {
    vec![
        profile.record(storage, 1, 10, "alpha-1.jar"),
        profile.record(storage, 2, 20, "beta-1.jar"),
        profile.record(storage, BRIDGE, 8420412, "bridge-1.1.1.jar"),
    ]
}

/// A friend's profile: the first release's export, imported, then lived in.
fn import_first_release(temp: &Path, name: &str, storage: &Storage) -> Profile {
    let profile = Profile {
        root: temp.join(name),
    };
    write_first_release_content(&profile, FRIEND_TWEAKS);
    profile.write("mods/friends-own-minimap.jar", "added by the friend");
    profile.write(
        "options.txt",
        "fov:95\nkey_key.inventory:key.keyboard.tab\n",
    );
    profile.write("saves/Our World/level.dat", "the friend's world");
    profile.write("local/ftbquests/progress.snbt", "claimed: [1A2B]");
    let mut records = first_release_records(&profile, storage);
    records.push(profile.record(storage, 4242, 1, "friends-own-minimap.jar"));
    profile.set_instance(name, records);
    profile
}

struct Session {
    sidecar: Sidecar,
    operation: u64,
}

impl Session {
    fn capture(&mut self, profile: &Profile, release_id: &str, previous: Option<&Value>) -> Value {
        let draft = self
            .sidecar
            .call(
                "release.capture",
                json!({
                    "modpackPath": profile.path(),
                    "releaseId": release_id,
                    "notes": format!("Notes for {release_id}"),
                    "previous": previous,
                }),
            )
            .expect("capture");
        assert_eq!(draft["errors"], json!([]), "capture reported errors");
        draft
    }

    fn publish(&mut self, profile: &Profile, release: &Value) -> Result<Value, String> {
        self.operation += 1;
        self.sidecar.call(
            "release.publish",
            json!({
                "operationId": format!("publish-{}", self.operation),
                "repo": REPO,
                "token": "test-token",
                "modpackPath": profile.path(),
                "release": release,
            }),
        )
    }

    fn fetch(&mut self, code: &str) -> Result<Value, String> {
        self.sidecar.call(
            "release.fetch",
            json!({ "repo": REPO, "code": code, "modpackKey": null }),
        )
    }

    fn plan(&mut self, profile: &Profile, fetched: &Value) -> Value {
        self.sidecar
            .call(
                "release.plan",
                json!({
                    "modpackPath": profile.path(),
                    "release": fetched["release"],
                    "releaseDigest": fetched["digest"],
                }),
            )
            .expect("plan")
    }

    fn apply(&mut self, profile: &Profile, fetched: &Value, plan: &Value) -> Result<Value, String> {
        self.operation += 1;
        self.sidecar.call(
            "release.apply",
            json!({
                "operationId": format!("apply-{}", self.operation),
                "modpackPath": profile.path(),
                "repo": REPO,
                "release": fetched["release"],
                "releaseDigest": fetched["digest"],
                "revision": fetched["revision"],
                "planFingerprint": plan["fingerprint"],
                "approved": [],
            }),
        )
    }

    /// Fetch, review and install, the way a player does.
    fn install(&mut self, profile: &Profile, code: &str) -> Result<Value, String> {
        let fetched = self.fetch(code)?;
        let plan = self.plan(profile, &fetched);
        if let Some(blocker) = plan["blockers"]
            .as_array()
            .and_then(|blockers| blockers.first())
        {
            return Err(blocker["message"].as_str().unwrap_or_default().to_string());
        }
        self.apply(profile, &fetched, &plan)
    }
}

fn codes_of(blockers: &Value) -> Vec<String> {
    blockers
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|blocker| blocker["code"].as_str().map(str::to_string))
        .collect()
}

#[test]
fn a_pack_is_published_adopted_updated_and_recovered_through_the_real_binary() {
    let storage = Storage::start();
    let temp = tempfile::tempdir().expect("temp dir");
    let mut session = Session {
        sidecar: Sidecar::spawn(Path::new(env!("CARGO_BIN_EXE_cemm")), Some(&storage)),
        operation: 0,
    };
    for name in ["alpha-1.jar", "beta-1.jar", "bridge-1.1.1.jar"] {
        storage.serve(name, "unused: every profile already has these");
    }

    // ---- The admin's profile, and the first release -----------------------------------
    let admin = Profile {
        root: temp.path().join("All the Mons Fusion"),
    };
    write_first_release_content(&admin, ADMIN_TWEAKS);
    admin.write(
        "config/ftbquests/quests/chapters/guns.snbt_merged",
        "generated by the game",
    );
    admin.write("options.txt", "the admin's own options");
    admin.write("saves/Admin Test World/level.dat", "the admin's test world");
    admin.write(".env", "GITHUB_TOKEN=not-for-publication");
    admin.set_instance(
        "All the Mons Fusion",
        first_release_records(&admin, &storage),
    );
    admin.write("cemm-pack.json", &spec(false));

    assert!(session
        .sidecar
        .call("release.read_spec", json!({ "modpackPath": admin.path() }))
        .expect("read spec")
        .is_object());

    let first_draft = session.capture(&admin, "r1", None);
    let head_before = storage.with(|store| store.head.clone());
    let first_receipt = session
        .publish(&admin, &first_draft["release"])
        .expect("publish r1");
    let first_code = first_receipt["code"].as_str().unwrap().to_string();
    let first_revision = first_receipt["revision"].as_str().unwrap().to_string();
    assert_eq!(first_code, format!("test-fusion/r1@{first_revision}"));
    assert_eq!(storage.with(|store| store.head.clone()), first_revision);
    assert_ne!(first_revision, head_before);
    // One commit, one branch move: the release appeared all at once.
    assert_eq!(storage.count("PATCH /api/repos/o/r/git/refs/heads/main"), 1);
    assert_eq!(first_receipt["uploaded_objects"], 6);

    // What is public is the pack, and only the pack.
    let published = storage.with(|store| store.head_paths());
    assert!(published.contains(&"test-fusion/r1/cemm-release.json".to_string()));
    assert!(published.contains(&"test-fusion/r1/cemm-manifest.json".to_string()));
    assert!(
        published.contains(&"README.md".to_string()),
        "existing content is kept"
    );
    let local_jar = format!(
        "test-fusion/objects/{}/{}",
        &sha256(b"hammer fix")[..2],
        sha256(b"hammer fix")
    );
    assert!(
        published.contains(&local_jar),
        "the local jar is hosted by content hash"
    );
    let everything: Vec<u8> =
        storage.with(|store| store.blobs.values().flatten().copied().collect());
    let everything = String::from_utf8_lossy(&everything);
    for private in [
        "not-for-publication",
        "the admin's own options",
        "the admin's test world",
        "generated by the game",
        "-Xmx8G",
    ] {
        assert!(
            !everything.contains(private),
            "'{private}' must never be uploaded"
        );
    }

    // ---- What a client that predates releases makes of it ------------------------------
    let legacy_view = session
        .sidecar
        .call(
            "github.download_manifest",
            json!({ "repo": REPO, "uuid": "test-fusion/r1", "modpackKey": null }),
        )
        .expect_err("the legacy manifest path must not yield a manifest");
    assert!(
        legacy_view.contains("This update needs CEMM 2.2.0 or newer"),
        "got: {legacy_view}"
    );

    // ---- A friend imports the export and adopts it with a short code -------------------
    let friend = import_first_release(temp.path(), "All the Mons Fusion (1)", &storage);
    let friend_before = friend.snapshot(&[]);
    let fetched = session
        .fetch("test-fusion/r1")
        .expect("fetch by short code");
    assert_eq!(fetched["kind"], "release");
    assert_eq!(
        fetched["revision"],
        first_revision.as_str(),
        "pinned at fetch time"
    );
    assert_eq!(fetched["code"], first_code.as_str());
    assert_eq!(fetched["digest"], first_receipt["release_digest"]);

    let plan = session.plan(&friend, &fetched);
    assert_eq!(plan["mode"], "adopt");
    assert_eq!(plan["blockers"], json!([]));
    assert_eq!(plan["change_count"], 0);
    assert_eq!(
        plan["download_count"], 0,
        "an imported profile downloads nothing"
    );
    assert_eq!(plan["extras"][0]["path"], "mods/friends-own-minimap.jar");
    let report = session.apply(&friend, &fetched, &plan).expect("adopt");
    assert_eq!(report["change_count"], 0);

    let friend_after = friend.snapshot(&[]);
    for (path, hash) in &friend_before {
        if path == "minecraftinstance.json" {
            continue;
        }
        assert_eq!(
            friend_after.get(path),
            Some(hash),
            "{path} changed during adoption"
        );
    }
    // Same profile, same name; only the pin was added to the launcher record.
    assert_eq!(friend.instance_record(BRIDGE).unwrap()["isLocked"], true);
    let instance: Value = serde_json::from_str(&friend.read("minecraftinstance.json")).unwrap();
    assert_eq!(instance["name"], "All the Mons Fusion (1)");
    assert_eq!(instance["javaArgsOverride"], "-Xmx8G -XX:+UseZGC");
    let status = session
        .sidecar
        .call("release.status", json!({ "modpackPath": friend.path() }))
        .expect("status");
    assert_eq!(status["release_id"], "r1");
    assert_eq!(status["revision"], first_revision.as_str());

    // ---- Second release: an update, an edit, a removal, and a required setting ---------
    storage.serve("alpha-2.jar", "alpha two");
    admin.remove("mods/alpha-1.jar");
    admin.write("mods/alpha-2.jar", "alpha two");
    admin.write("kubejs/server_scripts/recipes.js", "recipes v2");
    admin.remove("kubejs/server_scripts/old.js");
    admin.write(
        "config/tacztweaks.json",
        &ADMIN_TWEAKS.replace("true", "false"),
    );
    admin.write("cemm-pack.json", &spec(true));
    admin.set_instance(
        "All the Mons Fusion",
        vec![
            admin.record(&storage, 1, 11, "alpha-2.jar"),
            admin.record(&storage, 2, 20, "beta-1.jar"),
            admin.record(&storage, BRIDGE, 8420412, "bridge-1.1.1.jar"),
        ],
    );
    let previous = session
        .fetch(&first_code)
        .expect("the admin fetches r1 as the base");
    let second_draft = session.capture(&admin, "r2", Some(&previous["release"]));
    assert_eq!(second_draft["release"]["sequence"], 2);
    // GitHub pushes back on the uploads three times; the publish waits as told
    // and carries on, rather than failing a release for being large.
    storage.with(|store| store.rate_limit_next_blobs = 3);
    let uploads_before = storage.count("POST /api/repos/o/r/git/blobs");
    let second_receipt = session
        .publish(&admin, &second_draft["release"])
        .expect("publish r2");
    assert_eq!(storage.with(|store| store.rate_limit_next_blobs), 0);
    assert_eq!(
        storage.count("POST /api/repos/o/r/git/blobs") - uploads_before,
        // two payloads, three refusals, and the two release documents
        2 + 3 + 2
    );
    // Only what this release introduces is uploaded: one script, one config.
    assert_eq!(second_receipt["uploaded_objects"], 2);
    assert_eq!(second_receipt["reused_objects"], 3);

    // ---- Third release: alpha again, beta dropped --------------------------------------
    storage.serve("alpha-3.jar", "alpha three");
    admin.remove("mods/alpha-2.jar");
    admin.remove("mods/beta-1.jar");
    admin.write("mods/alpha-3.jar", "alpha three");
    admin.write("kubejs/startup_scripts/items.js", "items v2");
    admin.set_instance(
        "All the Mons Fusion",
        vec![
            admin.record(&storage, 1, 12, "alpha-3.jar"),
            admin.record(&storage, BRIDGE, 8420412, "bridge-1.1.1.jar"),
        ],
    );
    let previous = session
        .fetch(second_receipt["code"].as_str().unwrap())
        .expect("fetch r2");
    let third_draft = session.capture(&admin, "r3", Some(&previous["release"]));
    let third_receipt = session
        .publish(&admin, &third_draft["release"])
        .expect("publish r3");
    let third_code = third_receipt["code"].as_str().unwrap().to_string();

    // A release never changes: the same id cannot be published again.
    let head = storage.with(|store| store.head.clone());
    let refused = session
        .publish(&admin, &third_draft["release"])
        .expect_err("republishing must be refused");
    assert!(refused.contains("already published"), "got: {refused}");
    assert_eq!(storage.with(|store| store.head.clone()), head);

    // ---- A second friend, kept back for the failure cases ------------------------------
    let unlucky = import_first_release(temp.path(), "Fusion (unlucky)", &storage);
    session.install(&unlucky, &first_code).expect("adopt r1");
    let unlucky_before = unlucky.snapshot(&[]);
    let third_fetched = session.fetch(&third_code).expect("fetch r3");
    let third_plan = session.plan(&unlucky, &third_fetched);
    assert_eq!(third_plan["blockers"], json!([]));

    // The CDN serves different bytes of the same length under the right name.
    storage.serve("alpha-3.jar", "ALPHA THREE");
    let error = session
        .apply(&unlucky, &third_fetched, &third_plan)
        .expect_err("a wrong hash must fail the install");
    assert!(
        error.contains("did not arrive as published"),
        "got: {error}"
    );
    assert_eq!(
        unlucky.snapshot(&[]),
        unlucky_before,
        "wrong hash: profile changed"
    );

    // Or something larger than the release says the file is.
    storage.serve("alpha-3.jar", "a much larger file than was published");
    let error = session
        .apply(&unlucky, &third_fetched, &third_plan)
        .expect_err("an oversized download must fail the install");
    assert!(
        error.contains("could not be fetched as published"),
        "got: {error}"
    );
    assert!(error.contains("Nothing was changed"), "got: {error}");
    assert_eq!(
        unlucky.snapshot(&[]),
        unlucky_before,
        "oversize: profile changed"
    );

    // The connection drops halfway through a script.
    storage.serve("alpha-3.jar", "alpha three");
    let items_object = sha256(b"items v2");
    storage.with(|store| store.truncate_path_suffix = Some(items_object.clone()));
    assert!(session
        .apply(&unlucky, &third_fetched, &third_plan)
        .is_err());
    assert_eq!(
        unlucky.snapshot(&[]),
        unlucky_before,
        "interrupted download: profile changed"
    );
    storage.with(|store| store.truncate_path_suffix = None);
    assert!(!unlucky.exists(".cemm-transaction"));
    assert_eq!(
        session
            .sidecar
            .call("release.status", json!({ "modpackPath": unlucky.path() }))
            .unwrap()["release_id"],
        "r1",
        "a failed install is never recorded as applied"
    );

    // ---- The first friend skips r2 and goes straight to r3 -----------------------------
    let fetched = session.fetch(&third_code).expect("fetch r3");
    let plan = session.plan(&friend, &fetched);
    assert_eq!(plan["mode"], "upgrade");
    assert_eq!(plan["from_release"], "r1");
    assert_eq!(plan["blockers"], json!([]));
    assert_eq!(plan["conflicts"], json!([]));
    let report = session
        .apply(&friend, &fetched, &plan)
        .expect("upgrade r1 -> r3");

    // The pack content is now exactly the admin's.
    let pack_folders = ["mods/", "kubejs/", "config/ftbquests/"];
    let mut friend_pack = friend.snapshot(&pack_folders);
    friend_pack.remove("mods/friends-own-minimap.jar");
    let mut admin_pack = admin.snapshot(&pack_folders);
    admin_pack.retain(|path, _| !path.ends_with(".snbt_merged"));
    assert_eq!(friend_pack, admin_pack);
    assert_eq!(friend.read("mods/alpha-3.jar"), "alpha three");
    assert!(!friend.exists("mods/alpha-1.jar"));
    assert!(!friend.exists("mods/beta-1.jar"));
    assert!(!friend.exists("kubejs/server_scripts/old.js"));
    assert_eq!(friend.read("mods/bridge-1.1.1.jar"), "bridge 1.1.1");
    assert_eq!(
        friend.read("mods/atmons-ore-hammer-fix-1.0.0.jar"),
        "hammer fix"
    );

    // The required flag is set and the friend's own keys are exactly theirs.
    let tweaks: Value = serde_json::from_str(&friend.read("config/tacztweaks.json")).unwrap();
    let before: Value = serde_json::from_str(FRIEND_TWEAKS).unwrap();
    assert_eq!(tweaks["general"]["compatibility"]["sableCompat"], false);
    assert_eq!(tweaks["keyActions"], before["keyActions"]);

    // Everything personal is as it was.
    assert_eq!(
        friend.read("options.txt"),
        "fov:95\nkey_key.inventory:key.keyboard.tab\n"
    );
    assert_eq!(
        friend.read("saves/Our World/level.dat"),
        "the friend's world"
    );
    assert_eq!(
        friend.read("local/ftbquests/progress.snbt"),
        "claimed: [1A2B]"
    );
    assert_eq!(
        friend.read("mods/friends-own-minimap.jar"),
        "added by the friend"
    );

    // The launcher's record matches the files, and keeps what is its own.
    let alpha = friend.instance_record(1).unwrap();
    assert_eq!(alpha["installedFile"]["id"], 12);
    assert_eq!(alpha["fileNameOnDisk"], "alpha-3.jar");
    assert_eq!(alpha["isEnabled"], true);
    assert_eq!(alpha["launcherOnly"]["left"], "alone");
    assert!(friend.instance_record(2).is_none());
    assert_eq!(friend.instance_record(BRIDGE).unwrap()["isLocked"], true);
    assert_eq!(
        friend.instance_record(BRIDGE).unwrap()["installedFile"]["id"],
        8420412
    );
    assert!(friend.instance_record(4242).is_some());

    // The completion report carries what a player needs next.
    assert_eq!(report["server_release"]["name"], "TestFusion-Server-1");
    assert_eq!(
        report["manual_steps"][0],
        "Back up your world before the first launch."
    );
    assert!(report["known_issues"][0]
        .as_str()
        .unwrap()
        .contains("reload exploit"));
    let backup = friend
        .root
        .join(report["backup_dir"].as_str().expect("backup kept"));
    assert_eq!(
        std::fs::read_to_string(backup.join("files/mods/alpha-1.jar")).unwrap(),
        "alpha one"
    );
    assert_eq!(
        std::fs::read_to_string(backup.join("files/config/tacztweaks.json")).unwrap(),
        FRIEND_TWEAKS
    );

    // Same code again: nothing to do.
    let plan = session.plan(&friend, &fetched);
    assert_eq!(plan["mode"], "reapply");
    assert_eq!(plan["change_count"], 0);
    assert_eq!(plan["download_count"], 0);
    let downloads_before = storage.count("GET /raw/") + storage.count("GET /cdn/");
    let report = session.apply(&friend, &fetched, &plan).expect("reapply");
    assert_eq!(report["change_count"], 0);
    assert_eq!(
        storage.count("GET /raw/") + storage.count("GET /cdn/"),
        downloads_before,
        "a reapply downloads nothing"
    );

    // ---- The unlucky friend retries once the network behaves ---------------------------
    session
        .install(&unlucky, &third_code)
        .expect("retry succeeds");
    let mut unlucky_pack = unlucky.snapshot(&pack_folders);
    unlucky_pack.remove("mods/friends-own-minimap.jar");
    assert_eq!(
        unlucky_pack, admin_pack,
        "both friends end up with the same pack"
    );

    // ---- A code whose content changed is caught; a pinned code cannot change -----------
    let tampered_revision = storage.with(|store| {
        let mut tree = store.tree_at("main").unwrap().clone();
        let mut document: Value = serde_json::from_slice(
            store
                .file_at("main", "test-fusion/r3/cemm-release.json")
                .unwrap(),
        )
        .unwrap();
        document["notes"] = json!("quietly replaced after publication");
        let bytes = serde_json::to_vec(&document).unwrap();
        let blob = Store::id("blob", &bytes);
        store.blobs.insert(blob.clone(), bytes);
        tree.insert("test-fusion/r3/cemm-release.json".to_string(), blob);
        let parent = store.head.clone();
        store.head = store.commit_tree(tree, &parent);
        store.head.clone()
    });
    // The full code still names the original content, whatever `main` says now.
    let pinned = session.fetch(&third_code).expect("fetch pinned");
    assert_eq!(pinned["digest"], third_receipt["release_digest"]);
    assert_eq!(pinned["release"]["notes"], "Notes for r3");
    // The short code follows the branch, and the profile notices.
    let moved = session.fetch("test-fusion/r3").expect("fetch short");
    assert_eq!(moved["revision"], tampered_revision.as_str());
    assert_eq!(
        codes_of(&session.plan(&friend, &moved)["blockers"]),
        vec!["release-reused"]
    );

    // ---- Wrong targets are refused before anything is written --------------------------
    let stranger = Profile {
        root: temp.path().join("Some Other Pack"),
    };
    stranger.write("mods/unrelated.jar", "another pack entirely");
    stranger.set_instance(
        "Some Other Pack",
        vec![stranger.record(&storage, 9001, 1, "unrelated.jar")],
    );
    let stranger_before = stranger.snapshot(&[]);
    assert_eq!(
        codes_of(&session.plan(&stranger, &pinned)["blockers"]),
        vec!["not-this-pack"]
    );
    assert!(session.install(&stranger, &third_code).is_err());
    assert_eq!(stranger.snapshot(&[]), stranger_before);
    assert!(!stranger.exists(".cemm"));

    // ---- A publish that loses the race for the branch publishes nothing ----------------
    admin.write("kubejs/startup_scripts/items.js", "items v3");
    let previous = session.fetch(&third_code).expect("fetch r3");
    let fourth_draft = session.capture(&admin, "r4", Some(&previous["release"]));
    let head = storage.with(|store| {
        store.reject_next_ref_update = true;
        store.head.clone()
    });
    let error = session
        .publish(&admin, &fourth_draft["release"])
        .expect_err("a rejected branch update fails the publish");
    assert!(error.contains("Nothing was published"), "got: {error}");
    assert_eq!(storage.with(|store| store.head.clone()), head);
    assert!(
        session.fetch("test-fusion/r4").is_err(),
        "a failed publish leaves no code"
    );
    // Nothing was left half-published, so the same release id is still free.
    session
        .publish(&admin, &fourth_draft["release"])
        .expect("the retry publishes r4");

    // ---- Legacy codes still travel the same transport ----------------------------------
    let legacy_manifest = json!({
        "updateType": "config",
        "mods": [], "resourcepacks": [], "shaderpacks": [], "datapacks": [],
        "config_files": [{ "filename": "extra.toml", "relative_path": "config/extra.toml" }]
    });
    session
        .sidecar
        .call(
            "github.upload_update",
            json!({
                "operationId": "legacy-publish",
                "repo": REPO, "token": "test-token", "uuid": "1700000000000", "modpackKey": "old-pack",
                "manifest": legacy_manifest,
                "configFiles": [{ "filename": "extra.toml", "relative_path": "config/extra.toml", "content": "legacy = true" }]
            }),
        )
        .expect("legacy publish");
    let legacy = session
        .fetch("old-pack/1700000000000")
        .expect("legacy fetch");
    assert_eq!(legacy["kind"], "legacy");
    assert_eq!(legacy["manifest"]["updateType"], "config");
    // A legacy code is never overwritten either.
    let again = session
        .sidecar
        .call(
            "github.upload_update",
            json!({
                "operationId": "legacy-publish-again",
                "repo": REPO, "token": "test-token", "uuid": "1700000000000", "modpackKey": "old-pack",
                "manifest": legacy_manifest, "configFiles": []
            }),
        )
        .expect_err("an existing code must not be overwritten");
    assert!(again.contains("already exists"), "got: {again}");

    let files = session
        .sidecar
        .call(
            "github.download_config_files",
            json!({ "repo": REPO, "uuid": "old-pack/1700000000000", "modpackKey": null, "manifest": legacy["manifest"] }),
        )
        .expect("legacy payload download");
    let mods_before = friend.snapshot(&["mods/"]);
    session
        .sidecar
        .call(
            "install.apply_update",
            json!({
                "operationId": "legacy-install",
                "modpackPath": friend.path(),
                "manifest": legacy["manifest"],
                "configFiles": files,
                "options": null
            }),
        )
        .expect("a legacy config-only code still installs on a release profile");
    assert_eq!(friend.read("config/extra.toml"), "legacy = true");
    assert_eq!(
        friend.snapshot(&["mods/"]),
        mods_before,
        "config-only removes no addon"
    );
    let status = session
        .sidecar
        .call("release.status", json!({ "modpackPath": friend.path() }))
        .unwrap();
    assert_eq!(status["release_id"], "r3", "still on its release");
    assert_eq!(
        status["legacy_overlays"][0]["paths"][0],
        "config/extra.toml"
    );
    assert!(
        !status["owned"].as_array().unwrap().is_empty(),
        "ownership kept"
    );

    let mut legacy_full = legacy["manifest"].clone();
    legacy_full["updateType"] = json!("full");
    let refused = session
        .sidecar
        .call(
            "install.apply_update",
            json!({
                "operationId": "legacy-full",
                "modpackPath": friend.path(),
                "manifest": legacy_full,
                "configFiles": [],
                "options": { "old_manifest": null, "cleanup_old": true }
            }),
        )
        .expect_err("a legacy full update must be refused on a release profile");
    assert!(refused.contains("release line"), "got: {refused}");
    assert_eq!(friend.snapshot(&["mods/"]), mods_before);
}

/// A released client's own binary, handed what a release publishes.
///
/// Ignored by default: it needs an installed CEMM from before releases existed,
/// named by `CEMM_OLD_BINARY`. It checks the claim that matters most about
/// backward compatibility on the real thing rather than on a copy of its
/// parser: an old client given a release must fail, and must write nothing.
#[test]
#[ignore = "needs CEMM_OLD_BINARY pointing at an installed CEMM older than 2.2.0"]
fn a_released_old_binary_rejects_release_payloads_without_writing_anything() {
    let old_binary = PathBuf::from(std::env::var("CEMM_OLD_BINARY").expect("CEMM_OLD_BINARY"));
    let storage = Storage::start();
    let temp = tempfile::tempdir().expect("temp dir");

    // Publish a real release with the current build.
    let admin = Profile {
        root: temp.path().join("admin"),
    };
    write_first_release_content(&admin, ADMIN_TWEAKS);
    admin.set_instance("admin", first_release_records(&admin, &storage));
    admin.write("cemm-pack.json", &spec(false));
    let mut current = Session {
        sidecar: Sidecar::spawn(Path::new(env!("CARGO_BIN_EXE_cemm")), Some(&storage)),
        operation: 0,
    };
    let draft = current.capture(&admin, "r1", None);
    current.publish(&admin, &draft["release"]).expect("publish");
    let (tombstone, document): (Value, Value) = storage.with(|store| {
        (
            serde_json::from_slice(
                store
                    .file_at("main", "test-fusion/r1/cemm-manifest.json")
                    .unwrap(),
            )
            .unwrap(),
            serde_json::from_slice(
                store
                    .file_at("main", "test-fusion/r1/cemm-release.json")
                    .unwrap(),
            )
            .unwrap(),
        )
    });

    // An old profile with something to lose.
    let victim = Profile {
        root: temp.path().join("old-client-profile"),
    };
    victim.write("mods/keep-me.jar", "must survive");
    victim.write("config/keep.toml", "must survive");
    let before = victim.snapshot(&[]);

    let mut old = Sidecar::spawn(&old_binary, None);
    let pong = old
        .call("ping", Value::Null)
        .expect("the old binary speaks the protocol");
    println!("old binary answered ping: {pong}");

    for (label, payload) in [("tombstone", &tombstone), ("release document", &document)] {
        // The preview path: what the old client does first with a manifest.
        let diff = old
            .call("manifest.diff", json!({ "old": null, "new": payload }))
            .expect_err("the old client must not accept this as a manifest");
        println!("old binary, {label}, manifest.diff -> {diff}");

        // And the install path, in case anything ever got that far.
        let install = old
            .call(
                "install.apply_update",
                json!({
                    "operationId": "old-client",
                    "modpackPath": victim.path(),
                    "manifest": payload,
                    "configFiles": [],
                    "options": { "old_manifest": null, "cleanup_old": true }
                }),
            )
            .expect_err("the old client must not install this");
        println!("old binary, {label}, install.apply_update -> {install}");
        assert_eq!(
            victim.snapshot(&[]),
            before,
            "{label}: the old client wrote something"
        );
    }
    assert!(
        old.call("manifest.diff", json!({ "old": null, "new": tombstone }))
            .unwrap_err()
            .contains("This update needs CEMM 2.2.0 or newer"),
        "the old client's own error must carry the upgrade instruction"
    );
}

// ---------------------------------------------------------------------------
// A real pack, in disposable copies
// ---------------------------------------------------------------------------

fn copy_tree(from: &Path, to: &Path, skip: &dyn Fn(&Path) -> bool) -> (u64, u64) {
    let (mut files, mut bytes) = (0, 0);
    let mut pending = vec![(from.to_path_buf(), to.to_path_buf())];
    while let Some((source, target)) = pending.pop() {
        if source.is_file() {
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            bytes += std::fs::copy(&source, &target).unwrap();
            files += 1;
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&source) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if skip(&path)
                || entry
                    .file_type()
                    .map(|kind| kind.is_symlink())
                    .unwrap_or(true)
            {
                continue;
            }
            pending.push((path, target.join(entry.file_name())));
        }
    }
    (files, bytes)
}

/// Rewrites every string that names the source folder so it names the copy.
fn retarget(value: &mut Value, from: &str, to: &str) {
    match value {
        Value::String(text) if text.contains(from) => *text = text.replace(from, to),
        Value::Array(items) => items.iter_mut().for_each(|item| retarget(item, from, to)),
        Value::Object(map) => map.values_mut().for_each(|item| retarget(item, from, to)),
        _ => {}
    }
}

fn jars_in(profile: &Profile) -> std::collections::BTreeSet<String> {
    std::fs::read_dir(profile.root.join("mods"))
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.to_lowercase().ends_with(".jar"))
        .collect()
}

/// A description of the Fusion pack, for a copy of it that has none of its own.
fn built_in_fusion_spec() -> String {
    serde_json::to_string_pretty(&json!({
            "lineage": "atmons-fusion",
            "name": "All the Mons Fusion",
            "base": { "project_id": 1356598, "file_id": 9571494, "name": "All the Mons 2.0.1" },
            "minecraft": "1.21.1",
            "loader": "neoforge-21.1.265",
            "server_release": { "name": "ATMonsFusion-Server-2.0.1-update-1" },
            "pins": [{
                "project_id": BRIDGE,
                "file_id": 8420412,
                "reason": "Do not take 1.2.0 of the Aeronautics/FTB Chunks bridge."
            }],
            "content": [
                { "path": "kubejs" },
                { "path": "config/ftbquests/quests" },
                { "path": "defaultconfigs" },
                { "path": "mods/atmons-ore-hammer-fix-1.0.0.jar" },
                { "path": "config/tacztweaks.json", "policy": "default" }
            ],
            "settings": [{
                "path": "config/tacztweaks.json",
                "pointer": "/general/compatibility/sableCompat",
                "value": false,
                "reason": "TaCZ's own Sable support replaces the Tweaks one."
            }],
            "known_issues": ["Scorched Guns: the per-round reload/unload casing exploit is not fixed. Avoid unloading during a per-round reload."],
            "manual_steps": ["Back up your world before the first launch on this release."]
        }))
    .unwrap()
}

/// The five selected Fusion updates, applied to a copy of a real profile.
///
/// Ignored by default. `CEMM_FUSION_SOURCE` names a real Fusion client, which
/// is only ever read: its mods, scripts, quests and default configs are copied
/// into throwaway folders, one standing in for the admin and one — wound back
/// to before the five updates — for a friend. The release is published to the
/// in-process stand-in for GitHub. The five jars are downloaded from the real
/// CurseForge CDN, because their hashes are the point: each must match the
/// SHA-256 the pack's own maintenance record wrote down independently.
#[test]
#[ignore = "needs CEMM_FUSION_SOURCE pointing at a real Fusion client, and network access to the CurseForge CDN"]
fn a_copy_of_the_real_pack_takes_the_five_selected_updates() {
    let source = PathBuf::from(std::env::var("CEMM_FUSION_SOURCE").expect("CEMM_FUSION_SOURCE"));
    let updates: Vec<Value> = serde_json::from_str(
        &std::fs::read_to_string(source.join("tools/maintenance/2026-11-15/selected-updates.json"))
            .expect("the maintenance record"),
    )
    .unwrap();
    assert_eq!(updates.len(), 5);
    let field = |update: &Value, name: &str| update[name].as_str().unwrap().to_string();

    let storage = Storage::start();
    let temp = tempfile::tempdir().expect("temp dir");
    let started = std::time::Instant::now();

    // ---- The admin's copy: the pack as it stands ---------------------------------------
    let admin = Profile {
        root: temp.path().join("All the Mons Fusion"),
    };
    // The pack's own description if it has one: that is what its admin will
    // publish with, so that is what gets tested. A built-in one stands in for
    // a pack that has not been given a cemm-pack.json yet.
    let spec_text = match std::fs::read_to_string(source.join("cemm-pack.json")) {
        Ok(text) => {
            println!("using the pack's own cemm-pack.json");
            text
        }
        Err(_) => {
            println!("the pack has no cemm-pack.json; using the built-in description");
            built_in_fusion_spec()
        }
    };
    let spec: Value = serde_json::from_str(spec_text.trim_start_matches('\u{feff}'))
        .expect("cemm-pack.json is valid JSON");
    let content_paths: Vec<String> = spec["content"]
        .as_array()
        .expect("content paths")
        .iter()
        .map(|root| root["path"].as_str().expect("a content path").to_string())
        .collect();

    // The addon folders, plus exactly what the description names.
    let no_skip = |_: &Path| false;
    let mut copied = (0u64, 0u64);
    for folder in ["mods", "datapacks", "resourcepacks", "shaderpacks"]
        .into_iter()
        .map(str::to_string)
        .chain(content_paths.iter().cloned())
    {
        if folder.starts_with("mods/") {
            continue; // already inside the mods folder
        }
        let (files, bytes) = copy_tree(&source.join(&folder), &admin.root.join(&folder), &no_skip);
        copied = (copied.0 + files, copied.1 + bytes);
    }
    let mut instance: Value = serde_json::from_str(
        &std::fs::read_to_string(source.join("minecraftinstance.json")).unwrap(),
    )
    .unwrap();
    let source_text = source.to_string_lossy().replace('/', "\\");
    retarget(&mut instance, &source_text, &admin.root.to_string_lossy());
    admin.write(
        "minecraftinstance.json",
        &serde_json::to_string_pretty(&instance).unwrap(),
    );
    println!(
        "copied {} files, {:.1} MB, in {:.0?}",
        copied.0,
        copied.1 as f64 / 1_048_576.0,
        started.elapsed()
    );

    admin.write("cemm-pack.json", &spec_text);

    // ---- The friend's copy: the same pack, wound back to before the updates ------------
    let friend = Profile {
        root: temp.path().join("All the Mons Fusion (friend)"),
    };
    copy_tree(&admin.root, &friend.root, &|path: &Path| {
        path.file_name()
            .is_some_and(|name| name == "cemm-pack.json")
    });
    let mut friend_instance = instance.clone();
    retarget(
        &mut friend_instance,
        &admin.root.to_string_lossy(),
        &friend.root.to_string_lossy(),
    );
    for update in &updates {
        let (old_file, new_file) = (field(update, "old_file"), field(update, "new_file"));
        friend.remove(&format!("mods/{new_file}"));
        std::fs::copy(
            source
                .join("disabled-mods/custom-update-2026-11-15")
                .join(&old_file),
            friend.root.join("mods").join(&old_file),
        )
        .expect("the previous jar, kept by the maintenance run");
        for record in friend_instance["installedAddons"].as_array_mut().unwrap() {
            if record["addonID"] != update["project_id"] {
                continue;
            }
            let folder = record["modFolderPath"].as_str().unwrap().to_string();
            record["fileNameOnDisk"] = json!(old_file);
            record["filePaths"] = json!([format!("{folder}\\{old_file}")]);
            record["installedFile"]["id"] = update["old_file_id"].clone();
            record["installedFile"]["fileName"] = json!(old_file);
            record["installedFile"]["fileNameOnDisk"] = json!(old_file);
        }
    }
    for record in friend_instance["installedAddons"].as_array_mut().unwrap() {
        if record["addonID"] == BRIDGE {
            record["isLocked"] = json!(false);
        }
    }
    friend_instance["name"] = json!("All the Mons Fusion (friend)");
    friend.write(
        "minecraftinstance.json",
        &serde_json::to_string_pretty(&friend_instance).unwrap(),
    );
    // Their own keybinds, and the compatibility flag still on.
    let mut tweaks: Value = serde_json::from_str(&friend.read("config/tacztweaks.json")).unwrap();
    tweaks["general"]["compatibility"]["sableCompat"] = json!(true);
    tweaks["keyActions"] =
        json!({ "friendsOwnBinding": { "key": "key.mouse.5", "note": "must survive" } });
    let friend_key_actions = tweaks["keyActions"].clone();
    friend.write(
        "config/tacztweaks.json",
        &serde_json::to_string_pretty(&tweaks).unwrap(),
    );
    std::fs::copy(source.join("options.txt"), friend.root.join("options.txt")).unwrap();
    friend.write(
        "saves/Friends World/level.dat",
        "a world that is nobody's business",
    );
    friend.write(
        "mods/friends-own-extra.jar",
        "a mod the friend added themselves",
    );
    let personal = friend.snapshot(&["options.txt", "saves/", "mods/friends-own-extra.jar"]);
    let bridge_jar = "mods/create_aeronautics_ftb_chunks-1.21.1-NeoForge-1.1.1.jar";
    let bridge_before = sha256(&std::fs::read(friend.root.join(bridge_jar)).unwrap());

    // No CEMM_DEV_ADDON_ORIGIN: addon downloads go to the real CDN or nowhere.
    let mut session = Session {
        sidecar: Sidecar::spawn_with(
            Path::new(env!("CARGO_BIN_EXE_cemm")),
            &[
                ("CEMM_DEV_GITHUB_API", format!("{}/api", storage.origin)),
                ("CEMM_DEV_GITHUB_RAW", format!("{}/raw", storage.origin)),
            ],
            Duration::from_secs(45 * 60),
        ),
        operation: 0,
    };

    // ---- Capture and publish -------------------------------------------------------------
    let step = std::time::Instant::now();
    let draft = session
        .sidecar
        .call(
            "release.capture",
            json!({ "modpackPath": admin.path(), "releaseId": "acceptance-1", "notes": "Five selected updates.", "previous": null }),
        )
        .expect("capture");
    println!("capture took {:.0?}", step.elapsed());
    println!("summary: {}", draft["summary"]);
    for warning in draft["warnings"].as_array().unwrap() {
        println!("warning: {}", warning.as_str().unwrap());
    }
    for error in draft["errors"].as_array().unwrap() {
        println!("ERROR: {}", error.as_str().unwrap());
    }
    assert_eq!(draft["errors"], json!([]), "the copy must capture cleanly");
    let release = &draft["release"];
    // Every jar CurseForge has no record of is in the release as what it is:
    // a local binary, hosted, hashed. None may ride along unlisted.
    let recorded: std::collections::BTreeSet<String> = instance["installedAddons"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|addon| {
            addon["installedFile"]["fileName"]
                .as_str()
                .map(str::to_string)
        })
        .collect();
    let local_jars: Vec<String> = jars_in(&admin)
        .into_iter()
        .filter(|name| !recorded.contains(name))
        .collect();
    println!("local jars with no CurseForge record: {local_jars:?}");
    assert!(local_jars.contains(&"atmons-ore-hammer-fix-1.0.0.jar".to_string()));
    for name in &local_jars {
        let entry = release["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|file| file["path"] == format!("mods/{name}"))
            .unwrap_or_else(|| panic!("{name} is not part of the release"));
        assert_eq!(entry["source"], "local_binary");
        assert_eq!(entry["hosted"], true);
    }
    assert!(
        !release["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|file| file["path"].as_str().unwrap().ends_with(".snbt_merged")),
        "generated quest fragments must not be captured"
    );
    for update in &updates {
        let addon = release["addons"]
            .as_array()
            .unwrap()
            .iter()
            .find(|addon| addon["project_id"] == update["project_id"])
            .expect("selected update is in the release");
        assert_eq!(addon["file_id"], update["new_file_id"]);
        assert_eq!(addon["file_name"], update["new_file"]);
        // Captured from the admin's disk, and equal to what the maintenance
        // run recorded from the CDN download.
        assert_eq!(addon["sha256"], update["sha256"]);
        assert_eq!(addon["sha1"], update["sha1"]);
        assert_eq!(addon["download_url"], update["download_url"]);
    }
    let bridge = release["addons"]
        .as_array()
        .unwrap()
        .iter()
        .find(|addon| addon["project_id"] == BRIDGE)
        .unwrap();
    assert_eq!(bridge["file_id"], 8420412);
    assert_eq!(bridge["pinned"], true);

    let step = std::time::Instant::now();
    let receipt = session
        .publish(&admin, release)
        .expect("publish to test storage");
    let hosted_bytes: usize = storage.with(|store| store.blobs.values().map(Vec::len).sum());
    println!(
        "publish took {:.0?}: {} objects uploaded, {:.1} MB held by test storage, code {}",
        step.elapsed(),
        receipt["uploaded_objects"],
        hosted_bytes as f64 / 1_048_576.0,
        receipt["code"].as_str().unwrap()
    );
    let published = storage.with(|store| store.head_paths());
    assert!(!published
        .iter()
        .any(|path| path.contains("minecraftinstance") || path.contains("options.txt")));

    // The admin's own copy is, by construction, already the release.
    let fetched = session
        .fetch(receipt["code"].as_str().unwrap())
        .expect("fetch");
    let step = std::time::Instant::now();
    let admin_plan = session.plan(&admin, &fetched);
    println!("planning the admin copy took {:.0?}", step.elapsed());
    assert_eq!(admin_plan["blockers"], json!([]));
    assert_eq!(
        admin_plan["change_count"], 0,
        "a rebuilt candidate must equal its release"
    );

    // ---- The friend adopts and updates in one step ---------------------------------------
    let plan = session.plan(&friend, &fetched);
    println!(
        "friend plan: mode {}, {} downloads ({:.1} MB), {} changes, blockers {}, conflicts {}",
        plan["mode"],
        plan["download_count"],
        plan["download_bytes"].as_f64().unwrap() / 1_048_576.0,
        plan["change_count"],
        plan["blockers"],
        plan["conflicts"]
    );
    assert_eq!(plan["mode"], "adopt");
    assert_eq!(plan["blockers"], json!([]));
    assert_eq!(plan["conflicts"], json!([]));
    assert_eq!(
        plan["download_count"], 5,
        "exactly the five jars; nothing else is fetched"
    );
    let replaced: Vec<&Value> = plan["addons"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|action| action["op"] != "verified")
        .collect();
    assert_eq!(replaced.len(), 5);
    for update in &updates {
        let action = replaced
            .iter()
            .find(|action| action["project_id"] == update["project_id"])
            .expect("each selected update is a planned change");
        assert_eq!(action["op"], "replace");
        assert_eq!(action["from_file"], update["old_file"]);
    }
    assert_eq!(plan["settings"][0]["op"], "apply");
    let bridge_pin = plan["launcher"]["pins"]
        .as_array()
        .unwrap()
        .iter()
        .find(|pin| pin["project_id"] == BRIDGE)
        .expect("the bridge is among the pack's pins");
    assert_eq!(bridge_pin["state"], "will_pin");
    assert_eq!(plan["extras"][0]["path"], "mods/friends-own-extra.jar");

    let step = std::time::Instant::now();
    let report = session.apply(&friend, &fetched, &plan).expect("install");
    println!("install took {:.0?}: {report}", step.elapsed());

    for update in &updates {
        let new_jar = friend.root.join("mods").join(field(update, "new_file"));
        assert_eq!(
            sha256(&std::fs::read(&new_jar).expect("the updated jar is installed")),
            field(update, "sha256"),
            "{} must be byte-identical to the file the maintenance run recorded",
            new_jar.display()
        );
        assert!(!friend.exists(&format!("mods/{}", field(update, "old_file"))));
        let record = friend
            .instance_record(update["project_id"].as_u64().unwrap())
            .unwrap();
        assert_eq!(record["installedFile"]["id"], update["new_file_id"]);
        assert_eq!(record["fileNameOnDisk"], update["new_file"]);
    }
    // The bridge: same bytes, still 1.1.1, and now locked in the launcher record.
    assert_eq!(
        sha256(&std::fs::read(friend.root.join(bridge_jar)).unwrap()),
        bridge_before
    );
    let bridge_record = friend.instance_record(BRIDGE).unwrap();
    assert_eq!(bridge_record["installedFile"]["id"], 8420412);
    assert_eq!(bridge_record["isLocked"], true);
    // The flag is off; the friend's own bindings are untouched.
    let tweaks: Value = serde_json::from_str(&friend.read("config/tacztweaks.json")).unwrap();
    assert_eq!(tweaks["general"]["compatibility"]["sableCompat"], false);
    assert_eq!(tweaks["keyActions"], friend_key_actions);
    assert_eq!(
        friend.snapshot(&["options.txt", "saves/", "mods/friends-own-extra.jar"]),
        personal
    );
    // One active version of everything, and exactly the admin's set of jars.
    let mut friend_jars = jars_in(&friend);
    friend_jars.remove("friends-own-extra.jar");
    assert_eq!(friend_jars, jars_in(&admin));
    assert_eq!(
        std::fs::read(friend.root.join("mods/atmons-ore-hammer-fix-1.0.0.jar")).unwrap(),
        std::fs::read(admin.root.join("mods/atmons-ore-hammer-fix-1.0.0.jar")).unwrap(),
        "the local jar is the admin's, byte for byte"
    );
    let backup = friend.root.join(report["backup_dir"].as_str().unwrap());
    for update in &updates {
        assert!(backup
            .join("files/mods")
            .join(field(update, "old_file"))
            .is_file());
    }
    let instance: Value = serde_json::from_str(&friend.read("minecraftinstance.json")).unwrap();
    assert_eq!(instance["name"], "All the Mons Fusion (friend)");
    assert_eq!(
        instance["allocatedMemory"],
        friend_instance["allocatedMemory"]
    );

    // Same code again: verified, nothing fetched.
    let plan = session.plan(&friend, &fetched);
    assert_eq!(plan["mode"], "reapply");
    assert_eq!(plan["change_count"], 0);
    assert_eq!(plan["download_count"], 0);
    println!(
        "release {} @ {} — {} jars, friend == admin; total {:.0?}",
        receipt["release_id"].as_str().unwrap(),
        receipt["revision"].as_str().unwrap(),
        friend_jars.len(),
        started.elapsed()
    );
}
