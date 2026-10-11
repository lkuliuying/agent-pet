# 流萤键盘桌宠 v0.1 · 2D 轻侧视版

Windows 10/11 x64 原生免安装桌宠。2D Q 版流萤以轻侧视姿态坐在三排无字小键盘后方，采用灰银长发、青绿色发尾、蓝紫眼睛和白色制服造型。键盘面对角色，空格键靠近她，背沿与接口朝向用户。输入时双手按左右区域敲击，键盘右上方的浅色小气泡短暂显示最近按键，例如 `A`、`Space`、`Ctrl + C`。内置素材和界面编译进 `PetDesktop.exe`，运行不需要 Rust、Node、Python 或 WebView。

## 使用

解压整个交付包，双击 `PetDesktop.exe`。首次显示在屏幕右下角，默认置顶、90% 大小、语音关闭。角色舞台仍为 300×320 逻辑像素；增加顶部 30 像素留白后，窗口为 300×350，默认约 270×315。头顶保留提醒气泡，整机 CPU、内存和 Codex 额度仅在点击角色后显示。

- 左键点角色：开心，并展开“当前状态”面板，显示整机 CPU/内存及 Codex 剩余额度、重置时间。再次点角色、点击面板与角色以外的位置、按 Esc 或点击“收起”关闭面板；没有自动隐藏计时。面板宽 320 逻辑像素、最高 420 像素，内容较多时滚动，字体跟随系统 DPI，不随角色缩小。
- 按住拖动：移动桌宠并收起状态面板；右键：收起状态面板并打开独立快捷菜单，可暂停/恢复同步、开关勿扰、开关鼠标穿透、重置位置、进入完整设置或退出。点击外部、按 Esc 或选择操作会关闭菜单；只有“完整设置”打开设置窗口。打开设置或开启穿透也会收起状态面板。
- 系统托盘的青绿色图标：左键显示桌宠并关闭穿透；右键提供同样的快捷操作和完整设置入口。图标可能位于 Windows 托盘折叠区。开启穿透后从托盘恢复交互。
- 设置窗口默认约 860×600，左侧五个模块为“桌面与键盘、消息提醒、语音播报、Codex 联动、关于”，右侧只显示当前模块。小窗口仅内容区滚动，底部保存栏固定。电量状态在消息提醒模块，语音错误及联动状态分别在对应模块，关于页保留 Slint 信息。
- 所有模块共用一份草稿，切换模块或重复打开仍显示的设置窗口不会覆盖修改。“保存并应用”统一保存全部模块；关闭设置窗口后重新打开会读取当前生效设置。关于页无待保存修改时隐藏保存按钮。
- 快捷操作与重置位置立即保存并生效，只回填对应字段，保留其他草稿；保存失败维持原生效设置并报错。大小范围仍为 60%–160%，系统 DPI 另由 Windows 决定。重置位置回到角色所在屏幕右下角。小键盘用于装饰，不逐键映射，也不切换完整键盘布局。
- 标签表示最近一次按下的物理键及当时的修饰键，顺序为 Ctrl、Alt、Shift、Win，左右修饰键合并显示。多个普通键只显示最新一枚；数字键盘显示 `Num 1`、`Num Enter` 等。按住时保持，全部松开后停留 800 ms，再用 120 ms 淡出；新按键立即替换，不积累输入历史，不转换为输入法文字。
- 暂停同步立即清空按键。勿扰关闭当前文字和语音，并丢弃积压提醒；离开勿扰后不补播过期消息。两者均不停止整机 CPU / 内存采样，手动展开状态面板仍可查看读数。
- 语音使用 Windows 已安装的系统音色，选择音色、调整音量后可“试听”。没有音色或播放失败会在设置页提示，文字提醒继续工作。
- 退出请使用角色或托盘菜单。关闭设置窗口会继续保留桌宠。第二次启动仅提示已有实例，不开启第二套监听。

程序只维护当前按键和短暂动画状态，不记录输入顺序、文本、剪贴板或按键历史，不上传按键或对话内容；按需查询额度时由本机 Codex 访问账号服务。不注册开机启动，不需要管理员权限。跨完整性级别应用、硬件 `Fn`、安全桌面或未上报 Raw Input 的驱动行为不作同步保证。

## Codex 本地插件

包内 `codex-marketplace` 是独立的本地插件市场，包含同一版本的桥接 exe。源代码目录中的市场模板在执行打包脚本前不包含 exe。

在 PowerShell 中切到**已解压交付包目录**，运行：

```powershell
codex plugin marketplace add .\codex-marketplace
codex plugin add firefly-pet@firefly-local
```

然后重新打开 Codex 桌面应用，在插件与 hooks 管理界面审阅并信任这三个命令。当前官方说明也支持在插件目录中选择本地市场并安装；若本机 CLI 没有 `plugin add`，使用桌面应用安装。安装插件不等于已信任 hooks。不要手动跳过信任检查，也不需要编辑已有 `.codex/config.toml`。

必须是本地编排的 Codex 会话。云端编排即使使用本地工具也不支持此类命令 hooks。首次接入分别在真实会话中触发轮次结束、审批请求、中断，观察桌宠设置页的接收状态和气泡。审批可由正常审批机制自动处理，不要为了测试执行破坏性命令。

| 事件 | 固定播报 | 语义 |
| --- | --- | --- |
| `Stop` | Codex 这一轮回复结束了。 | 不代表整个任务成功 |
| `PermissionRequest` | Codex 发起了审批请求。 | 不推断一定需要人工审批 |
| `Interrupt` | 这一轮已中断。 | 不推断中断原因 |

设置页的演示按钮仅测试表情与播报。以下命令测试完整的本地管道，显示“联动演示”，也不代表真实 Codex 接入：

```powershell
.\PetDesktop.exe --codex-test
```

桥接命令为 `PetDesktop.exe --codex-hook`，从标准输入读取 hook JSON，最多 1 MiB。只反序列化事件类型、会话及轮次标识，忽略提示词、工具参数、回复和 transcript 路径；向管道发送的 JSON 最多 2048 字节，不读取 Codex 数据库或历史文件。传输内容只在内存保留，去重标识最多 512 条、5 分钟。

插件使用 `async: true` 和 2 秒超时。桥接连接等候最多 150 ms，写入等候最多 200 ms；桌宠不运行、输入无效或发送失败时仍返回 `{}` 和退出码 0，绝不返回 allow/deny 或继续任务的决定。Windows 批处理只定位包内 exe，不拼接 hook 输入为命令。非 Windows 插件命令为中立空操作。

管道为 `\\.\pipe\FireflyPet-v1-<当前用户 SID>`，DACL 只授权当前用户，拒绝远程客户端；不开放 TCP 端口。同用户本地进程仍可发送状态，因此设置页“已收到 hooks 状态事件”仅表示收到消息，不能鉴别是否由真正的 Codex 进程发送。模拟与 hooks 消息在界面中分别标记。

更新版本后重新安装插件并按 Codex 要求复核 hooks。保留本地市场所在目录，以便 Codex 后续刷新；不向公开插件市场发布。当前机器的真实 Codex 接入验收状态见 `docs/VALIDATION.md`。

### Codex 账号额度

额度查询独立于 hooks 和“接收 Codex 事件”开关，不需要安装提醒插件。需在本机安装 Codex CLI、将 `codex.exe` 所在目录加入 PATH，并在所用 Codex 登录环境中登录 ChatGPT。从资源管理器启动的桌宠使用该进程继承的 PATH；安装或更新 PATH 后需重新启动桌宠。API Key 等不提供 ChatGPT 订阅额度的环境会显示说明，不伪造剩余百分比。

展开状态面板时立即查询，持续显示时每 60 秒刷新，也可点击面板“刷新”或设置 → Codex 联动 →“刷新额度”。收起面板停止周期刷新并取消在途查询；设置页手动刷新只执行一次，结果显示在该页并与面板共享。快速重复刷新合并为一次，每次查询最多等待 10 秒，查询期间不阻塞角色动画、键盘同步或系统采样。

桌宠从 PATH 的绝对目录查找 `codex.exe`，创建隐藏的短生命周期 `codex app-server --listen stdio://` 子进程，依次初始化、调用 `account/read`（`refreshToken: false`）与 `account/rateLimits/read`。工作目录设为该 CLI 所在目录，避免加载桌宠项目的 Codex 配置；登录仍由 Codex 自身处理，额度属于该 CLI 的账号与工作区，独立配置环境可能与桌面应用不同。桌宠不读取凭据、历史或数据库，不创建会话、调用模型、修改 Codex 配置或使用额度重置。

优先显示服务端的 `rateLimitsByLimitId`，兼容旧 `rateLimits`，按返回周期展示各类额度；剩余为 `100 − usedPercent` 并限制在 0–100，未知数据仍为“未知”。重置和更新时间显示本地时间，不根据到达重置时间自行推断满额。加载与失败状态清除旧额度读数，未安装、未登录、不支持、超时、网络及响应错误均有独立提示。结果仅保存在内存，不输出原始账号或错误响应。

查询使用单个后台工作线程、有界唤醒/结果通道和取消代次；stdout 按行最多 1 MiB、每路输出最多 4 MiB，stderr 只排空而不展示。使用 Windows 作业回收自己启动的查询进程及其后代，不按进程名结束用户的 Codex。查询不开放本地监听端口。CPU/内存仍每 5 秒采样，原有高占用告警不受面板显示状态影响。

## 系统提醒规则

- 每 5 秒读取 `GetSystemTimes`、`GlobalMemoryStatusEx`、`GetSystemPowerStatus` 和 Windows 网络连接状态。第一轮 CPU 或不可用的数据显示未知；无电池不伪造百分比。
- CPU / 内存严格大于 90% 连续 30 秒，每类告警间隔至少 5 分钟。低于阈值、关闭开关或采样未知会重置连续时间。
- 使用电池时在 ≤20% 和 ≤10% 各提醒一次；接入外部电源或回到 >20% 重置。电源提醒反映外部供电连接，不声称电池一定正在充电。
- 互联网连接状态变化连续两个采样一致才播报，不把局域网连接等同于已连接互联网。所有提醒独立开关，关闭某项会移除其待播消息；同类提醒去重 5 分钟。
- 最多排队 12 条，审批和极低电量优先，消息排队超过 45 秒即丢弃；每条气泡显示 7 秒。默认全屏、演示或系统报告用户不在场时抑制主动提醒。输入同步与提醒相互独立。

## 结构与配置

本项目以本 README 记录可复用的架构、构建、接口与限制；没有额外项目记忆文件。

源码仓库为 [lkuliuying/agent-pet](https://github.com/lkuliuying/agent-pet)，主分支为 `main`。仓库保留源码、内置角色资源、锁定依赖清单、测试、插件模板、构建脚本与文档；`target/`、`artifacts/`、`.runtime/` 和插件目录中的 exe 不纳入版本控制。旧桌面截图 `docs/native-preview.jpg` 含有后方窗口内容，仅在本地保留；其他文档中的模拟预览不代表真实账号或设备验收。

| 文件或模块 | 职责 |
| --- | --- |
| `src/app.rs`、`ui/app.slint` | UI 状态绑定、生命周期、交互和托盘；不直接采集系统或键盘数据 |
| `src/app/usage_panel.rs`、`ui/status-panel.slint` | 点击信息面板、共享额度展示、按需刷新及关闭取消 |
| `src/usage.rs`、`src/platform/codex_usage.rs` | 额度解析与请求状态、Codex stdio 查询、超时与自建进程回收 |
| `ui/settings.slint`、`ui/quick-menu.slint`、`ui/controls.slint` | 分模块设置、独立快捷菜单和共享界面控件 |
| `src/ui_state.rs` | 快捷操作候选配置与草稿合并、先保存后生效、资源读数和旧位置坐标换算 |
| `src/keyboard.rs` | 物理键元数据、按设备维护按下集合、最近按键标签、重复按下合并和左右手时限；布局数据只用于识别与分类 |
| `src/notices.rs` | 告警状态机、有界队列、去重与勿扰规则 |
| `src/bridge.rs` | hooks 最小字段解析、协议验证和固定播报文案 |
| `src/config.rs` | 版本化配置验证、原子保存和损坏文件备份 |
| `src/assets.rs`、`assets/firefly/character.json` | 角色清单 v1、图集裁剪、分层锚点与表情顺序 |
| `src/platform/` | Raw Input、窗口样式、当前用户管道、系统采样和 SAPI 语音的 Win32 封装 |

UI 线程每 16 ms 合并标签、淡出与手部状态；原始输入线程每 8 ms 泵消息，重复按下不排队。标签在输入事件到达时保存最近一次按下的快照，短于一帧的快速点击仍可显示；到期清除，不保存历史。Raw Input 使用 `RIDEV_INPUTSINK | RIDEV_DEVNOTIFY`，扫描码的 E0 位区分左右修饰键、主区和数字区 Enter；Pause / PrintScreen / NumLock 单独归一化。设备断开清除该设备及相关标签，锁屏及电源广播清空全部状态，每 500 ms 只核对仍按下键的真实 VK 防止丢失松键。手部动作 110 ms，空闲 90 秒困倦。

采样、桥接、额度查询和语音各有专用线程及有界消息通道。退出时设置共享停止标识、取消未完成管道 I/O 与额度查询、清理窗口和 Raw Input 注册、停止语音并 join 工作线程。没有运行时线程无限重试或按键动画队列。

配置位于 `%LOCALAPPDATA%\FireflyPet\settings.json`。`version` 必须为 1，大小上限 64 KiB。写入临时文件并 `sync_all`，随后 `MoveFileExW(REPLACE_EXISTING | WRITE_THROUGH)` 原子替换。损坏、未知版本或未知字段配置会保留到同目录 `settings.invalid-<时间戳>.json`，设置页提示恢复；备份失败时不覆盖损坏原文件。有效设置保留位置、大小、键盘布局、各项开关和系统音色 ID，不含按键或对话内容。

旧配置中的 `tkl` 字段仍按原值读写以保持兼容，但不再影响界面；其他设置沿用原值，不重置用户配置。`position` 仍表示旧 300×320 舞台的物理像素锚点：显示时减去 `30 × 大小比例 × 屏幕 DPI 比例` 的顶部偏移，保存时加回，避免重复升级/重启累积偏移。接近工作区边界时仍会进行可见范围约束。不可用的已保存音色 ID 不因修改其他选项而被清空。

角色图集为 2D 轻侧视透明 PNG，裁剪和定位由 manifest v1 描述，包括身体、左右前臂与手、装饰键盘，以及待机、眨眼、专注、开心、困倦、惊讶六张表情（提醒复用惊讶）。六张表情共享目标位置与比例，袖口重叠连接独立前臂。手部偏移已调整到键盘中部键帽，图集、身体、表情和键盘保持原样。呼吸与双手位移在 UI 运行时合成，键盘不随呼吸移动；加载时核对 PNG 实际尺寸、裁剪范围和动画最外边界。按键气泡靠键盘右上方，长标签向左扩展，超长内容省略；头顶提醒与角色之间保留间距，整机指标移入点击面板，没有常驻心情或资源文字条。首版仅内置流萤，资源嵌入 exe；尚未提供运行时导入其他角色的 UI。素材来源、生成提示及裁剪校准见 `assets/firefly/README.md`，已确认的角色造型基准见 `docs/firefly-2d-approved.png`。

## 构建和验证

开发环境：Windows x64、Rust 1.92+ 的 MSVC 工具链、Visual Studio C++ Build Tools 和 Windows 10/11 SDK。Slint 与 slint-build 固定为 1.18.1，Winit + FemtoVG-WGPU；使用静态 CRT。GPU 驱动需支持所用 WGPU 后端，透明渲染须在目标电脑验收。

以下构建和验证命令在源码工程目录运行，便携包用户只需双击 exe。在已配置的 x64 Native Tools 环境中：

```powershell
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo fmt --all -- --check
cargo build --release --locked
```

普通 PowerShell 可使用脚本临时配置当前进程的编译环境，`--` 分隔符请以数组传递，避免 PowerShell 参数绑定吞掉它：

```powershell
.\scripts\native-cargo.ps1 test --locked
.\scripts\native-cargo.ps1 -CargoArguments @('clippy','--all-targets','--locked','--','-D','warnings')
.\scripts\native-cargo.ps1 -CargoArguments @('fmt','--all','--','--check')
.\scripts\native-cargo.ps1 build --release --locked
.\scripts\smoke-desktop.ps1
.\scripts\package-windows.ps1 -SkipBuild
```

首次构建需下载锁定依赖；缓存完整时可添加 `--offline`。Windows 命名管道和桌面测试必须在正常用户会话中进行，受限沙箱可能无法发现同一测试创建的管道。打包脚本生成独立时间戳目录、zip、SHA-256 清单以及依赖许可文件，不安装、不提交 Git、不发布。

开发测试可用 `--data-dir=<测试目录>` 隔离配置，`--exit-after=<秒>` 在 1–3600 秒后自动退出；默认正常运行没有自动退出。这两个参数不会改变当前用户单实例约束。

素材预览示例使用与桌宠相同的 Slint 界面和资源，只显示模拟数据，不启动输入监听、管道、托盘、额度查询或配置写入；点击角色可展开固定模拟数据的状态面板。参数依次为标签、表情索引 0–5、缩放 0.6–1.6；追加 `--animate` 可循环演示六种表情、单手/双手动作、长标签和提醒。可选 `--background=light` 或 `--background=dark` 仅给开发预览增加验收底色，正式程序始终使用默认透明背景。180 秒自动退出；示例不包含在免安装包内：

```powershell
.\scripts\native-cargo.ps1 -CargoArguments @('run','--release','--example','visual_check','--locked','--offline','--','Ctrl + C','0','0.9')
.\scripts\native-cargo.ps1 -CargoArguments @('run','--release','--example','visual_check','--locked','--offline','--','Space','0','1.6','--animate','--background=dark')
```

本机已执行检查、观察结果和待验收项统一记录于 `docs/VALIDATION.md`。不将静态素材预览等同于实际桌宠行为，不将自动化输入等同于物理键盘，不将渲染缩放模拟等同于修改系统 DPI，不将管道测试等同于真实 Codex hooks。

`ui_check` 是另一项原生开发示例，输出 60%/90%/160% × 六表情 × 待机/双手敲击、五个模块在两种窗口尺寸下的布局、快捷菜单及额度面板的正常/加载/失败/多额度/未知读数截图，并检查模块草稿、关于页保存按钮、右键分流、左键开关、拖动分流与菜单 Esc 回调。截图只来自示例渲染缓冲区，不采集桌面；输出目录必须尚不存在，不启动输入监听/管道/托盘/额度查询或读写用户配置，不随便携包交付：

```powershell
.\scripts\native-cargo.ps1 -CargoArguments @('build','--release','--example','ui_check','--locked','--offline')
.\target\release\examples\ui_check.exe .runtime\ui-review-new
```

额度进程集成测试在 `target/usage-fixtures` 编译只依赖 Rust 标准库的合成服务端，检查协议顺序、失败、取消、超时及进程树回收。默认测试不查询真实账号。需要验证本机实际登录环境时单独运行以下只读测试，仅输出额度窗口数量，不输出账号或原始响应：

```powershell
.\scripts\native-cargo.ps1 -CargoArguments @('test','--test','usage_process','real_codex_quota_read_only','--locked','--offline','--','--ignored','--nocapture')
```

## 参考与许可

- [Slint Winit 与渲染器](https://docs.slint.dev/latest/docs/slint/guide/backends-and-renderers/backend_winit/)
- [Windows Raw Input](https://learn.microsoft.com/en-us/windows/win32/inputdev/about-raw-input)
- [Windows 异步命名管道](https://learn.microsoft.com/en-us/windows/win32/api/namedpipeapi/nf-namedpipeapi-connectnamedpipe)
- [OpenAI 本地插件与市场](https://developers.openai.com/plugins/build/plugins)
- [Codex hooks 事件、超时与信任](https://learn.chatgpt.com/docs/hooks)
- [Codex app-server 账号与额度接口](https://learn.chatgpt.com/docs/app-server)

角色为 AI 生成的流萤同人素材，非官方游戏资源；首版按个人本地使用交付。没有角色声音克隆、AI 对话或模型服务调用。包内保留第三方许可说明，设置页保留 Slint 的 AboutSlint 信息。源码仓库未指定项目整体开源许可；便携程序未做代码签名，构建产物不随源码提交。
