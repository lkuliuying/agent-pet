# v0.1 2D 轻侧视版与历史验证记录

更新日期：2026-10-11。工程：`F:\Program\pet_desktop`。本文件记录本机证据，不表示 Windows 10/11 全部设备兼容性已经验收。

## 源码入库前复核：2026-10-11

按用户要求初始化本地 `main`，将 `origin` 配置为 `https://github.com/lkuliuying/agent-pet.git`；配置前通过 GitHub 仓库元数据和 `git ls-remote --symref` 确认远程为空、默认分支为 `main`。本地仅设置 `core.autocrlf=false` 保留源码换行，不修改全局 Git 配置，沿用已有提交身份。

提交范围为原生程序与角色资源、测试与预览示例、Codex 本地插件模板与交付脚本、项目文档和模拟验证图片，共 58 个文件。`target/`、`artifacts/`、`.runtime/` 与插件目录的 exe 继续忽略；`docs/native-preview.jpg` 含后方窗口内容，增加精确忽略规则并保留本地原图。对提交候选的凭据模式扫描未发现匹配，4 个 JSON 文件解析和 4 个 PowerShell 脚本语法检查通过；这些检查不等同于完整安全审计。

在正常 Windows 用户会话使用现有依赖缓存复核，未安装或升级依赖：

| 命令 | 观察结果 |
| --- | --- |
| `scripts/native-cargo.ps1 test --locked --offline` | 40 项库测试、1 项应用绑定测试、1 项 hooks 进程测试、3 项额度进程测试通过；1 项真实账号测试按原有默认设置忽略 |
| `scripts/native-cargo.ps1 -CargoArguments @('clippy','--all-targets','--locked','--offline','--','-D','warnings')` | 通过 |
| `scripts/native-cargo.ps1 -CargoArguments @('fmt','--all','--','--check')` | 通过 |
| `rustfmt --edition 2024 --check tests/support/usage_fixture.rs` | 通过 |
| `scripts/native-cargo.ps1 build --release --bins --examples --locked --offline` | 正式程序及两个原生示例构建通过 |

首次暂存后执行 `git diff --cached --check`，发现 `.cargo/config.toml`、`Cargo.toml` 末尾空行，以及 `ui/app.slint` 最后一行 CRLF 被当前 Git 检查视为尾随空白。逐文件 SHA-256 与初始化前基线一致，确认这三处为原有格式；本轮保留源码原字节并记录提示，不扩大为格式整理。`.gitignore` 是本轮修改文件，其末尾多余空行已移除。

项目记忆沿用 README：补充仓库地址、主分支与源码提交边界，移除“没有发布到远程”的旧当前状态说明。下方历次记录中的“未初始化 Git”等描述仍属于各自实施时点的历史事实。本轮仅调整 Git 忽略规则与文档，业务源码、依赖清单和锁文件保持原样；不新增许可、重打包、创建 Release 或执行真实账号及桌面交互验收。

## 点击状态面板与 Codex 额度：2026-10-11

整机 CPU/内存移出常驻角色窗口，左键点击角色保留开心表情并切换独立状态面板；再次点击、外部点击或 Esc 收起。面板显示按需读取的 Codex 剩余额度、重置时间和更新时间，设置页共享查询状态及刷新入口。查询独立于 hooks 开关，配置保持 v1；角色资源、尺寸、位置锚点、系统采样、告警及 hooks 协议保持原样。

### 已执行检查

通过 `scripts/native-cargo.ps1` 使用已有 MSVC/SDK 与锁定依赖执行；没有新增依赖包或修改 Cargo.lock，仅启用现有 windows 库的 JobObjects 与 Time 功能。

| 命令 | 观察结果 |
| --- | --- |
| `scripts/native-cargo.ps1 check --locked --offline` | 通过；首次发现 Slint 弃用属性，已改用 `content-width`，后续构建无该警告 |
| `scripts/native-cargo.ps1 test --locked --offline` | 最终正常用户会话下 40 项库测试、1 项应用绑定测试、1 项 hooks 进程测试、3 项额度进程测试通过；1 项真实账号测试默认不运行，已另行执行 |
| `scripts/native-cargo.ps1 -CargoArguments @('clippy','--all-targets','--locked','--offline','--','-D','warnings')` | 通过 |
| `scripts/native-cargo.ps1 -CargoArguments @('fmt','--all','--','--check')` | 通过 |
| `rustfmt --edition 2024 --check tests/support/usage_fixture.rs` | 独立编译的原生夹具格式检查通过 |
| `scripts/native-cargo.ps1 build --release --bins --examples --locked --offline` | 最终正式程序及两个原生示例构建通过 |
| `scripts/native-cargo.ps1 -CargoArguments @('test','--test','usage_process','real_codex_quota_read_only','--locked','--offline','--','--ignored','--nocapture')` | 本机 Codex CLI 0.162.0-alpha.17.2 通过真实只读接口返回 1 个额度窗口，测试通过；只输出数量，不输出账号或原始响应 |
| `target/release/examples/ui_check.exe .runtime/quota-ui-20261011-final` | 退出码 0，保存 58 张自身渲染缓冲区截图；默认隐藏、左键开关、右键分流、拖动取消、生产控制器查询合并、面板与设置共享结果及既有模块草稿断言通过 |
| `scripts/smoke-desktop.ps1` | 首次被既有实例的单实例保护停止；用户从托盘退出后复跑通过，隔离配置 v1 与位置写入成功、stderr 为空、自动退出码 0，退出后未发现残留桌宠进程 |
| `git -c core.autocrlf=false diff --no-index --check -- <基线> <当前文件>` | 对相关源代码、UI、测试、示例、README 与清单执行，未报告空白错误；目录不是 Git 仓库，差异比较返回 1 表示存在预期变更 |

测试覆盖新旧额度响应、多类别与非固定周期、缺失数据、0/100 和越界百分比、重置时间溢出、账号类型、错误分类、重复请求合并及过期结果丢弃。原生进程夹具验证初始化/账号/额度请求顺序、成功返回、退出、畸形与超长响应、网络错误、未登录、API Key、超时、取消，以及自建主进程和后代进程回收；不使用真实凭据。

验证中遇到的问题：沙箱内既有命名管道测试返回 `0x80070002`，格式工具返回 `os error 5`；正常用户会话复跑通过。最初的 PowerShell 进程夹具在时限内未进入脚本，导致三个夹具测试超时；独立诊断也观察到 3 秒内未写入启动标记。已改为用现有 rustc 编译标准库原生夹具，保持原超时与回收断言，三个测试最终约 2 秒完成。没有扩大查询超时、跳过或削弱失败断言。

### 界面与证据边界

58 张截图包含原有角色 60%/90%/160% 与表情矩阵、两种尺寸的五模块设置、快捷菜单，以及 8 张正常/加载/错误/多额度/未知/耗尽面板截图。已检查面板正常读数、错误提示、长名称与多项滚动、设置小窗口，以及去掉指标条后的角色。面板字体保持系统 DPI 尺寸，不跟随角色缩放。代表性截图：`quota-panel-native.png`、`quota-settings-native.png`；均为模拟数据，不是真实账号额度截图。

最终 Release exe SHA-256：`261f4c29da79903e161e54667eba66373824a789aea8c20a086fc99c01a15ccf`。真实额度接口的成功证明查询链路可用，不替代与 Codex 界面逐项核对，也不代表真实多屏/DPI、物理外部点击、Esc 及焦点恢复均已验收。原生绑定模拟、状态机与几何测试已覆盖对应逻辑；真实桌面行为仍需目标电脑试用。

最终隔离启动使用 `.runtime/smoke-20261011-095846`，没有修改日常配置。10.001 秒、24 个逻辑处理器的样本中，桌宠进程占整机 CPU 约 0.475%，Working Set 290.8 MiB，Private Memory 381.8 MiB。这是短时进程用量样本，不代表面板整机资源读数或性能提升；启动检查时面板默认隐藏，没有触发额度查询。

### 项目记忆与范围复核

本轮读取并更新 README 与本验证记录。README 是指定项目记忆，无独立记忆文件；原记录与改动前源码一致，没有历史冲突。本次同步了点击显示、独立额度查询、刷新/取消/失败语义、可执行文件发现方式、模块职责及验证命令，并将“无上传”表述明确为不上传按键或对话内容。旧指标条描述已从当前使用说明移除，下方旧版本证据保留为历史。

实施前基线保存在 `.runtime/quota-panel-baseline-20261011-092933`。逐文件哈希确认 Cargo.lock、配置模块、桥接、提醒、系统采样及键盘模块未改动。未初始化 Git，未安装依赖、覆盖历史交付、修改日常配置、提交、发布或部署。

## 历史记录：手部、快捷菜单与分模块设置：2026-10-10

本次沿用已确认的三张界面预览，实现左右手锚点调整、头顶整机 CPU / 内存指标、独立快捷菜单与五模块设置。配置继续使用 v1，保留旧 `tkl`、音色 ID 和舞台位置语义；图集、依赖、输入和提醒规则及 Codex 协议未修改。

### 工程检查

通过 `scripts/native-cargo.ps1` 使用已有 MSVC/SDK 和锁定的离线依赖缓存执行：

| 命令 | 结果 |
| --- | --- |
| `cargo test --locked --offline` | 33 项库测试、1 项应用绑定测试、1 项独立进程集成测试通过 |
| `cargo clippy --all-targets --locked --offline -- -D warnings` | 通过 |
| `cargo fmt --all -- --check` | 通过 |
| `cargo build --release --bins --examples --locked --offline` | 正式程序及两个原生示例构建通过 |
| `cargo build --release --locked --offline` | 正式程序构建通过 |

新增测试覆盖：快捷操作只合并对应草稿字段、保存失败不改变生效配置、v1 位置在 60%/90%/160% 和模拟 DPI 比例下往返、未知/无效资源数据、负坐标显示器与边缘菜单定位、不可用音色及旧字段保留。已有键盘、资源、告警、配置恢复、hooks 中立退出和 Win32 管道测试保留并通过。

过程中遇到的限制和修正：首次命名管道测试在沙箱中返回 `0x80070002`，正常用户会话复跑通过；rustfmt 和 Clippy 在沙箱中遇到 `os error 5`，正常用户会话通过。格式检查发现新增模块声明顺序，已修正。首轮截图示例因最后一个窗口隐藏而提前退出，已保留预览窗口至检查结束；截图写入使用缓冲，避免大量小文件写操作。没有跳过、删除或弱化测试。

### 原生界面检查

最终运行 `target/release/examples/ui_check.exe .runtime/ui-review-20261010-verified`，获得 50 张实际 Slint/Winit/FemtoVG-WGPU 渲染缓冲区截图，进程退出码 0。它们是共享生产 UI 的模拟状态，不是真实物理键盘、Codex 或整机占用采样证据。

- 36 张角色状态：60%/90%/160% × 六种表情 × 待机/双手敲击，并叠加呼吸边界、长按键标签、提醒与指标条。检查代表性最大/最小尺寸和手部细节，指尖已位于中部键帽，袖口保持重叠，各信息区域未越界。
- 10 张设置状态：五个模块在 860×600 和 680×440 的布局。仅当前模块展示，较小窗口内容滚动，侧栏和保存区域固定；关于页无修改时隐藏保存按钮。最终截图另复查保存栏分隔线及按钮固定尺寸。
- 2 张快捷菜单状态：暂停、勿扰及穿透的开启/关闭文案；2 张浅/深底色的默认大小角色状态，验证未知指标与短按键标签。
- 示例断言通过：模块切换和重复 `show` 保留草稿；关于页有其他模块修改时显示保存按钮、无修改时隐藏；右键只触发快捷菜单回调，不触发抚摸；Esc 触发菜单关闭回调。生产程序的外部点击、原生焦点和托盘行为仍需真实桌面验收。

交付保存三张代表性原生模拟截图：`ui-refinement-native-pet.png`、`ui-refinement-native-menu.png`、`ui-refinement-native-settings.png`。截图只读取示例渲染缓冲区并转换为 PNG，没有采集其他应用或桌面内容；开发示例和完整截图矩阵不随免安装包交付。

### 启动检查与待验收项

隔离位置脚本 `.runtime/verify-position.ps1` 首次在沙箱中因 `Shell_NotifyIconW(NIM_ADD) failed` 未通过。在正常用户会话检测到旧便携包桌宠运行，位置检查和 `scripts/smoke-desktop.ps1` 均被单实例保护安全停止。经用户授权正常关闭窗口后，旧版仍驻留托盘；Computer Use 恢复旧设置窗口返回 `foreground window did not report a process id`，最终由用户从托盘退出旧版，再执行以下检查。未强制终止用户实例。

- `.runtime/verify-position.ps1`：60%/90%/160% 各连续启动两次，共 6 次；全部退出码 0、stderr 为空，位置保持 `(500,240)`，配置 v1、旧 `tkl`、暂停/勿扰和不可用音色 ID 均保留。只使用工作区内新建的隔离配置；这验证本机当前 DPI 下的实际重启，不代表切换系统 DPI 或多屏已经验收。
- `scripts/smoke-desktop.ps1`：正式 Release 程序启动、写入位置与自动退出通过，退出码 0、stderr 为空。10.001 秒样本、24 个逻辑处理器，桌宠进程占整机 CPU 约 0.521%，Working Set 250.2 MiB，Private Memory 329.2 MiB；这是短时进程用量样本，不代表性能提升或头顶整机资源读数。
- 最终 exe SHA-256：`f81db9566a9852249b6590a16d37ab37cb39e22de8253762f949c1012de2b9a8`。图集 SHA-256 保持 `4de12e1c9886e67edd8086052b21cbb0619f019ce17e083b074cc96a76c29df4`。
- 历史交付包保留。README、资源说明和本记录同步了新菜单、模块划分、指标位置、锚点换算及实际验证边界；没有新增项目记忆系统。

真实多屏边缘定位、切换系统 DPI、点击外部关闭及焦点恢复、穿透后的托盘恢复、物理键盘同步、系统音色播放和真实 Codex hooks 仍待目标电脑试用。最小 60% 时文字较小，默认 90% 更适合日常使用。没有新增依赖、安装器、Git 操作、远程发布或日常配置迁移。

## 2D 轻侧视改版：此前验证记录

本轮按用户确认图重制为白色制服、灰银长发、青绿色发尾、蓝紫眼睛的 2D Q 版侧视造型。键盘空格键靠近角色，背沿与接口朝向用户。300×320 舞台、默认 90%、现有输入和播报规则、配置 v1 与旧 `tkl` 兼容均保留。

### 工程检查

通过 `scripts/native-cargo.ps1` 配置当前进程的 MSVC/SDK 环境，在正常用户会话中完成以下检查；`--offline` 使用现有锁定依赖缓存。

| 命令 | 结果 |
| --- | --- |
| `cargo test --lib assets:: --locked --offline` | 6 项资源测试通过 |
| `cargo test --locked --offline` | 最终源码的 28 项库测试 + 1 项独立进程集成测试通过，0 失败 |
| `cargo clippy --all-targets --locked --offline -- -D warnings` | 最终源码通过，包括预览示例 |
| `cargo fmt --all -- --check` | 通过；实际格式化仅针对本轮修改的两个 Rust 文件 |
| `cargo build --release --bins --examples --locked --offline` | 正式程序及原生预览示例构建通过 |
| `cargo build --release --locked --offline` | 正式程序最终构建通过 |

资源测试覆盖 RGBA PNG 头、图集实际尺寸不匹配、截断输入、裁剪整数溢出和越界、无效浮点数、表情顺序/数量、图层宽高比、六表情目标位置一致，以及呼吸和最大下压时的舞台边界。既有键盘、告警、配置恢复、旧 `tkl`、桥接协议和真实 Win32 管道测试继续通过。

过程中遇到的失败与处理：受限会话中的 `rustfmt` 返回 `拒绝访问。 (os error 5)`，在正常用户会话执行后通过；Clippy 指出预览示例的 `step % 2 == 0` 应使用 `is_multiple_of(2)`，已修正并复查通过；一次构建因正在运行的预览实例占用 exe 返回 `LNK1104`，预览自动退出后重建通过。没有删减或弱化测试。

### 原生视觉观察

本轮使用实际 Slint/Winit/FemtoVG-WGPU 原生 `visual_check` 示例；该示例与正式程序共享 UI、图集、清单和呼吸幅度，但只设置模拟展示状态，不采集键盘、不启动管道/托盘、不写配置。示例不随便携包交付。可选验收底色默认为透明，正式程序不设置这一内部展示属性。

- 默认 90% 窗口约 270×288，原生观察待机、眨眼、专注、开心、困倦轮播；另以 60% 静态惊讶覆盖全部六表情。没有明显表情位置跳动或袖口断层；键盘为面向角色的斜视方向。
- `visual_check.exe 'Space' 0 0.9 --animate --background=light`：观察六状态轮播中的标签、提醒显隐及手部模拟驱动，浅色合成清晰。截图 `firefly-2d-native-light.jpg`。
- `visual_check.exe 'Ctrl + Alt + Shift + Win + Print Screen' 4 1.6 --background=dark`：480×512 原生窗口内长标签未越界，标签与头顶提醒同时显示且不遮脸。截图 `firefly-2d-native-dark.jpg`。
- `visual_check.exe 'Enter' 5 0.6 --background=light`：180×192 窗口中图层、惊讶表情和两个气泡均未裁切；最小尺寸文字较小，默认 90% 更适合日常观看。截图 `firefly-2d-native-small.jpg`。
- 透明模式原生预览可透出后方窗口；为避免将后方私人内容带入交付，保存的三张截图使用开发预览底色。它们是原生模拟状态截图，不是真实 Codex 或物理输入证据。
- 辅助 HTTP 资源页连接超时，未作为验收依据。最终视觉结论来自上述原生窗口。

### 正式程序启动与采样

`scripts/smoke-desktop.ps1` 使用工作区内独立测试配置启动最终 exe：位置写入成功、自动退出码 0、stderr 为空。10.002 秒、24 个逻辑处理器下，整机 CPU 样本为 0.508%，Working Set 215.1 MiB，Private Memory 251.0 MiB。这是没有持续输入的短时样本，不是输入延迟、长期内存或性能改进结论。

- 正式 exe SHA-256：`f2c49259bb02b4193c59ef229117bc22b5c1b7eea65c025e7be00e889c950891`。
- 新图集 SHA-256：`4de12e1c9886e67edd8086052b21cbb0619f019ce17e083b074cc96a76c29df4`。
- 另以 `--data-dir=<工作区内隔离目录> --exit-after=90` 启动正式程序，进程响应正常、窗口标题为“流萤 · 键盘桌宠”；Computer Use 未枚举出该无激活工具窗口。没有绕过工具接口控制该窗口。

### 便携包预检

`scripts/package-windows.ps1 -SkipBuild` 生成了独立时间戳的 2D 候选包；`scripts/verify-package.ps1 -Directory artifacts/FireflyPet-v0.1.0-windows-x64-20261010-161754` 检查通过。所有清单文件哈希一致，Stop / PermissionRequest / Interrupt 三个插件命令夹具均返回 `{}`、退出码 0，分别耗时 320 / 140 / 141 ms。验证时没有运行中的桌宠实例；这些结果只证明本地命令夹具和未连接时的中立退出，不代表真实 Codex 接入。

候选 zip 共 599 个条目，包含确认图、三张原生预览、文档和插件副本，没有开发预览 exe、`.runtime` 或 `target` 内容。正式 exe、包根目录 exe、插件内 exe 的 SHA-256 均与上方一致。首次手工核对误用了不存在的插件 `bin` 子目录，按包内实际路径 `codex-marketplace/plugins/firefly-pet/PetDesktop.exe` 重查后一致。新旧四个 zip 均与各自 `.sha256` 文件一致，之前三个交付包保留。本记录补充后重新生成最终时间戳包，并再次执行包校验；最终文件名与 SHA-256 以交付目录随附清单为准。

### 本版仍待验收

真实物理键盘到桌宠的完整输入链路及延迟、点击开心/拖动惊讶、不抢焦点、穿透与托盘恢复、多屏拖动和真实 Windows 100%/150%/200% DPI、锁屏/休眠/设备断开、真实系统提醒、中文语音听感、真实 Codex 三类 hooks 和干净目标机器兼容性仍未完成。本轮界面缩放及模拟动作不替代这些项目，历史观察也不自动视为新版通过。

### 文档与变更边界

实施前读取 README、资源说明和本验证记录；未发现独立项目记忆，README 明确承担这一职责。旧文档与旧立体版实现一致。本轮同步 README 的 2D 造型、气泡位置、资源校验与预览命令，更新资源说明并保存实际生成提示和确认图，历史证据保留在下方。没有修改依赖/锁文件、输入采集、系统采样、语音、Codex 协议、真实用户设置或旧交付包，没有 Git 操作或发布。

---

## 历史记录：立体风格紧凑版

以下为上一轮立体风格紧凑版的原有记录，仅供追溯；其中图集、哈希和“本轮”均指历史候选，不代表当前 2D 版本。

本轮将舞台改为 300×320、默认 90% 下约 270×288 逻辑像素，使用新立体风格图集、三排装饰键盘和独立的最近按键标签。旧 `tkl` 字段继续兼容读写，完整键盘布局和常驻心情条已退出界面。

### 工程与状态机

- `cargo test --locked --offline`：23 个库测试和 1 个独立进程集成测试通过，0 失败。新增覆盖物理键名、左右修饰键合并与固定顺序、数字键盘、最新键替换、重复按下、短于一帧的点击、长按、多设备、800 ms 保持及 120 ms 淡出、暂停/锁屏清理入口、相关设备断开和较早按键遗漏松开事件后的清理；旧 `tkl` 两种值均保留其他配置。
- `cargo clippy --all-targets --locked --offline -- -D warnings`：通过，包含新增原生素材预览示例。
- `cargo fmt --all -- --check`：通过。
- `cargo build --release --example visual_check --locked --offline`：通过。示例使用同一 Slint UI 和图集，只显示模拟数据，不监听输入、不启动管道、不写配置，不随便携包交付。
- `cargo build --release --locked --offline`：通过，最终 exe SHA-256 为 `a7b81efd8b2d1d3d86c50477df8e32278cf926e21dee5a6bcda841841826b1bf`。
- `scripts/smoke-desktop.ps1`：最终 exe 启动、位置写入隔离配置及自动退出检查通过，退出码 0，stderr 为空。10.001 秒、24 个逻辑处理器下观察到整机 CPU 0.488%、Working Set 212.8 MiB、Private Memory 247.0 MiB；这是无持续输入的短时样本，不是性能承诺或原生交互验收。
- 首次沙箱内库测试 21 项通过、命名管道测试返回 `系统找不到指定的文件 (0x80070002)`；相同管道测试随后在正常用户会话通过。没有修改管道逻辑或弱化测试。

### 资源几何预览与原生验收边界

原生候选已通过进程检查确认运行、窗口标题为“流萤 · 键盘桌宠”且响应正常，但无激活工具窗口未被 Computer Use 枚举。随后对独立 `visual_check` 示例的桌面访问被工具拒绝，返回 `Computer Use was not approved to use visual_check`。此后停止该桌面操作，没有改用其他接口控制同一个窗口。

改用仅绑定 127.0.0.1 的临时静态资源页读取同一图集、清单和定位参数，检查合成几何；该页面不是 Slint 原生窗口，也不是键盘端到端测试。预览观察到：

- 60%、90%、160% 三种大小的身体、表情、手部、键盘均在舞台内；六种表情完整，双手位于键盘上，没有明显图层断层。
- 默认舞台约 270×288，60% 约 180×192，160% 约 480×512 逻辑像素。
- `A`、`Space`、`Ctrl + C`、`Num Enter`、`Ctrl + Alt + Shift + Win + Print Screen` 等示例标签均未越界；DOM 读取确认 `scrollWidth <= clientWidth`。
- 浅色与深色背景透明合成正常；头顶提醒与键盘上沿标签同时显示时不遮脸。
- 截图：`compact-preview.jpg` 为默认比例局部，`compact-preview-grid.jpg` 为完整六状态检查。截图是模拟资源预览，不作为真实 Codex、原生字体渲染或按键延迟证据。

本轮原生拖动、真实物理输入、不抢焦点、穿透恢复及 Windows DPI 实测仍待验收。此前版本已观察的操作不能自动视为新版通过。

### 范围与项目记忆

本轮开始读取 README、资源说明和本验证记录，并检查现有源代码；未找到额外指定项目记忆。旧说明与改版前实现一致，没有发现历史记忆冲突。README 已同步紧凑尺寸、标签语义、旧配置兼容、模块职责、素材与预览方式；资源说明保存了两轮内置 imagegen 的实际提示。没有新增依赖、修改 Codex 插件协议或真实用户配置，没有初始化或提交 Git，也没有发布。旧交付保留。

---

## 历史记录：改版前完整键盘候选

以下原样保留上一轮完整键盘版本的证据；涉及 104/87 键、旧图集、旧尺寸和旧哈希的描述仅用于历史追溯，不代表当前紧凑界面。

## 已执行的工程检查

在 Windows 正常用户会话中，通过 `scripts/native-cargo.ps1` 配置当前进程 MSVC / SDK 环境后执行：

| 命令 | 观察结果 |
| --- | --- |
| `cargo test --locked --offline` | 16 个库测试 + 1 个独立进程集成测试通过，0 失败 |
| `cargo clippy --all-targets --locked --offline -- -D warnings` | 通过 |
| `cargo fmt --all -- --check` | 通过 |
| `cargo build --release --locked --offline` | 通过，生成 x64 `PetDesktop.exe` |
| `scripts/smoke-desktop.ps1` | 候选构建启动成功、隔离配置写入位置、自动退出码 0、stderr 无错误；最终包未重跑 GUI，见下文 |
| PowerShell `Parser.ParseFile` 和插件 JSON `ConvertFrom-Json` | 脚本与 JSON 语法检查通过 |
| `scripts/package-windows.ps1 -SkipBuild` | 生成 exe、插件、说明、许可、全部文件 SHA-256、zip 及 zip SHA-256 |
| `scripts/verify-package.ps1 -Directory <交付目录>` | 全部清单文件哈希一致，Stop / PermissionRequest / Interrupt 三个命令夹具均 `{}` / 退出码 0，观察耗时 315 / 136 / 136 ms |
| `dumpbin /dependents target/release/PetDesktop.exe` | 导入表为 Windows 系统 DLL，未出现 VC Runtime 动态依赖；动态 GPU 驱动依赖仍需目标电脑验证 |
| `Get-AuthenticodeSignature target/release/PetDesktop.exe` | `NotSigned`，程序未签名 |

测试覆盖：104/87 键总数及唯一性、E0 与 Pause/PrintScreen/NumLock、左右修饰键、主区及数字区 Enter、多设备按键合并、重复按下、左右手同时联动、松键、暂停及无效键；资源阈值、30 秒持续和 5 分钟冷却、无效采样、20/10 电量边界、网络去抖；提醒去重、容量限制、过期、勿扰、关闭分类清理；配置验证、原子替换、损坏备份、64 KiB 限制、拒绝无效设置且保留原文；hook 正文剥离、无效 JSON、过长消息；真实 Win32 管道传输、重连、畸形消息、空闲客户端超时、停止；三类 hook 和畸形输入在独立进程中均 `{}` / 退出码 0。

最初的离线构建因托盘依赖未缓存失败，随后仅下载本方案锁定依赖并生成 Cargo.lock。沙箱内 rustfmt 曾返回访问拒绝，命名管道测试返回找不到管道；正常用户会话中仍发现原连接逻辑丢消息，修复为可取消的 OVERLAPPED I/O 后通过。新增配置读取边界后曾出现 Clippy 风格错误，已修正并重新全部通过。没有跳过或削弱失败测试。

## 已观察的桌面行为

以下桌面观察来自 Release 候选。最终包另有原生窗口初始化时机修正，该修正仅复跑无界面测试、Clippy、格式检查及 Release 构建；不将候选截图视为最终包全部桌面行为的证明。

- Winit / FemtoVG-WGPU 窗口实际启动，背景透明，可见后方桌面应用；完整 104 键和 87 键布局显示正常。
- 修正表情缩放与手部锚点后，待机、开心、困倦、惊讶在实际窗口显示；左右手位于键盘上沿，不遮住整排键帽。
- 点击角色切换开心；拖动触发惊讶，实际位置由 `[1853,816]` 移至 `[1725,770]` 并写入测试配置。
- 右键打开设置，设置窗口获取焦点；关闭设置后桌宠继续运行。87 键设置保存并即时应用。
- 本机读到 CPU 与内存；无电池或数据不可用显示为“无电池或不可用”。枚举到 Windows 中文音色 Microsoft Huihui Desktop。
- 明确带模拟标记的 `PetDesktop.exe --codex-test` 经真实管道进入 UI，显示“联动演示”和“Codex 这一轮回复结束了。”，角色同步出现提醒表情。原始窗口截图 `native-preview.jpg` 背后可见其他窗口，属于透明背景证据；该截图仅在本地保留，不包含在源码仓库中。
- `SLINT_SCALE_FACTOR=1 / 1.5 / 2` 的三个进程级渲染缩放均观察到完整角色和键盘；这是渲染缩放模拟，没有修改 Windows 系统 DPI，不能替代真实系统缩放与多屏验证。

界面验证过程中，用户按物理 Esc 停止了 Computer Use。此后停止桌面控制；未完成的界面项目保留在下方验收清单。

## Codex 与插件证据边界

本机 CLI 版本：`0.162.0-alpha.17.2`。

- 在项目 `.runtime/codex-plugin-home` 内隔离配置，执行 `codex plugin marketplace add ... --json` 和 `codex plugin add firefly-pet@firefly-local --json`，均返回成功，识别版本 0.1.0。
- 这是市场 / 插件清单安装检查；没有修改实际用户 Codex 配置，没有信任 hooks，没有调用模型。
- `cmd.exe /d /s /c` 调用包内 `notify.cmd` 的带空格路径夹具已运行：退出码 0、stdout 为 `{}`、stderr 为空。
- 桌宠关闭时，独立进程 hook 测试验证了中立快速退出；没有把它描述成真实 Codex 会话测试。
- 用户仍需在真实 Codex 桌面应用里审阅并信任 hooks，再分别触发三类事件。当前“真实 Codex hooks 端到端验收”未完成。

交付包包含 Slint Royalty-free 2.0 等依赖许可文本及解析依赖清单。`SHA256SUMS.txt` 校验包内全部文件，zip 旁的 `.sha256` 校验压缩包。测试及打包不包含任何用户真实配置、hook 正文、密钥或运行时目录。

## 资源采样

`scripts/smoke-desktop.ps1` 对 Release 候选构建执行 10.002 秒采样：默认 104 键、90% 桌宠大小、设置窗口关闭、语音关闭、没有持续输入，24 个逻辑处理器。随后通过源码复核修正原生窗口首次初始化的设置应用时机，并重跑无界面工程检查；用户已停止桌面控制，因此没有对最终包重新启动 GUI 采样。

| 指标 | 本机样本 |
| --- | --- |
| CPU 占整机逻辑处理器总容量 | 0.449% |
| Working Set | 213.5 MiB |
| Private Memory | 265.7 MiB |
| 正常退出 | 退出码 0 |
| 采样候选 exe SHA-256（不是最终包哈希） | `efa0aa8c2b35dc562ce1d3a9db47c116a21448fa24555e6838f70a6b462467b0` |

这是一次本机短时样本，不是长期内存泄漏、所有硬件或高频输入性能结论。一次早期缩放测试在采样结束前自动退出，产生 0 内存读数，已判为无效且未使用。

真实物理按键到屏幕变化的端到端延迟尚未测量。代码中的 8 ms 消息泵和 16 ms UI 合并间隔是调度参数，不是实测输入延迟。

## 待人工或目标环境验收

| 项目 | 验收步骤 |
| --- | --- |
| 物理键盘与左右手 | 在记事本持续输入、同时按左右区域、长按、Ctrl/Shift/Alt 组合、左右修饰键、数字键盘及 NumLock 切换；所有松开后键帽恢复，提醒期间输入不中断 |
| 不抢焦点 | 在记事本保持输入，点头部、拖动桌宠；应继续向记事本输入。自动化工具会主动激活目标窗口，不能用其点击证明此项 |
| 鼠标穿透与托盘恢复 | 开启穿透并保存，点击桌宠所在位置应操作后方应用；点击本程序托盘图标恢复；测试托盘退出与暂停/勿扰菜单 |
| 系统 DPI / 多屏 | Windows 实际 100%、150%、200% 与不同 DPI 显示器间拖动、重启记忆、断开外接屏后的可见位置 |
| 锁屏、休眠、设备断开 | 按键后锁屏或睡眠，再恢复；拔掉对应键盘；没有残留按下状态 |
| 系统提醒 | 真实电池 20%/10%、供电切换、互联网连接变化、持续高占用及冷却；不要为测试故意耗尽电池或破坏网络任务 |
| 全屏勿扰 | 在实际全屏应用中发送模拟事件，期间不出现主动气泡/语音，退出全屏不补播积压消息 |
| 中文语音 | 试听和低音量播报，确认实际可听见、语句和气泡一致、关闭后及时停止；当前只验证音色枚举，未完成听感验收 |
| 真实 Codex | 按 README 安装并信任，在桌面应用触发 Stop / PermissionRequest / Interrupt；关闭桌宠后正常继续会话 |
| 目标机器 | 干净 Windows 10/11 x64、不同 GPU、无中文音色环境、管理员应用与普通应用；未做兼容性或代码签名认证 |

## 本次文件与项目记忆

实施前目录为空，未发现指定项目记忆、已有源代码或 Git 仓库。以新建 README 记录架构、模块边界、配置、构建、事件语义及接入限制；没有创建独立记忆系统，也没有改动全局记忆、真实 Codex 设置或其他项目。全部源码与交付脚本为本次新增，未执行 Git 初始化、提交、推送、安装或发布。
