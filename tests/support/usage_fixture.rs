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
