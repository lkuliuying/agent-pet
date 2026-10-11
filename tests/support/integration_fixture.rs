use std::{fs, path::Path, process};

fn quoted(path: &Path) -> String {
    format!(
        "\"{}\"",
        path.to_string_lossy()
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
    )
}

fn main() {
    let exe = std::env::current_exe().unwrap();
    let root = exe.parent().unwrap();
    let args: Vec<_> = std::env::args().skip(1).collect();
    let state = root.join("market.txt");
    let installed = root.join("installed");
    assert_eq!(args.first().map(String::as_str), Some("plugin"));
    match (args[1].as_str(), args.get(2).map(String::as_str)) {
        ("marketplace", Some("list")) => {
            let local = fs::read_to_string(&state)
                .ok()
                .map(|s| {
                    format!(
                        ",{{\"name\":\"firefly-local\",\"root\":{}}}",
                        quoted(Path::new(&s))
                    )
                })
                .unwrap_or_default();
            println!("{{\"marketplaces\":[{{\"name\":\"other\",\"root\":\"unrelated\"}}{local}]}}");
        }
        ("marketplace", Some("remove")) => {
            assert_eq!(args[3], "firefly-local");
            fs::remove_file(state).unwrap();
            println!("{{}}");
        }
        ("marketplace", Some("add")) => {
            fs::write(state, &args[3]).unwrap();
            println!("{{}}");
        }
        ("list", _) => {
            assert_eq!(args[2], "--marketplace");
            assert_eq!(args[3], "firefly-local");
            println!(
                "{{\"installed\":[{}]}}",
                if installed.exists() {
                    "{\"pluginId\":\"firefly-pet@firefly-local\",\"enabled\":true}"
                } else {
                    ""
                }
            );
        }
        ("add", Some("firefly-pet@firefly-local")) => {
            let failure = root.join("fail-once");
            if failure.exists() {
                fs::remove_file(failure).unwrap();
                process::exit(2);
            }
            fs::write(installed, b"yes").unwrap();
            let path = fs::read_to_string(state).unwrap();
            println!(
                "{{\"installedPath\":{}}}",
                quoted(&Path::new(&path).join("plugins/firefly-pet"))
            );
        }
        _ => process::exit(3),
    }
}
