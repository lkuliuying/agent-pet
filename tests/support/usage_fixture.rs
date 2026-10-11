use std::{
    io::{self, Write},
    os::windows::process::CommandExt,
    process::Command,
    time::Duration,
};

fn sleep() {
    std::thread::sleep(Duration::from_secs(30));
}

fn read(method: &str) -> String {
    let mut line = String::new();
    io::stdin().read_line(&mut line).unwrap();
    assert!(line.contains(&format!("\"method\":\"{method}\"")));
    line
}

fn send(line: &str) {
    println!("{line}");
    io::stdout().flush().unwrap();
}

fn persistent(mode: &str) {
    let mut count = 0;
    loop {
        let mut line = String::new();
        if io::stdin().read_line(&mut line).unwrap() == 0 {
            return;
        }
        let id: u64 = line
            .split("\"id\":")
            .nth(1)
            .unwrap()
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse()
            .unwrap();
        if line.contains("account/read") {
            if mode == "persistent-auth" && count > 0 {
                send(&format!(r#"{{"id":{id},"result":{{"account":null}}}}"#));
            } else {
                send(&format!(
                    r#"{{"id":{id},"result":{{"account":{{"type":"chatgpt","id":"fixture"}}}}}}"#
                ));
            }
        } else {
            assert!(line.contains("account/rateLimits/read"));
            count += 1;
            if mode == "persistent-change" {
                send(r#"{"method":"account/updated","params":{"authMode":"chatgpt"}}"#);
                send(&format!(
                    r#"{{"id":{id},"result":{{"rateLimits":{{"primary":{{"usedPercent":99}}}}}}}}"#
                ));
                sleep();
                return;
            }
            send(
                r#"{"method":"account/rateLimits/updated","params":{"rateLimits":{"limitId":"extra","primary":{"usedPercent":40}}}}"#,
            );
            send(
                r#"{"method":"account/rateLimits/updated","params":{"rateLimits":{"limitId":"codex","primary":{"usedPercent":30}}}}"#,
            );
            send(&format!(
                r#"{{"id":{id},"result":{{"rateLimitsByLimitId":{{"codex":{{"primary":{{"usedPercent":10}},"secondary":{{"usedPercent":20}}}}}}}}}}"#
            ));
            if mode == "persistent-long" {
                let padding = "x".repeat(180_000);
                for _ in 0..30 {
                    send(&format!(
                        r#"{{"method":"account/rateLimits/updated","params":{{"rateLimits":{{"limitId":"extra","primary":{{"usedPercent":35}}}},"ignored":"{padding}"}}}}"#
                    ));
                }
            }
        }
    }
}

fn main() {
    if std::env::args().any(|arg| arg == "--sleep") {
        sleep();
        return;
    }
    let pid_file = std::env::var_os("FIREFLY_FIXTURE_PID_FILE").unwrap();
    std::fs::write(pid_file, std::process::id().to_string()).unwrap();
    let mode = std::env::var("FIREFLY_FIXTURE_MODE").unwrap();
    let init = read("initialize");
    assert!(init.contains("\"id\":1"));
    match mode.as_str() {
        "timeout" => {
            sleep();
            return;
        }
        "exit" => return,
        "invalid" => {
            send("{broken");
            sleep();
            return;
        }
        "oversize" => {
            print!("{}", "x".repeat(1_048_577));
            io::stdout().flush().unwrap();
            sleep();
            return;
        }
        _ => {}
    }
    send(r#"{"id":1,"result":{}}"#);
    read("initialized");
    if mode.starts_with("persistent") {
        persistent(&mode);
        return;
    }
    let account = read("account/read");
    assert!(account.contains("\"refreshToken\":false"));
    match mode.as_str() {
        "signed-out" => {
            send(r#"{"id":2,"result":{"account":null}}"#);
            sleep();
            return;
        }
        "api-key" => {
            send(r#"{"id":2,"result":{"account":{"type":"apiKey"}}}"#);
            sleep();
            return;
        }
        _ => {}
    }
    send(r#"{"id":2,"result":{"account":{"type":"chatgpt"}}}"#);
    read("account/rateLimits/read");
    if mode == "network" {
        send(
            r#"{"id":3,"error":{"code":-32000,"message":"error sending request: fixture-private-value"}}"#,
        );
        sleep();
        return;
    }
    if mode == "descendant" {
        let child = Command::new(std::env::current_exe().unwrap())
            .arg("--sleep")
            .creation_flags(0x08000000)
            .spawn()
            .unwrap();
        std::fs::write(
            std::env::var_os("FIREFLY_FIXTURE_CHILD_PID_FILE").unwrap(),
            child.id().to_string(),
        )
        .unwrap();
        // 夹具故意不等待后代，由被测查询适配层的作业负责回收。
        drop(child);
    }
    eprintln!("fixture-private-value must not reach the UI");
    send(r#"{"method":"account/rateLimits/updated","params":{"ignored":true}}"#);
    send(
        r#"{"id":3,"result":{"rateLimits":{"primary":{"usedPercent":25,"windowDurationMins":300,"resetsAt":1900000000},"secondary":{"usedPercent":42,"windowDurationMins":10080}}}}"#,
    );
    sleep();
}
