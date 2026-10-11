# 流萤键盘桌宠 v0.1 · 2D 轻侧视版

Windows 10/11 x64 原生免安装桌宠。2D Q 版流萤以轻侧视姿态坐在三排无字小键盘后方，采用灰银长发、青绿色发尾、蓝紫眼睛和白色制服造型。键盘面对角色，空格键靠近她，背沿与接口朝向用户。输入时双手按左右区域敲击，键盘右上方的浅色小气泡短暂显示最近按键，例如 `A`、`Space`、`Ctrl + C`。内置素材和界面编译进 `PetDesktop.exe`，运行不需要 Rust、Node、Python 或 WebView。

## 使用

解压整个交付包，双击 `PetDesktop.exe`。首次显示在屏幕右下角，默认置顶、90% 大小、语音关闭。角色舞台仍为 300×320 逻辑像素；增加顶部 30 像素留白后，窗口为 300×350，默认约 270×315。头顶保留提醒气泡，CPU、内存、GPU 和 Codex 额度仅在点击角色后显示。

- 左键点角色：默认显示开心表情，并展开 CPU、内存、GPU、Codex 四个同排小框，整行 352×32 逻辑像素。点击 Codex 小框展开或收起额度详情，可查看各周期、重置时间、刷新和当前会话；整块展开最高 440 像素，详情较多时滚动。再次点角色、点击面板与角色以外的位置或按 Esc 关闭面板；没有自动隐藏计时。字体跟随系统 DPI，不随角色缩小。
- 按住拖动：移动桌宠并收起状态面板；右键：收起状态面板并打开独立快捷菜单，可暂停/恢复同步、开关勿扰、开关鼠标穿透、重置位置、进入完整设置或退出。点击外部、按 Esc 或选择操作会关闭菜单；只有“完整设置”打开设置窗口。打开设置或开启穿透也会收起状态面板。
- 系统托盘的青绿色图标：左键显示桌宠并关闭穿透；右键提供同样的快捷操作和完整设置入口。图标可能位于 Windows 托盘折叠区。开启穿透后从托盘恢复交互。
- 设置窗口默认约 860×600，左侧六个模块为“桌面与键盘、角色表情、消息提醒、语音播报、Codex 联动、关于”，右侧只显示当前模块。小窗口仅内容区滚动，底部保存栏固定。电量状态在消息提醒模块，语音错误及联动状态分别在对应模块，关于页保留 Slint 信息。
- 所有模块共用一份草稿，切换模块或重复打开仍显示的设置窗口不会覆盖修改。“保存并应用”统一保存全部模块；关闭设置窗口后重新打开会读取当前生效设置。关于页无待保存修改时隐藏保存按钮。
- 快捷操作与重置位置立即保存并生效，只回填对应字段，保留其他草稿；保存失败维持原生效设置并报错。大小范围仍为 60%–160%，系统 DPI 另由 Windows 决定。重置位置回到角色所在屏幕右下角。小键盘用于装饰，不逐键映射，也不切换完整键盘布局。
- 标签表示最近一次按下的物理键及当时的修饰键，顺序为 Ctrl、Alt、Shift、Win，左右修饰键合并显示。多个普通键只显示最新一枚；数字键盘显示 `Num 1`、`Num Enter` 等。按住时保持，全部松开后停留 800 ms，再用 120 ms 淡出；新按键立即替换，不积累输入历史，不转换为输入法文字。
- 暂停同步立即清空按键。勿扰关闭当前文字和语音，并丢弃积压提醒；离开勿扰后不补播过期消息。两者均不停止 CPU / 内存 / GPU 采样，手动展开状态面板仍可查看读数。
- 鼠标在桌面任意位置移动时，角色、双手和键盘一起朝左右转向，瞳孔独立跟随上下左右位置。中心线左右各 16 逻辑像素内保持原朝向，鼠标在另一侧持续 100 ms 后切换；拖动时锁定朝向，松手后恢复。文字始终正向，按键气泡随键盘换侧，左侧键区仍驱动屏幕左手。
- 鼠标跟随默认常开，没有独立开关；暂停键盘同步、勿扰和穿透均不影响跟随。眨眼和开心闭眼时不显示活动瞳孔，鼠标活动不重置键盘空闲计时。光标读取暂时失败时保持朝向、瞳孔平滑回中，恢复读取后继续；不保存鼠标轨迹。
- 语音固定使用内置流萤 0.3 音色，启用后离线播放预设提醒，不依赖系统音色或模型服务。音量默认 70%，试听使用未保存的草稿音量；开关和音量仍统一保存。无输出设备或播放失败会提示，连接设备后可重新试听，文字提醒继续工作。
- 退出请使用角色或托盘菜单。关闭设置窗口会继续保留桌宠。第二次启动仅提示已有实例，不开启第二套监听。

程序只维护当前按键和短暂动画状态，不记录输入顺序、文本、剪贴板或按键历史，不上传按键或对话内容；后台查询额度时由本机 Codex 访问账号服务。不注册开机启动，不需要管理员权限。跨完整性级别应用、硬件 `Fn`、安全桌面或未上报 Raw Input 的驱动行为不作同步保证。

## 角色表情设置

从角色右键或系统托盘进入“完整设置 → 角色表情”。可为七个场景选择内置的待机、眨眼、专注、开心、困倦、惊讶；基础待机必须选择表情，其余场景可选“不切换”，让位给其他有效场景。关闭点击或拖动表情不会关闭信息面板或窗口移动，关闭提醒表情也不会关闭消息气泡。

预览框使用与桌宠相同的角色图层，瞳孔保持居中；选择预览不会改变桌面角色或配置草稿，修改场景下拉框则更新草稿并预览所选表情。“保存并应用”成功后生效，失败时原配置仍有效。恢复默认只重置表情草稿，大小、语音音量、提醒开关等其他草稿保持原样；关闭设置后重开会丢弃未保存修改。改变表情配置并保存后会清理之前的点击/拖动瞬态，按当前输入和提醒状态重新计算，避免重放旧交互。

初始配置如下，所有场景默认启用：

| 场景 | 初始表情 | 初始时间 | 可调整范围 |
| --- | --- | --- | --- |
| 基础待机 | 待机 `idle` | 其他场景结束后显示 | 必选一张表情 |
| 待机眨眼 | 眨眼 `blink` | 每 4.8 秒的前 150 毫秒 | 间隔 1–30 秒；持续 50–1000 毫秒，且小于间隔 |
| 键盘输入 | 专注 `focused` | 最近一次有效按下后 1.2 秒 | 持续 0.1–10 秒 |
| 点击角色 | 开心 `happy` | 2 秒 | 持续 0.1–10 秒 |
| 无键盘输入 | 困倦 `sleepy` | 空闲超过 90 秒 | 等待 15–3600 秒 |
| 拖动角色 | 惊讶 `surprised` | 每次有效拖动刷新 2 秒 | 持续 0.1–10 秒 |
| 消息提醒 | 惊讶 `surprised` | 跟随气泡，目前 7 秒 | 系统与 Codex 消息共用一项，不单独修改气泡时长 |

优先级为“拖动 → 点击 → 提醒 → 输入或困倦 → 待机眨眼 → 基础待机”，由场景决定，不因多个场景选择同一张图片而改变。眨眼周期沿用程序启动时刻，只在没有更高优先级场景时显示；提醒消失或被勿扰清除后让位给其他场景。启动后没有键盘输入时，以启动时刻计算空闲，默认最初约 1.2 秒也显示专注。按住同一个键不会持续重置空闲计时；暂停键盘同步不暂停这个计时。鼠标跟随和闭眼时隐藏动态瞳孔的原有规则保持不变。

自定义配置保存在原 `settings.json` 的可选 `expressions` 对象中，表情存储上述稳定名称，禁用场景存储 `null`，时间字段使用整数毫秒。旧配置缺少该对象或其中字段时使用默认值，其他用户偏好原样保留。配置全为默认时省略该对象。旧程序会拒绝不认识的字段，因此**回退旧版前，应先在新版恢复默认表情并保存**；不要直接让旧版读取自定义表情配置。

## Codex 本地联动

从“设置 → Codex 联动 → 安装／更新联动”接入。按钮将内嵌资源及当前桥接 exe 部署到桌宠数据目录的 `codex-bridge/bridge-<版本时间>/`，再通过 Codex CLI 登记 `firefly-local`、安装 `firefly-pet@firefly-local`。只切换该市场，保留其他插件和用户已有 `hooks.json`；核对 `installedPath` 下的 exe、manifest、hooks 与当前资源逐字节一致。安装失败显示具体阶段，并尝试恢复原市场登记；旧版本目录保留，不自动清理。

启动和“重新检测”只检查，不自动安装。检测区分未安装、未启用、需要信任、等待事件和版本不兼容；真实事件投递状态单独显示。CLI 操作最多等待 30 秒，失败文案不转发原始输出。

**必须由用户在 Codex 中审阅并信任 hooks**，桌宠不代替信任操作。CLI 与 Desktop 必须使用同一个 Codex 配置环境。独立 CLI 的 `hooks/list` 检测通过不能证明 Desktop 已重载；必要时重启或恢复 Desktop 会话，再触发真实事件。云端编排即使使用本地工具也不支持此类命令 hooks。官方说明：[hooks 与信任](https://learn.chatgpt.com/docs/hooks)。

包内 `codex-marketplace` 仍支持手动安装；源码模板在打包前不含 exe。在已解压交付包目录执行：

```powershell
codex plugin marketplace add .\codex-marketplace
codex plugin add firefly-pet@firefly-local --json
```

若该市场已指向旧目录，先用 `codex plugin marketplace list --json` 记录旧路径，再执行 `codex plugin marketplace remove firefly-local`，重跑以上命令。不要清空其他插件或配置。核对返回的 `installedPath` 内 exe 与包根目录 SHA-256 一致。桥接插件 manifest 为 `0.2.0`，桌宠产品版本仍为 `0.1.0`；不能只凭版本号判断缓存已更新。手动安装需保留包目录；一键安装使用稳定数据目录，移动试用包不影响已部署资源。变更 hooks 后均需重新审阅和信任。

| 事件 | 播报或用途 | 语义 |
| --- | --- | --- |
| `Stop` | Codex 这一轮回复结束了。 | 不代表整个任务成功 |
| `PermissionRequest` | Codex 发起了权限请求。 | 不推断一定需要人工审批 |
| `Interrupt` | 这一轮已中断。 | 不推断中断原因 |
| `SessionStart` / `UserPromptSubmit` | 生命周期与提交输入状态 | 不单独播报 |
| `PreToolUse` / `PostToolUse` / `SessionEnd` | 关联工具请求、解除等待、结束会话 | 不单独播报 |

桥接 `PetDesktop.exe --codex-hook` 从 stdin 读取最多 1 MiB 的 hook JSON。协议 v2 只传递事件类型、会话／轮次标识、可选工具请求标识、工具名及投递时间，兼容 v1 三类消息；`SessionStart` / `SessionEnd` 允许缺少轮次。提示词、回复、工具参数及 transcript 路径被丢弃。单条管道消息最多 2048 字节，结果仅存内存。结束／中断按会话和轮次去重；权限请求有标识时按请求去重，缺失标识时每次接收独立提醒。单个待执行工具可关联处理状态，但不据此合并缺失标识的权限提醒。

插件使用 `async: true`、2 秒超时。桥接连接总期限 500 ms、写入等候最多 200 ms；写入失败不重试。桌宠未启动、无效输入或投递失败仍中立返回 `{}` 和退出码 0，绝不返回审批决定。批处理只定位桥接 exe，不拼接输入；非 Windows 为中立空操作。

管道 `\\.\pipe\FireflyPet-v1-<当前用户 SID>` 仅授权当前用户、拒绝远程客户端，不开放 TCP。相同用户的进程仍可投递，因此“已收到事件”不能鉴别发送进程身份。设置页演示按钮与 `PetDesktop.exe --codex-test` 明确标记为模拟，不计作真实接入证据。

### 当前会话与询问

只跟踪本机记录中 `originator = "Codex Desktop"`、`source = "vscode"` 的主会话；CLI、子代理和云端不进入列表。名称取 `session_index.jsonl`，缺失时使用项目名与短标识。读取目录取绝对 `CODEX_HOME`，否则为 `%USERPROFILE%\.codex`。只提取名称、项目、状态、标识和时间，结果仅留内存；正文流式跳过，不保存或上传提示词、回复、工具参数及凭据，不读取凭据文件。

启动按 Desktop 后台进程启动时间寻找最近记录；进程无法确认时检查最近七天并把活动状态标为待确认。每个文件从元数据及尾部最多 4 MiB 重建，随后监听目录变化、每秒读取新增数据，最多跟踪 1024 个文件。大型正文流式跳过，单条读取上限 64 MiB；名称索引增量读取。来源退出、超过 120 秒缺少新证据或无法解析时显示“状态待确认”，不会猜成完成。记录格式和进程布局属于本机已核实的适配接口，可能随 Desktop 升级变化。

识别同步 `request_user_input`、结构化异步询问及 `AgentMessage.questions`，普通回复中的问句不触发。同步工具返回解除对应等待；异步工具立即返回不解除，等待实际用户消息或提交输入事件。结束轮次移出当前列表，异步问题未回答时仍保留“待回答”；中断和会话结束清除等待。权限事件只表示发起请求，无法确认处理结果时显示待确认。

待回答和权限请求优先排列，设置页与 Codex 小框详情共享列表，长列表滚动。Codex 在前台也提醒；遵守勿扰、全屏抑制与语音开关，勿扰期间更新列表、不积压语音。真实双会话、询问、权限和语音验收边界见 `docs/VALIDATION.md`。

### Codex 账号额度

额度独立于 hooks 和“Codex 会话提醒”开关，不需安装插件。先查 PATH 绝对目录；未找到时查 `%LOCALAPPDATA%\OpenAI\Codex\bin\<16 位十六进制目录>\codex.exe`，排除 `.bak` 和缺失 exe 的目录，按 exe 修改时间选择。不修改系统 PATH；非标准安装仍可通过 PATH 提供 CLI。API Key 登录不伪造订阅额度。

桌宠启动立即查询，正常每 15 秒后台刷新；关闭面板、拖动、穿透、暂停输入均不停止同步。手动刷新立即触发，重复请求合并。一个隐藏常驻 `codex app-server --listen stdio://` 进程复用连接，调用 `account/read`（`refreshToken: false`）、`account/rateLimits/read`，并处理 `account/rateLimits/updated`。独立连接不能保证收到其他客户端全部通知，轮询补齐变化；“实时”指后台自动同步，数值仍取决于服务端更新速度。官方说明：[app-server](https://learn.chatgpt.com/docs/app-server)。

每次初始化及查询合计最多等待 10 秒，错误后按 2、4、8…最长 60 秒退避重连。工作目录为 CLI 所在目录，避免加载桌宠项目配置；账号由 Codex 自身登录环境决定，可能与使用不同配置环境的 Desktop 不同。额度查询不创建会话、不调用模型、不重置额度、不读取凭据文件。

优先解析 `rateLimitsByLimitId`，兼容旧 `rateLimits`，显示 `100 − usedPercent`，限制 0–100；未知保留未知。通知按额度标识和周期合并，保留其他窗口；在途完整响应不会覆盖更新的通知。重置及更新时间使用本地时间，不自动推断满额。

后台刷新保留上次成功值，失败显示原因、过期标记和上次成功时间；确认未登录、登录失效或账号变化时清除旧账号数据。小框显示所有窗口最低剩余值，任一未知则显示未知；有旧数据但刷新失败显示“已过期”。结果只存内存，不展示原始账号或错误响应。

服务使用单线程、有界通道和单帧 1 MiB 上限；长期累计输出不设 4 MiB 总上限，stderr 仅排空不展示。最多 64 个额度窗口，溢出明确报错。退出只回收桌宠自行创建的 Windows 作业及其子进程，不按名称结束用户 Codex，不开放监听端口。

GPU 沿用 Windows PDH `GPU Engine(*)\Utilization Percentage`：先合并同一 GPU 引擎的进程用量，再取最忙引擎，限制 0–100%，不把并行引擎相加、不显示显存。首次采样、驱动或权限不可用时显示未知。计数器缓冲区最多 4 MiB，长度竞争最多重读一次；没有外部监控进程。

## 系统提醒规则

- 每 5 秒读取 `GetSystemTimes`、`GlobalMemoryStatusEx`、`GetSystemPowerStatus` 和 Windows 网络连接状态。第一轮 CPU 或不可用的数据显示未知；无电池不伪造百分比。
- CPU / 内存 / GPU 默认启用，各自默认达到或超过 90% 持续 30 秒、每 5 分钟提醒。设置页分别调整阈值 1–100%、持续时间 5–3600 秒（步长 5 秒）、重复间隔 1–60 分钟；每 5 秒采样，触发时间按采样粒度计算。读数未知、低于阈值或关闭类别时清除连续计时。保存规则变化会重新计时并清除该类别已排队提醒；资源提醒独立冷却，不受消息队列固定 5 分钟去重限制。文字显示类别与当前占用，语音使用该类别固定录音。
- 使用电池时在 ≤20% 和 ≤10% 各提醒一次；接入外部电源或回到 >20% 重置。电源提醒反映外部供电连接，不声称电池一定正在充电。
- 互联网连接状态变化连续两个采样一致才播报，不把局域网连接等同于已连接互联网。所有提醒独立开关，关闭某项会移除其待播消息；普通相同标识提醒去重 5 分钟，资源提醒按独立规则冷却；权限请求按请求标识或独立接收区分。
- 最多排队 12 条，审批和极低电量优先，消息排队超过 45 秒即丢弃；每条气泡显示 7 秒。默认全屏、演示或系统报告用户不在场时抑制主动提醒。输入同步与提醒相互独立。

高占用规则保存在可选 `resource_alerts` 对象，旧配置缺失字段时使用默认值、保留其他偏好；全默认时省略对象。旧版会拒绝新字段，回退前先在新版恢复三类阈值／时间／间隔默认值、启用 GPU 并保存，使该对象省略；自定义表情同样按前文恢复后再回退。不要直接让旧版读取含新字段的配置。
## 结构与配置

本项目以本 README 记录可复用的架构、构建、接口与限制；没有额外项目记忆文件。

源码仓库为 [lkuliuying/agent-pet](https://github.com/lkuliuying/agent-pet)，主分支为 `main`。仓库保留源码、内置角色资源、锁定依赖清单、测试、插件模板、构建脚本与文档；`target/`、`artifacts/`、`.runtime/` 和插件目录中的 exe 不纳入版本控制。含有后方窗口内容的旧桌面截图 `docs/native-preview.jpg` 已于 2026-10-11 清理，同名文件仍保持忽略；其他文档中的模拟预览不代表真实账号或设备验收。

2026-10-11 本地清理后，`target/` 仅保留当前源码构建的 `target/release/PetDesktop.exe`；`artifacts/` 仅保留 `FireflyPet-v0.1.0-windows-x64-20261010-233331` 目录及同名 `.zip`、`.zip.sha256`。该交付包早于当前程序，保留原样用于历史交付追溯。最近一次功能修改前的源码基线、最终界面截图、启动与位置验证资料及入库哈希基线保留在 `.runtime/`，具体清单见 `docs/VALIDATION.md`。编译缓存、示例 exe 和较早的临时产物已清理；后续测试、构建和预览会重新生成所需产物，首次编译耗时会增加。

| 文件或模块 | 职责 |
| --- | --- |
| `src/app.rs`、`ui/app.slint` | UI 状态绑定、生命周期、交互和托盘；不直接采集系统或键盘数据 |
| `src/app/usage_panel.rs`、`ui/status-panel.slint` | 点击信息面板、共享后台额度和会话列表，关闭仅影响显示 |
| `src/usage.rs`、`src/platform/codex_usage.rs`、`src/platform/quota_service.rs` | 额度解析与请求状态、Codex stdio 查询、超时与自建进程回收 |
| `src/platform/gpu.rs`、`src/platform/system.rs` | 原生 GPU 引擎聚合、CPU/内存/电源/网络采样；不可用读数保留未知 |
| `ui/settings.slint`、`ui/quick-menu.slint`、`ui/controls.slint` | 分模块设置、独立快捷菜单和共享界面控件 |
| `src/ui_state.rs` | 快捷操作候选配置与草稿合并、先保存后生效、资源读数和旧位置坐标换算 |
| `src/keyboard.rs` | 物理键元数据、按设备维护按下集合、最近按键标签、重复按下合并和左右手时限；布局数据只用于识别与分类 |
| `src/gaze.rs`、`src/gaze_render.rs` | 全屏鼠标坐标换算、朝向防抖和注视平滑；眼部遮罩、预乘透明度合成及单帧缓存 |
| `src/expressions.rs` | 表情稳定标识、场景配置验证、交互时刻与纯场景优先级选择 |
| `src/app/expression_settings.rs`、`ui/expression-settings.slint` | 表情草稿转换、页内预览资源绑定和场景编辑控件 |
| `ui/character.slint` | 桌宠与设置预览共用的角色图层、镜像和手部动画 |
| `src/resource_alerts.rs`、`src/app/resource_settings.rs`、`ui/resource-alert.slint` | 高占用规则验证、设置草稿转换和三类规则控件 |
| `src/sessions.rs`、`src/platform/codex_sessions.rs`、`src/platform/session_changes.rs`、`src/platform/codex_presence.rs` | 状态归并、记录增量读取、目录通知和 Desktop 进程识别 |
| `src/platform/integration.rs`、`ui/session-list.slint` | 本地插件安装诊断和两处共享会话列表 |
| `src/notices.rs` | 告警状态机、有界队列、去重与勿扰规则 |
| `src/bridge.rs` | hooks 最小字段解析、协议验证和固定播报文案 |
| `src/config.rs` | 版本化配置验证、原子保存和损坏文件备份 |
| `src/assets.rs`、`assets/firefly/character.json` | 角色清单 v1、图集裁剪、分层锚点与表情顺序 |
| `src/voice.rs`、`assets/voice/` | 18 条内置流萤语音、WAV 校验、采样音量缩放和可取消的播放队列 |
| `src/platform/` | Raw Input、窗口样式、当前用户管道、系统采样和原生 waveOut 音频播放的 Win32 封装 |

UI 线程每 16 ms 合并标签、淡出与手部状态；原始输入线程每 8 ms 泵消息，重复按下不排队。标签在输入事件到达时保存最近一次按下的快照，短于一帧的快速点击仍可显示；到期清除，不保存历史。Raw Input 使用 `RIDEV_INPUTSINK | RIDEV_DEVNOTIFY`，扫描码的 E0 位区分左右修饰键、主区和数字区 Enter；Pause / PrintScreen / NumLock 单独归一化。设备断开清除该设备及相关标签，锁屏及电源广播清空全部状态，每 500 ms 只核对仍按下键的真实 VK 防止丢失松键。手部动作 110 ms；空闲表情默认超过 90 秒变为困倦，可在角色表情模块调整。

现有 UI 定时器每至少 32 ms 读取一次 `GetCursorPos`，按窗口物理坐标、系统 DPI、角色大小、顶部留白和呼吸偏移换算，不新增线程或鼠标钩子。朝向以固定舞台中心判定，避免镜像后的脸部位置形成反馈。双眼共用注视方向，平滑时间常数为 80 ms，最大横移 3、纵移 2 逻辑像素，困倦表情纵移限制为 1；遮罩限制最终可见区域。素材仅启动时解码，遮罩预计算；偏移量化到四分之一源像素后缓存最后一张合成表情，不逐帧处理整张图集。

采样、桥接、额度查询和语音各有专用线程及有界消息通道。退出时设置共享停止标识、取消未完成管道 I/O 与额度查询、清理窗口和 Raw Input 注册、停止语音并 join 工作线程。没有运行时线程无限重试或按键动画队列。语音队列容量为 8，单线程串行播放；关闭语音、进入勿扰和退出通过独立取消信号停止当前音频，旧请求直接失效，不补播。试听可在语音总开关关闭时单独播放，但仍遵守当前生效的勿扰设置。

配置位于 `%LOCALAPPDATA%\FireflyPet\settings.json`。`version` 必须为 1，大小上限 64 KiB。写入临时文件并 `sync_all`，随后 `MoveFileExW(REPLACE_EXISTING | WRITE_THROUGH)` 原子替换。损坏、未知版本或未知字段配置会保留到同目录 `settings.invalid-<时间戳>.json`，设置页提示恢复；备份失败时不覆盖损坏原文件。有效设置保留位置、大小、键盘布局、各项开关和语音音量，不含按键或对话内容。

旧配置中的 `tkl` 字段仍按原值读写以保持兼容，但不再影响界面；其他设置沿用原值，不重置用户配置。`position` 仍表示旧 300×320 舞台的物理像素锚点：显示时减去 `30 × 大小比例 × 屏幕 DPI 比例` 的顶部偏移，保存时加回，避免重复升级/重启累积偏移。接近工作区边界时仍会进行可见范围约束。旧 `voice_id` 只在读取时兼容并忽略，保存时移除，不触发损坏恢复；其他字段完整保留。旧版语音开关沿用原值，新配置仍默认关闭。

角色图集为 2D 轻侧视透明 PNG，裁剪和定位由 manifest v1 描述，包括身体、左右前臂与手、装饰键盘，以及待机、眨眼、专注、开心、困倦、惊讶六张表情（提醒默认复用惊讶）。六张表情共享目标位置与比例，袖口重叠连接独立前臂。手部偏移已调整到键盘中部键帽，图集、身体、表情和键盘保持原样。呼吸与双手位移在 UI 运行时合成，键盘不随呼吸移动；加载时核对 PNG 实际尺寸、裁剪范围和动画最外边界。按键气泡靠键盘右上方，长标签向左扩展，超长内容省略；头顶提醒与角色之间保留间距，整机指标移入点击面板，没有常驻心情或资源文字条。首版仅内置流萤，资源嵌入 exe；尚未提供运行时导入其他角色的 UI。素材来源、生成提示及裁剪校准见 `assets/firefly/README.md`，已确认的角色造型基准见 `docs/firefly-2d-approved.png`。

语音来源、生成参数和新增文案流程见 `assets/voice/README.md`。所有固定通知使用明确的音频标识，Codex 对外协议保持 v1；动态错误仅播报“桌宠遇到了一点问题，请查看提示”，气泡保留具体原因。新增文案需要补充 WAV 并重新构建。打包检查包含 WAV 时间戳，便携包附带语音来源和哈希记录，模型及 Python 环境不进入包。

## 构建和验证

Codex 桥接验收需先退出桌宠再运行 `scripts/verify-package.ps1 -Directory <交付目录>`：脚本占用桌宠单实例锁，并用自己的接收端核对八类消息，避免夹具被用户桌宠当作 hooks 状态。它检查必需文件与哈希、marketplace 路由、manifest 引用、事件集合、exe 一致性及实际投递，不能仅以 `{}` 和退出码 0 判定接入成功。`tests/verify_package.ps1 -Directory <交付目录>` 还验证中文空格路径、缺失 exe、exe 不一致、错误市场路径和空操作脚本；临时夹具仅写入 `.runtime/`。这两项均不是 Codex 真实会话或桌宠 UI 验收。

2026-10-11 鼠标跟随功能验证会重新生成前次清理后的测试、编译缓存及预览程序；旧交付包保持原样，本轮程序以 `target/release/PetDesktop.exe` 为准。

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

预览还支持 `--left`、`--gaze=x,y`（屏幕方向，两个分量各在 −1…1）及 `--follow-mouse`。前两者使用固定模拟朝向与注视位置；最后一项读取当前全屏鼠标位置，不记录轨迹，不修改用户配置。可与 `--animate` 合用。

本机已执行检查、观察结果和待验收项统一记录于 `docs/VALIDATION.md`。不将静态素材预览等同于实际桌宠行为，不将自动化输入等同于物理键盘，不将渲染缩放模拟等同于修改系统 DPI，不将管道测试等同于真实 Codex hooks。

`ui_check` 是另一项原生开发示例，输出 60%/90%/160% × 六表情 × 待机/双手敲击、六个模块在两种窗口尺寸下的布局、六种表情页内预览及滚动末尾、快捷菜单及额度面板的正常/加载/失败/多额度/未知读数截图，并检查表情预览隔离、恢复默认、模块草稿、关于页保存按钮、右键分流、左键开关、拖动分流与菜单 Esc 回调。截图只来自示例渲染缓冲区，不采集桌面；输出目录必须尚不存在，不启动输入监听/管道/托盘/额度查询或读写用户配置，不随便携包交付：

```powershell
.\scripts\native-cargo.ps1 -CargoArguments @('build','--release','--example','ui_check','--locked','--offline')
.\target\release\examples\ui_check.exe .runtime\ui-review-new
```

追加 `--gaze` 切换为眼部验证矩阵，输出 60%/90%/160% × 左右朝向 × 六表情 × 九注视方向共 324 张自身渲染截图，并检查镜像后的左右手映射和拖动释放。原有菜单、面板与草稿断言继续执行：

```powershell
.\target\release\examples\ui_check.exe .runtime\gaze-review-new --gaze
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
