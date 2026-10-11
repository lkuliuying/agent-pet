#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(windows)]
mod app;

fn main() {
    #[cfg(windows)]
    {
        let args: Vec<String> = std::env::args().skip(1).collect();
        if args.iter().any(|a| a == "--codex-hook") {
            if let Ok(event) = pet_desktop::bridge::parse_hook(std::io::stdin().lock()) {
                let _ = pet_desktop::platform::ipc::send(&event);
            }
            // hooks 始终中立退出，桌宠故障不能改变 Codex 的审批或执行决定。
            println!("{{}}");
            return;
        }
        if args.iter().any(|a| a == "--codex-test") {
            let event = pet_desktop::bridge::CodexEvent {
                version: 1,
                kind: pet_desktop::bridge::CodexKind::Stop,
                session_id: "simulation".into(),
                turn_id: format!("test-{}", std::process::id()),
                simulated: true,
                ..Default::default()
            };
            match pet_desktop::platform::ipc::send(&event) {
                Ok(()) => println!("模拟事件已发送，不代表真实 Codex 接入。"),
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
            return;
        }
        if let Err(e) = app::run(&args) {
            pet_desktop::platform::desktop::alert(&format!("桌宠遇到问题：{e}"));
            std::process::exit(1);
        }
    }
    #[cfg(not(windows))]
    eprintln!("流萤桌宠首版仅支持 Windows。 ");
}
