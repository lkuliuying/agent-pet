# v0.1 2D 轻侧视版与历史验证记录

更新日期：2026-10-11。工程：`F:\Program\pet_desktop`。本文件记录本机证据，不表示 Windows 10/11 全部设备兼容性已经验收。

## Git 分批交付后补验：2026-10-11

前五批源码与文档提交已推送至 `origin/main`，补验源码基线为 `7cef0f9`。此前 hook 进程测试及包投递验证因运行中的桌宠占用单实例锁受阻；用户退出桌宠后，在正常本机权限下依次复跑，未修改源码、测试断言或单实例保护。

| 实际执行命令 | 观察结果 |
| --- | --- |
| `.\scripts\native-cargo.ps1 test --locked --offline` | 退出码 0；库 89、应用 4、hook 进程 1、额度进程 3，合计 97 通过、0 失败；4 项真实环境测试按既有设置默认忽略 |
| `.\scripts\verify-package.ps1 -Directory artifacts\FireflyPet-v0.1.0-windows-x64-20261011-194045` | 退出码 0；清单、市场路由及两个 exe 一致性通过；八类事件均实际送达隔离接收端，桥接进程均返回 `{}` 和退出码 0 |
| `.\tests\verify_package.ps1 -Directory artifacts\FireflyPet-v0.1.0-windows-x64-20261011-194045` | 退出码 0；中文空格路径正常，缺失 exe、exe 不一致、错误市场路径和无投递脚本四项反例均按预期拒绝，共 5 组通过 |

本次已解除上述测试阻塞；这些结果仍不代表真实 Codex 会话、hooks 信任后的 Desktop 投递、人工听感或跨设备验收完成。原交付包保持不变。本轮仅修正 README 中包校验“三类消息”的过时表述为与脚本一致的“八类消息”，并补充本节验证记录。

## 可配置监控、后台额度及 Codex 会话：2026-10-11

本节为当前实现及本轮证据，以下各节保留对应阶段的历史记录，不能据旧包或旧测试推断当前功能已完成真实环境验收。

### 实现与边界

- CPU、内存、GPU 三类独立规则：默认启用、达到或超过 90% 持续 30 秒、重复间隔 5 分钟。支持阈值 1–100%、持续 5–3600 秒且按 5 秒调整、间隔 1–60 分钟。沿用 5 秒采样和 GPU 最忙引擎口径。未知、低于阈值或关闭时重置连续计时；保存规则时立即重置计时和冷却，清除该类别排队消息。两次采样之间改动再改回也重置；1 分钟重复不再受固定消息去重压制。
- 额度启动即查、常态 15 秒轮询，使用一个隐藏的自有 app-server 进程，处理更新通知，按桶和周期合并。关闭面板不取消服务；定时查询保留旧值，错误显示过期和更新时间，确认登录失效或账号改变时清空旧数据。单次初始化及查询共 10 秒期限，失败退避 2–60 秒；单帧 1 MiB、有界通道和最多 64 个额度窗口，不再按累计 4 MiB 断开。退出只回收自己的 Windows 作业。
- 主会话适配只允许本机 `Codex Desktop` / `vscode` 来源，排除 CLI 和子代理。按进程启动时间发现最近文件，启动读取元数据及尾部最多 4 MiB，随后目录通知加每秒增量读取；名称索引也增量读取。大记录以 serde 白名单流式跳过正文，单条最大 64 MiB；来源退出、120 秒无新证据或格式不兼容显示待确认。只把名称、项目、标识和状态放入内存，不持久化或上传对话、工具参数及凭据。
- 同步请求等待对应工具结果；异步问题等待实际用户消息，工具即时返回不会解除。普通问句不触发。结束／中断后移除当前列表，未回答的异步问题在正常轮次结束后仍保留。权限有请求标识时去重，无标识时分别提醒；文字为“发起了权限请求”，不推断自动审批在等人工。
- 桥接协议 v2 兼容旧 v1，插件模板 0.2.0 提供 8 类 hook。一键接入只操作 `firefly-local` / `firefly-pet`，在桌宠自己的稳定数据目录部署，核对实际 `installedPath` 与已加载源文件，失败尝试恢复原市场登记。用户须在 Codex 中审阅、信任 hooks；程序不自动信任。`hooks/list` 的事件名使用 camelCase，与配置文件的 PascalCase 事件键不同。
- 设置保留六模块共用草稿和统一保存。两处会话列表共享数据，询问／权限优先；设置页的接入操作位于长列表之前。新增 GPU、等待回答语音，重做权限及模拟权限语音，全部为原流萤固定离线音色。勿扰／全屏抑制更新列表但不积压语音；Codex 在前台也会提醒。

### 已执行验证

Cargo 命令通过项目现有 `scripts/native-cargo.ps1` 运行，均使用锁定的离线依赖。

| 命令 | 本轮观察结果 |
| --- | --- |
| `scripts/native-cargo.ps1 test --locked --offline` | 最新完整执行：库 89、应用 4、hook 进程 1、额度进程 3，共 97 通过，0 失败，4 项环境测试按原有显式手动策略默认忽略。包含阈值相等、边界、独立冷却、保存重计时、同步／异步询问、乱序和同轮审批、旧协议、隐私白名单等 |
| `scripts/native-cargo.ps1 -CargoArguments @('test','--lib','platform::integration','--locked','--offline')` | 2 通过；安装／失败回滚、中文空格路径、其他 hooks 保留、实际文件和各信任状态校验 |
| `scripts/native-cargo.ps1 -CargoArguments @('clippy','--all-targets','--locked','--offline','--','-D','warnings')` | 通过，0 警告 |
| `scripts/native-cargo.ps1 -CargoArguments @('fmt','--all','--','--check')` | 通过；仅格式化本轮修改的 Rust 文件，未全仓改写 |
| `scripts/native-cargo.ps1 -CargoArguments @('test','--lib','real_desktop','--locked','--offline','--','--ignored','--nocapture')` | 真实账号只读额度查询通过，取得 1 个窗口；首次状态读取失败，排查及修复见下文，未隐藏失败 |
| `scripts/native-cargo.ps1 -CargoArguments @('test','--lib','real_desktop_metadata','--locked','--offline','--','--ignored','--nocapture')` | 修复后单独复跑通过：确认本机 Desktop，重建 1 个当前会话，无格式不兼容，无历史提醒重播。未输出名称、正文、账号或凭据 |
| `.runtime/verify-monitoring-cli.ps1 -Codex 'C:\Users\likecandy\AppData\Local\OpenAI\Codex\bin\2e5e00daee91c61d\codex.exe'` | 实际 CLI 0.162.0-alpha.17.2 的独立 `CODEX_HOME` 检查通过：0.1.0 升到 0.2.0，中文空格路径，8 类事件加载，逐文件哈希及已加载来源一致，保持 untrusted，已有 hooks 原字节不变。结果仅保留测试元数据，位于 `.runtime/monitoring-cli-20261011-192837/result.json`；未登记用户真实配置 |
| `target/release/examples/ui_check.exe .runtime/monitoring-ui-20261011` | 初轮 78 张自身渲染截图和交互断言通过；此后新增滚动模式，再验证新增布局 |
| `.runtime/monitoring-native-runners/ui_check.exe .runtime/monitoring-scroll-final-20261011 --monitoring` | 9 张截图及全部绑定断言通过：860×600、680×440 两种逻辑尺寸，3 类规则，长名称与 12 会话滚动，固定保存栏，关闭面板后共享额度由模拟 75% 更新为 64%，手动刷新合并 |
| `git diff --check` | 换行问题修正后通过；只规范本轮编辑文件，不改其他用户文件 |

额度常驻进程夹具覆盖通知和轮询交错、账号切换旧响应隔离、登录失效、请求合并、自建进程回收，以及连接累计超过 5 MiB 仍不断开；此测试不等于连续运行数日的稳定性验收。

两轮增量语音生成使用原离线环境和同一权重，分别执行 `scripts/generate-voice.py` 的 `--only gpu,codex-question`、`--only codex-permission,demo-codex-permission`，其余参数见 `assets/voice/README.md` 的复现命令。资源测试核对全部 18 条 PCM 格式、可听能量与气泡内时长；未安装新依赖，未把模型加入交付包。

初次完整测试因旧桌宠占用单实例而在 hook 夹具处停止；用户确认退出后完整通过。沙箱内曾出现本机路径和音频生成访问失败，转为正常本机权限后通过。真实记录首轮检测暴露大型正文和 Desktop 子代理来源差异，现已分别流式跳过正文及明确排除子代理；相应夹具与真实只读复跑均通过。真实 CLI 又发现 hooks/list 事件名大小写差异，已据真实返回修正适配和回归用例。滚动示例曾因关闭唯一窗口而提前退出，仅产生 8 张图；修正示例生命周期后明确检查 9 张图及结束断言通过。

### 最终构建与本机运行

- `scripts/native-cargo.ps1 -CargoArguments @('build','--release','--locked','--offline','--examples','--bin','PetDesktop')` 最终通过。应用 31,361,536 字节，SHA256 为 `e6b8058db1f42700639f0feb666e2f111b83ce9fc7800a2b6c8710954e524574`。
- `.runtime/monitoring-native-runners/voice_check.exe --all` 通过本机实际音频设备播放全部 18 条，退出码 0；当前播放取消 5 ms，取消后未出现积压播放结果；退出释放音频 19 ms。验证程序复制自本轮 Release 示例构建以避免后续链接占用；不等于人工听感、英文发音及细微漏字已验收。
- `scripts/smoke-desktop.ps1 -Binary target/release/PetDesktop.exe -SampleSeconds 20` 通过：真实应用用独立配置运行，stderr 为空、退出码 0、位置配置成功写入；20.002 秒样本 CPU 占整机 0.508%，Working Set 325.9 MiB、Private 421.8 MiB，24 个逻辑处理器。这里只是本机短时样本，不证明长期泄漏或其他设备兼容性。
- PowerShell `Parser.ParseFile` 检查 `scripts/verify-package.ps1`、`scripts/package-windows.ps1`、`tests/verify_package.ps1` 通过。`Cargo.lock` 与实施前 SHA256 一致。当前文档的三类配置、后台刷新和白名单读取说明已对照最终源码复核。

### 独立试用包校验

本轮新建 `artifacts/FireflyPet-v0.1.0-windows-x64-20261011-194045/` 及同名 ZIP、`.zip.sha256`；历史交付原样保留。运行 `scripts/package-windows.ps1 -SkipBuild` 成功，包内桌宠和桥接 exe 均为上述最终构建。

- `scripts/verify-package.ps1 -Directory artifacts/FireflyPet-v0.1.0-windows-x64-20261011-194045` 全部通过：清单、市场路由、两个 exe 一致性和 8 类事件实际投递到隔离接收端；每次返回 `{}`、退出码 0，初次校验分别耗时 348、130、127、120、124、124、124、126 ms。
- `tests/verify_package.ps1 -Directory artifacts/FireflyPet-v0.1.0-windows-x64-20261011-194045` 5 组全部通过：中文空格路径正常；缺失 exe、两个 exe 不一致、错误 marketplace 路径及无投递脚本按预期拒绝。原交付包未被反例改写，夹具在 `.runtime/package-check-e7400d49c08b45d98f58226e7efeb12b`。
- 使用 PowerShell / `ZipFile.OpenRead` 对 ZIP 内 606 个清单文件逐个 SHA256 核对，并复核 ZIP 与 `.zip.sha256` 一致；没有模型权重、Python、FFmpeg、外置 WAV、凭据或用户配置。补入本段验证记录后同步包内文档、重新计算清单与 ZIP 哈希，交付前再次核对。

### 本轮记忆同步与保留范围

读取指定项目记忆 `README.md` 和本文件后核对当前调用链。旧文档“仅展开时刷新”、固定 CPU／内存规则及三类 hooks 已被本轮替代，现同步后台刷新、规则配置、会话读取白名单／上限、安装及信任步骤、配置回退限制和验证边界。旧配置字段缺失使用默认值，非默认 `resource_alerts` 会被旧程序拒绝，回退前须在新版恢复相应默认并保存。没有新增记忆系统，也没有改全局记忆。

实施前已有大量未提交修改，源码及资源基线与 SHA256 清单位于 `.runtime/monitoring-baseline-20261011-183327`。本轮在其上增量实现；`Cargo.lock`、既有表情／跟随、生成脚本等无关工作保持原样。仅增加现有 Windows crate 的 ToolHelp 功能标志，没有新增或升级依赖。未提交、推送或发布。

### 仍需真实验收

目前真实证据为本机只读额度、主会话元数据、隔离 CLI 安装／加载，以及原生 UI／音频 API。尚不能宣称以下场景已端到端通过：用户账号数值在关闭面板时发生变化、两个并行 Desktop 主会话、真实同步／异步问题及回答后恢复、实际权限请求与信任后的 Desktop hook 投递、真实勿扰／全屏场景、跨设备和多屏 DPI。夹具通过不能代替这些验收，程序不能代替用户授予 hooks 信任。

实际验收顺序：启动本轮独立试用包；在设置保存需要的阈值和语音开关；点击安装／更新联动，在 Codex 中审阅并信任；重新检测后运行两个 Desktop 会话，核对询问／权限气泡、回答后的状态、勿扰不补播；关闭详情继续使用 Codex，至少等待 15 秒后重开核对额度和更新时间。不要为测试故意耗尽系统资源或破坏网络任务。

## 内置流萤 0.3 语音：2026-10-11

本节记录监控功能扩展前的 16 条语音验收；当前 18 条资源及本轮边界见上节。后文系统音色、SAPI 与音色枚举相关记录属于历史版本。

### 实现与资源

- 使用作者 0.3 `Firefly_v2ProPlus_20260403_DPO` 原始 GPT / SoVITS 权重及作者参考音频，模型归档 MD5 与另一作者镜像一致，SHA256、引擎固定修订与依赖均已记录。来源、许可边界和复现步骤见 `assets/voice/README.md`、`sources.json`、`generation.json` 与 `scripts/voice-generation-requirements.txt`。
- 16 条固定语音编译进 exe，Windows `waveOut` 串行播放，音量在内存缩放采样，不改系统音量。语音线程不依赖 SAPI、模型服务或网络。旧 `voice_id` 只读取忽略，保存时省略；语音开关、音量、位置、表情和其他设置保留。
- 首轮两条录音出现明显提前结束；使用相同权重、原文和切句方式，调整两条随机种子重新生成。最终全部为 32 kHz / 单声道 / PCM16，时长 1.438–6.083 秒，峰值 -1 dBFS，无满幅采样；最长句内低电平间隔 0.86 秒，没有截断正文。
- 本地 Whisper 转写用于排查整句遗漏；有同音字和 `Codex` 英文识别差异，不作为人工听感通过的证据。完整试听合集位于 `artifacts/FireflyVoice-0.3-20261011-164423/试听合集.wav`，76.58 秒，逐条起点见同目录 `试听目录.md`。最终 WAV 与生成报告及转写报告的哈希一致。

### 已执行检查

以下 Cargo 命令均通过 `scripts/native-cargo.ps1` 配置当前进程 MSVC / Windows SDK 环境：

| 命令 | 观察结果 |
| --- | --- |
| `scripts/native-cargo.ps1 test --lib voice --locked --offline` | 8 项语音专项测试通过；覆盖真实系统规则与 6 类 Codex 事件的音频映射、PCM 格式/边界、音量 0/100%、有界队列满时取消、停止后不补播、输出失败后重试、旧配置读写 |
| `scripts/native-cargo.ps1 test --lib --locked --offline` | 69 通过，2 项既有环境测试按默认忽略 |
| `scripts/native-cargo.ps1 test --locked --offline` | 用户保存并退出旧版后完整复跑通过：库 69、应用 4、hook 进程 1、额度进程 3，共 77 项通过，3 项既有环境测试默认忽略，0 失败。首次执行曾因旧版占用单实例而阻止 hook 夹具，未改测试或绕过保护 |
| `scripts/native-cargo.ps1 test --test usage_process --locked --offline` | 完整测试被单实例保护阻止后，独立执行剩余进程夹具；3 通过，1 项真实账号测试按原设置忽略 |
| `scripts/native-cargo.ps1 -CargoArguments @('clippy','--all-targets','--locked','--offline','--','-D','warnings')` | 通过；首次发现新增代码的 `manual_is_multiple_of` 提示，改用标准方法后复跑通过 |
| `scripts/native-cargo.ps1 -CargoArguments @('fmt','--all','--','--check')` | 通过 |
| `scripts/native-cargo.ps1 build --release --locked --offline` | 通过；exe 30,138,880 字节，SHA256 `ab3fba2df765a965ff2f2be9420defde2339d775cba9ac3ae75daed0a2dc9348` |
| `scripts/native-cargo.ps1 build --example voice_check --example ui_check --locked --offline` | 两个验证程序构建通过 |
| `target/debug/examples/voice_check.exe --all` | 本机实际音频设备依次播放全部 16 条，退出码 0；有 8 条积压时取消当前播放 15 ms，取消后未出现积压播放结果；播放中退出释放资源 18 ms |
| `target/debug/examples/ui_check.exe .runtime/voice-ui-20261011-1648` | 退出码 0，78 张自身渲染截图及现有草稿/交互绑定断言通过；人工查看正常和较小窗口的语音页，固定音色、音量、试听、错误文字和保存栏均完整 |
| `.runtime/voice-generation/venv/Scripts/python.exe .runtime/voice-generation/check_audio.py` | 16 条完成本地转写和波形检查；只作辅助核对，不等于音色、细微漏字及英文发音通过人工验收 |
| `.runtime/voice-generation/venv/Scripts/python.exe .runtime/voice-generation/make_audition.py` | 核对 16 条音频与生成/转写记录哈希并生成合集，退出码 0 |
| Python `ast.parse`、PowerShell `Parser.ParseFile`、`git diff --check` | 生成脚本、打包及包校验脚本语法通过，差异无空白错误；Cargo.lock 与实施前哈希一致 |
| `scripts/package-windows.ps1 -SkipBuild` | 成功生成新的时间戳便携目录、ZIP 和 SHA256；完整包无 `.ckpt`、`.pth`、`.onnx`、Python、FFmpeg 或外置 WAV，ZIP 哈希复核通过 |
| `scripts/verify-package.ps1 -Directory artifacts/FireflyPet-v0.1.0-windows-x64-20261011-165009` | 候选包的清单哈希、必需文件、路由和两个 exe 一致性检查完成；到消息夹具阶段因旧桌宠占用单实例返回“请先退出桌宠，再执行包夹具验证。”，整体退出码 1。不能称完整包验证通过 |
| `scripts/verify-package.ps1 -Directory artifacts/FireflyPet-v0.1.0-windows-x64-20261011-165136` | 用户退出旧版后全部通过；Stop / PermissionRequest / Interrupt 实际送达隔离接收端，耗时 368 / 124 / 124 ms，均退出码 0、输出 `{}` |
| `tests/verify_package.ps1 -Directory artifacts/FireflyPet-v0.1.0-windows-x64-20261011-165136` | 5 组夹具通过：中文空格目录、缺失 exe、两个 exe 不一致、错误市场路径、未投递事件的空操作脚本 |
| `scripts/smoke-desktop.ps1 -Binary artifacts/FireflyPet-v0.1.0-windows-x64-20261011-165136/PetDesktop.exe -SampleSeconds 10` | 包内 exe 真实启动并使用隔离配置，退出码 0、stderr 为空、配置写入成功；10.001 秒样本 CPU 占整机 0.833%，Working Set 338.5 MiB、Private 437.6 MiB。exe 哈希与上列构建一致，不是长期资源或其他设备兼容性结论 |

隔离生成环境的首次运行曾缺少 `matplotlib`、fastText 大模型；补齐上游资源后生成成功。转写工具的 PyAV 19 容器接口不兼容，检查脚本改为直接读取 PCM 并重采样到 16 kHz 后完成检查。这些依赖只用于资源制作，均未进入应用运行包。首次新增系统规则测试遗漏已有 GPU 字段，补齐默认字段后专项及库测试通过。

### 验收边界与项目记忆

原生播放检查验证了真实音频 API 的完成、停止及退出；无设备后的恢复由注入失败的工作线程测试覆盖，未断开用户音频设备。UI 夹具中的无设备文字为显式模拟状态，不能视为真实断开设备的验收。

用户自行保存并退出旧版后，本轮确认其进程结束，完成独占测试、烟测和完整包校验。随后使用 `.runtime/voice-live-20261011-1701` 隔离配置启动同一包内 exe，通过 Computer Use 操作真实设置窗口并读取隔离配置交叉核对：

- 初始旧配置含 `voice_id`、`voice=true`、`volume=70`，主程序正常读取；写回后移除旧音色字段，保留启用状态和音量。
- 拖动音量到 35% 时显示未保存修改，磁盘配置仍为 70%；点击“保存并应用”后配置才变为 35%。执行双击试听未出现错误提示；单次试听可显示“正在按当前草稿音量试听流萤语音…”，随后恢复就绪提示。
- 关闭、重新启用、再次关闭语音并保存，界面与配置保持一致；重启后关闭状态和 35% 音量仍保留。
- 在消息页启用勿扰并保存，配置为 `quiet=true`；试听显示“勿扰期间不播放语音，关闭勿扰后可重新试听。”。解除勿扰并保存后，试听恢复，最终配置为 `voice=false`、`quiet=false`、`volume=35`。
- 日常配置在正常用户上下文中复核 SHA256，与启动测试前一致；测试没有替用户保存或丢弃旧版草稿，也没有写入日常设置。

透明桌宠的工具窗口未被桌面工具列出，用户从桌宠菜单打开普通设置窗口后完成上述操作。第一轮测试到 10 分钟自动退出时间，勿扰保存尚未完成；第二轮重新启动后完成勿扰验证。两次尝试从试听到保存关闭语音的工具调用间隔均超过该条 6.083 秒时长，因此不据此声称真实界面在播放中停止的时延已验收；播放中停止、清空积压及退出释放资源的证据来自上列原生 `voice_check` 与工作线程测试。第二轮操作完成后仅结束已核实路径、参数和 PID 的隔离测试进程，这次清理不计作正常退出验收。真实界面中途停止/退出、物理断开输出设备与最终听感仍需人工试用确认。

本轮读取 README 和本验证记录并对照源码，实施前系统语音说明与旧实现一致。README 已同步离线播放、固定音色、独立取消、旧配置兼容、模块职责和新增文案流程；新增语音资源说明记录准确来源与生成参数。历史记录保留并标注适用版本，没有新建独立项目记忆体系。已有其他未提交 diff 与实施前记录一致，未提交、推送或发布，未修改 Windows 语音组件及日常配置。

## 角色表情设置：2026-10-11

### 实现与证据边界

- 在原生设置窗口新增“角色表情”，按基础待机、待机眨眼、键盘输入、点击、闲置、拖动、消息七个场景配置已有六张表情。基础待机必选，其他场景可“不切换”。页内预览不会改变桌面角色、已应用配置或草稿；场景下拉框修改草稿并预览，保存成功才应用。恢复默认只重置表情草稿。
- 初始值、可调整范围与优先级见 README 的“角色表情设置”。计时和图片选择按场景解耦；保存新的表情配置后清理旧点击/拖动瞬态，保留当前输入与提醒上下文。提醒仍统一跟随气泡，未修改 Codex hooks 协议、气泡时长或语音逻辑。
- 新增可选 `expressions` 配置；缺失字段补默认，使用稳定名称和整数毫秒，非法枚举、负数、越界时间及眨眼持续时间不小于间隔时拒绝保存。默认对象不落盘，恢复默认保存后可供旧版读取；旧版不能直接读取自定义表情字段。保留原有配置原子写入、损坏备份和先保存后生效机制。
- 提取共享角色图层供桌面角色和页内预览使用，原图集和眼部素材未改。没有新增依赖或修改 Cargo.lock，没有安装、提交、推送或修改远程资源。实施前副本和原始差异保存在 `.runtime/expression-baseline-20261011-140452`。

### 已执行验证

以下命令在工程目录运行。涉及命名管道、原生 UI 和打包的命令在正常用户会话中执行；测试与启动配置使用隔离目录。

| 命令 | 观察结果 |
| --- | --- |
| `scripts/native-cargo.ps1 -CargoArguments @('test','--locked','--offline','--lib','expressions')` | 6 项通过，覆盖表情时间边界、优先级、禁用、重复交互、相同图片映射及原有闭眼合成测试 |
| `scripts/native-cargo.ps1 -CargoArguments @('test','--locked','--offline','--bin','PetDesktop')` | 4 项通过，覆盖表情草稿转换、非法草稿及已有设置保留 |
| `scripts/native-cargo.ps1 -CargoArguments @('test','--locked','--offline')` | 最终退出码 0；61 项库测试、4 项主程序测试、1 项 hook 进程测试、3 项额度进程测试，合计 69 项通过；3 项需要真实账号或 GPU 的手动测试保持忽略；文档测试 0 项 |
| `scripts/native-cargo.ps1 -CargoArguments @('test','--locked','--offline','--test','hook_process','--test','usage_process')` | 用户退出旧桌宠后的定向复验通过，后续完整测试也通过 |
| `scripts/native-cargo.ps1 -CargoArguments @('clippy','--all-targets','--locked','--offline','--','-D','warnings')` | 退出码 0 |
| `scripts/native-cargo.ps1 -CargoArguments @('fmt','--all','--','--check')` | 最终退出码 0 |
| `scripts/native-cargo.ps1 -CargoArguments @('build','--release','--locked','--offline','--bin','PetDesktop','--example','ui_check')` | 最终退出码 0；交付 exe SHA-256 为 `4f5501d89665f3022ddd184fa61a260c1bd980786219ba1ec0e9769c2c8775b2` |
| `target/release/examples/ui_check.exe .runtime/expressions-ui-verified-20261011-142703` | 78 张自身渲染截图；六表情预览、860×600 / 680×440 两种布局、滚动末尾、固定保存栏、六模块草稿、预览隔离、恢复默认及原有菜单/四框交互断言通过 |
| `target/release/examples/ui_check.exe .runtime/expressions-gaze-verified-20261011-142741 --gaze` | 324 张自身渲染截图，覆盖三种角色大小、两种朝向、六表情、九注视方向；原有镜像手部和拖动释放断言通过 |
| `python .runtime/validate-gaze-matrix.py .runtime/expressions-gaze-verified-20261011-142741` | 216 组像素比较通过；闭眼不变化，睁眼注视变化限于脸部区域。Python 启动器有路径定位警告，但脚本执行完成、生成 pixel-check.json，退出码 0 |
| `scripts/smoke-desktop.ps1 -SampleSeconds 10` | 最终新版正常启动并自动退出，退出码 0、无 stderr、隔离配置含位置；报告 exe 哈希与交付构建一致 |
| `scripts/package-windows.ps1 -SkipBuild` | 生成时间戳免安装目录、ZIP、SHA-256 文件和依赖许可，无覆盖旧包 |
| `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/verify-package.ps1 -Directory artifacts/FireflyPet-v0.1.0-windows-x64-20261011-142954` | 候选包清单、路由、exe 一致性及 Stop / PermissionRequest / Interrupt 三类消息实际送达隔离接收端均通过，退出码 0。候选包用于记录本节验证结果，最终交付包包含此记录并另做同项校验 |
| `git diff --check` | 通过，无新增空白错误 |

最终可视检查使用 `.runtime/expressions-ui-verified-20261011-142703`，普通与最小窗口均能滚动到消息规则末尾，保存栏始终可见。用 `Get-FileHash` 对照旧 `.runtime/compact-ui-20261011-132432`，38 张角色截图完全相同；对照旧 `.runtime/gaze-ui-final-20261011`，324 张眼部截图完全相同。共享图层提取没有改变这些固定场景的渲染结果；截图及程序化事件属于测试夹具，不等于人工鼠标、键盘或真实多屏 DPI 验收。

### 失败、修复与排除的证据

- 第一次完整测试被正在运行的旧试用包占用单实例保护而中止，错误为“请先退出桌宠再运行 hook 进程测试”。未绕过保护；用户确认退出后，定向与完整测试均通过。
- 首次编译发现共享组件根元素的镜像变换不生效警告，已把变换移至内部角色图层；同时改用 Slint 当前 `content-y` 属性。最终构建无这些警告，旧图像逐字节对比通过。
- 沙箱中的 rustfmt 两次报 `拒绝访问。 (os error 5)`，随后使用正常用户环境完成限定文件格式化；第一次全局格式检查提示示例模块顺序，修正后检查通过。
- 首轮小窗口末尾截图使用了尺寸改变前的滚动范围；验收程序增加一次布局等待，最终截图已显示末尾及保存栏。
- 一次 Release 构建期间提前启动自建 ui_check，造成 `LNK1104` 文件占用。仅结束已核实的本轮检查进程，再等待构建成功后重新验证。该轮旧 exe 启动结果、`expressions-ui-final-20261011-142400` 和中断的眼部截图均排除在最终证据之外。最终启动证据必须匹配上述新 exe 哈希。

### 待用户试用

1. 解压最终交付 ZIP，启动其中 `PetDesktop.exe`，从右键或托盘进入“完整设置 → 角色表情”。
2. 切换六种页内预览，桌面角色应继续原有行为。把点击表情改为其他表情、调整持续时间，保存后点击桌面角色确认效果；拖动与状态小框仍可正常使用。
3. 将输入表情设为“不切换”，保存后键盘和手部同步应继续工作；设置闲置等待时间并观察切换。暂停键盘同步不停止空闲计时，鼠标活动不重置它。
4. 在其他模块保留未保存的大小或音量，再到表情页恢复默认，其他草稿应保持；保存、退出并重启后配置应仍有效。回退旧版前必须先在新版恢复默认表情并保存。
5. 真实物理输入、多屏/DPI、全屏抑制及实际账号环境仍需试用。本轮不重跑真实额度/GPU 查询，先前记录属于历史证据；真实 Stop / PermissionRequest / Interrupt 会话验收继续见下文。本轮未使用 `--codex-test` 作为真实接入证据。

本轮共修改或新增 13 个源码/文档文件：`src/expressions.rs`、`src/app/expression_settings.rs`、`src/app.rs`、`src/config.rs`、`src/lib.rs`、`src/ui_state.rs`（仅增加相关测试）、`ui/character.slint`、`ui/expression-settings.slint`、`ui/settings.slint`、`ui/app.slint`、`examples/ui_check.rs`、`README.md`、`docs/VALIDATION.md`。已读取指定项目记忆 README 及素材说明；README 的五模块描述、固定困倦时长描述与本轮实现不再一致，已依据源码和原生截图更新为六模块、可配置规则、共享图层职责和配置回退要求。未新增记忆系统，未修改全局记忆。

## 四框状态条与 Codex Desktop 识别：2026-10-11

### 实现与证据边界

- 已按用户确认的四框预览实现原生 Slint 状态条：CPU、内存、GPU、Codex 同排，352×32 逻辑像素；点击 Codex 展开各额度窗口、重置时间和刷新入口。默认没有大标题和大按钮，详情关闭不停止状态条的周期刷新，整个状态条关闭仍取消查询。
- Codex 继续使用现有短生命周期 stdio app-server 协议。PATH 中没有 exe 时，自动探测本机已核实的 Desktop `LOCALAPPDATA/OpenAI/Codex/bin/<16 位十六进制目录>/codex.exe` 布局，排除 `.bak`、缺失 exe、相对路径，多个候选按 exe 修改时间排序。没有修改 PATH、Codex 配置或登录状态，没有读取凭据文件。
- GPU 接入既有 5 秒系统采样线程，通过原生 PDH 读取引擎计数器；同一硬件引擎跨进程相加后取最忙引擎，不合计并行引擎，不将未知显示为 0。新增的只是现有 windows 0.62.2 的 Performance feature；Cargo.lock 与实施前哈希相同。
- 实施前源码副本及 Git 差异保存在 `.runtime/compact-status-baseline-20261011-131238`。此前鼠标跟随和 hooks 修复均保留，没有提交、推送或更新远程资源。

GPU API 依据：[PdhGetFormattedCounterArrayW](https://learn.microsoft.com/en-us/windows/win32/api/pdh/nf-pdh-pdhgetformattedcounterarrayw)、[语言无关计数器](https://learn.microsoft.com/en-us/windows/win32/api/pdh/nf-pdh-pdhaddenglishcounterw)、[Microsoft 对最忙引擎口径的说明](https://devblogs.microsoft.com/directx/gpus-in-the-task-manager/)。并不保证桌宠与任务管理器不同采样时刻的读数完全相同。

### 已执行验证

下列 Cargo 命令通过现有 `scripts/native-cargo.ps1` 执行，使用锁定离线依赖；真实设备与账号检查单独显式运行。

| 命令 | 实际结果 |
| --- | --- |
| `scripts/native-cargo.ps1 check --all-targets --locked --offline` | 通过 |
| `scripts/native-cargo.ps1 test --locked --offline` | 最终 54 个库测试、1 个应用测试、1 个 hooks 进程测试、3 个额度进程测试通过，合计 59 项；3 项真实环境测试默认忽略，本轮均另行显式执行通过 |
| `scripts/native-cargo.ps1 -CargoArguments @('test','--lib','real_','--locked','--offline','--','--ignored','--nocapture')` | 两项通过：查找输入不含 Codex PATH 时，Desktop 自动发现及真实只读额度接口返回 1 个额度窗口；真实 GPU 计数器返回有效 0–100% 读数。账号测试只打印窗口数，没有输出账号或原始响应 |
| `scripts/native-cargo.ps1 test --test usage_process --locked --offline` | 3 项进程夹具通过，覆盖协议顺序、失败分类、超时、取消和子进程回收；原有真实账号测试按默认忽略 |
| `target/debug/deps/usage_process-5434131892a90065.exe real_codex_quota_read_only --exact --ignored --nocapture` | 在独立子进程中将 PATH 设为 Windows Machine 与 User PATH 的组合，去掉 Codex 会话临时附加目录；生产 `query()` 自动发现 Desktop 并真实返回 1 个额度窗口。子进程退出码 0，没有修改持久环境变量 |
| `scripts/native-cargo.ps1 -CargoArguments @('clippy','--all-targets','--locked','--offline','--','-D','warnings')` | 最终通过 |
| `scripts/native-cargo.ps1 -CargoArguments @('fmt','--all','--','--check')` | 通过；首次在受限会话格式化遇到 `os error 5`，正常用户环境复跑成功 |
| `scripts/native-cargo.ps1 build --example ui_check --locked --offline` 与 `target/debug/examples/ui_check.exe .runtime/compact-ui-20261011-132432` | 退出码 0，62 张自身渲染缓冲区截图及绑定断言通过。覆盖四框尺寸、100%、未知、失败文字、详情展开收起、长名称、多额度滚动、共享刷新、原有点击和拖动分流 |
| `scripts/native-cargo.ps1 build --release --bins --examples --locked --offline`，最终 `scripts/native-cargo.ps1 build --release --bin PetDesktop --locked --offline` | 通过；最后一次只整理 Slint 缩进与 LF 换行后重新构建交付 exe |
| `scripts/smoke-desktop.ps1` | 最终 Release 在独立数据目录中启动，配置 v1 与位置写入成功，stderr 为空，正常退出码 0；没有读写日常桌宠配置 |
| `scripts/package-windows.ps1 -SkipBuild` | 成功生成时间戳独立交付包，包根和插件 exe 均为下方 Release 哈希，保留旧交付 |
| `scripts/verify-package.ps1 -Directory <本轮交付目录>` | 必需文件、全部哈希、marketplace/hooks 路由、两个 exe 一致性通过；Stop、PermissionRequest、Interrupt 三类消息均被隔离接收端收到，中立退出码 0 |
| `tests/verify_package.ps1 -Directory <本轮交付目录>` | 中文空格路径正常；缺失 exe、两个 exe 不同、错误市场路径和空操作 hooks 四项反例均按预期拒绝，共 5 项通过 |
| `Get-FileHash` 与 ZIP 条目流 SHA-256 核对 | ZIP 校验文件匹配；603 个文件中，602 项内容逐一匹配包内清单，另 1 项为清单本身；没有未登记文件 |
| `git diff --check` | 通过 |

首次全套测试在旧桌宠占用单实例锁时明确失败，未向旧桌宠发送测试 hooks；用户从托盘退出后完整复跑通过。首次原生截图暴露了 100% 文本溢出和展开后状态条布局偏移，已缩小框内间距并固定状态条容器；相同截图和点击断言复跑通过，没有删减失败断言。

最终可视检查使用 `.runtime/compact-ui-20261011-132432`：正常状态、三项 100%、未连接、连接失败、未知、展开错误详情、多额度滚动均无相邻文本覆盖。截图数据为夹具，仅证明原生布局及绑定行为，不是真实账号截图或人工物理输入验收。

最终 Release exe SHA-256：`f40b53d8584425e340383f6f5fda77a73b0286b5cc4b1655cd0cd681b50b2018`。以实施前 Git 差异逐文件比较，12 个非本轮变更文件保持一致；未改动原有配置格式、角色素材、锁文件、hooks 定义或打包脚本。

### 试用步骤与未验证项

1. 退出旧桌宠，解压本轮独立试用包并启动根目录 `PetDesktop.exe`，不要覆盖正在运行的旧目录。
2. 左键点角色，检查 CPU、内存、GPU、Codex 四框同排；GPU 初次采样为未知，等待约 5 秒后检查读数。点击外部、Esc、再次点角色和拖动分别检查关闭行为。
3. 不添加 Codex 专用 PATH，保持本机 Desktop 的正常登录环境。点击 Codex 小框检查额度详情与刷新；如果显示“未登录”或其他状态，记录具体文字，不将它与 hooks 接入状态混同。
4. 对照同一账号、工作区和时刻的 Codex 界面核对额度；测试面板收起后不再周期查询、重新打开可刷新。实际账号与 Desktop 界面是否完全一致、多 GPU 设备、不同驱动、真实多屏/DPI 和人工物理点击仍待用户试用。
5. 本轮没有运行 `--codex-test`。真实 Stop / PermissionRequest / Interrupt 会话投递仍按下一节步骤单独验收；59 项自动测试、真实额度读取和 GPU 采样均不能替代这三项证据。

已同步指定项目记忆 README 的尺寸、交互、查找顺序、GPU 指标口径、错误语义与模块职责；旧说明中的 PATH 唯一前提和 320×420 面板描述已更新，以下旧轮次记录保留作历史证据。

本轮源码变更清单：`Cargo.toml`（启用现有 Windows Performance 功能）、`src/platform/gpu.rs`（新增原生采样及聚合测试）、`src/platform/mod.rs`、`src/platform/system.rs`、`src/notices.rs`（采样接入与字段）、`src/platform/codex_usage.rs`（Desktop 自动发现与验证）、`src/usage.rs`（紧凑额度摘要和错误提示）、`src/app.rs`、`src/app/usage_panel.rs`（GPU 与四框绑定）、`ui/status-panel.slint`（四框与详情布局）、`examples/ui_check.rs`（布局/交互验证）、`README.md`、`docs/VALIDATION.md`（项目记忆和验收记录）。本轮没有修改其余已有变更文件。

## Codex 本地插件接入审计与验收：2026-10-11

**当前结论：范围内的桥接修复、包命令投递测试和隔离 Codex 加载检查通过；真实 Codex 会话的 Stop / PermissionRequest / Interrupt 三项均未验收。** 本节没有运行 `--codex-test`，也不把手工输入 hook JSON、加载清单、管道测试或历史模拟截图当作真实会话证据。

### 配置与调用链核对

- 检查现有历史包 `artifacts/FireflyPet-v0.1.0-windows-x64-20261010-233331`：marketplace、plugin manifest、hooks.json、notify.cmd 与源码模板的 SHA-256 分别相同。marketplace 根为包内 `codex-marketplace`，目录引用相对于该根；manifest 显式引用 `./hooks/hooks.json`，桥接 exe 位于插件根目录。没有发现路径、事件名称或异步超时配置错误，因此保留模板。
- 本机 CLI 为 `codex-cli 0.162.0-alpha.17.2`。`codex plugin marketplace list --json` 的日常 CLI 环境没有 `firefly-local`，缓存目录也没有该市场。这里只核实 CLI 环境，未读取或修改凭据；桌面应用是否使用同一个配置环境仍须在实际客户端确认。
- 在 `.runtime/codex-acceptance-20261011-114415/isolated-codex` 中安装历史包，调用实际 `codex app-server` 的 `initialize`、`hooks/list`。三项均为 `source=plugin`、`enabled=true`、`async=true`、`timeoutSec=2`，`errors` 和 `warnings` 为空；`${PLUGIN_ROOT}` 被解析为该隔离缓存的绝对路径。三项均为 **`trustStatus=untrusted`**，没有自动信任、绕过信任、创建模型会话或调用账号接口。加载结果保存在同一验收目录的 `loader-result.json`，只含本地插件元数据。
- 换到新时间戳候选包时，CLI 实际返回 `marketplace 'firefly-local' is already added from a different source; remove it before adding this source`。在上述隔离环境中执行 `codex plugin marketplace remove firefly-local` 后重新登记新来源、安装插件成功；缓存 exe 的 SHA-256 已变为本轮新程序哈希，三类 hooks 加载仍无错误且均未信任。新结果另存为 `loader-result-FireflyPet-v0.1.0-windows-x64-20261011-115229.json`，旧证据未覆盖。
- 输入必须具备事件名、`session_id`、`turn_id`；当前官方文档对三类事件均定义 `turn_id`，没有证据支持放宽成随机或空标识。解析保留 1 MiB 限制及正文剥离，管道仍使用 v1 五字段、2048 字节上限和当前用户 DACL。

官方依据：[插件打包与本地市场](https://developers.openai.com/plugins/build/plugins)、[hooks 事件、异步运行和信任规则](https://learn.chatgpt.com/docs/hooks)。按本机实际加载结果确认占位符和 schema 兼容；官方说明不能替代当前客户端的运行验收。

### 已修复问题及证据

| 问题 | 修复与验证 |
| --- | --- |
| `WaitNamedPipeW(150)` 只等一次，随后 `CreateFileW` 也只尝试一次；管道短时占用或并发争用会丢事件 | 先尝试连接，只对暂忙/尚未创建做总计 500 ms 内的重试，写入仍有 200 ms 等待且不重复写。旧实现遇到 220 ms 占用实际失败，报 `0x80070079`；修复后同一用例及 8 路并发均通过 |
| 同一轮的多个审批共用去重 key，5 分钟内后续审批不显示 | 每次接收 `PermissionRequest` 生成内存序号，保留 Stop / Interrupt 的同轮去重；单元测试实际经过提醒队列，两个审批均可取出。未扩展输入字段或管道协议 |
| 包校验仅验证 `{}` / exit 0，缺少 exe 或空操作也会通过 | 校验必需文件及哈希覆盖、市场路由、manifest、三类事件及双 exe 一致性，再启动独立接收端检查实际消息。旧脚本在缺少桥接 exe 的夹具上仍三项成功，新脚本正确拒绝；返回 `{}` 却不投递的脚本同样被拒绝 |
| 进程测试手工输入 JSON，却可误投递到正在运行的用户桌宠 | 测试和包校验占用桌宠单实例锁，只接入自己创建的接收端；已有桌宠时明确停止验证。生产 hook 始终中立退出的行为保留 |

### 本轮执行记录

工作目录均为 `F:\Program\pet_desktop`。命名管道与 Rust 验证在正常 Windows 用户环境执行；没有安装或升级依赖。

| 命令 | 实际结果 |
| --- | --- |
| `scripts/native-cargo.ps1 -CargoArguments @('test','--lib','platform::ipc::tests::local_pipe_transports_and_reconnects','--locked','--offline','--','--exact')` | 先复现旧发送逻辑失败，再用同一用例验证修复通过；覆盖空闲客户端回收、畸形/超长消息、重连、8 路并发及停止 |
| `scripts/native-cargo.ps1 test --locked --offline` | 51 个库测试、1 个应用绑定测试、1 个 hooks 进程测试、3 个额度进程测试通过，合计 56 项；原有真实账号测试继续忽略，未查询账号 |
| `scripts/native-cargo.ps1 -CargoArguments @('clippy','--all-targets','--locked','--offline','--','-D','warnings')` | 通过 |
| `rustfmt --edition 2024 src/bridge.rs src/platform/ipc.rs tests/hook_process.rs` | 仅格式化本轮三个 Rust 变更文件 |
| `scripts/native-cargo.ps1 -CargoArguments @('fmt','--all','--','--check')` | 最终通过 |
| `scripts/native-cargo.ps1 build --release --locked --offline` | 通过；exe SHA-256 为 `c3d0562625f6699f7697a2687b79343ae4d25b4aed1ca7da3d7208dd1155b5d5` |
| `scripts/verify-package.ps1 -Directory artifacts/FireflyPet-v0.1.0-windows-x64-20261010-233331` | 增强脚本验证历史包成功，三类消息在隔离接收端实际收到，exit 0 / `{}`；不是 UI 或会话证据 |
| `tests/verify_package.ps1 -Directory artifacts/FireflyPet-v0.1.0-windows-x64-20261010-233331` | 中文空格目录通过；缺少 exe、exe 不一致、错误市场路径、无投递脚本四个反例均按预期拒绝 |
| `python .runtime/codex-acceptance-20261011-114415/inspect_loader.py artifacts/FireflyPet-v0.1.0-windows-x64-20261010-233331` | 隔离安装及实际 app-server 加载通过；三项未信任，未触发会话 |
| PowerShell `Parser.ParseFile` 与 `git diff --check` | 脚本语法及差异空白检查通过 |
| `scripts/smoke-desktop.ps1 -SampleSeconds 2` | 候选桌宠退出码 0，stderr 为空，隔离配置创建、启动及自动退出通过；没有触发 hooks，不能据此证明 UI 收到事件 |
| `scripts/package-windows.ps1 -SkipBuild` | 生成初验包 `FireflyPet-v0.1.0-windows-x64-20261011-115229`，程序与插件副本使用本轮同一 exe，历史交付包保持原样 |
| `scripts/verify-package.ps1 -Directory artifacts/FireflyPet-v0.1.0-windows-x64-20261011-115229` | 新包哈希和三类消息实际投递通过，分别为 358 / 127 / 119 ms，均 exit 0 / `{}` |
| `tests/verify_package.ps1 -Directory artifacts/FireflyPet-v0.1.0-windows-x64-20261011-115229` | 新候选包的正常路径及四个故障反例均通过 |
| `powershell.exe -NoProfile -File tests/verify_package.ps1 -Directory artifacts/FireflyPet-v0.1.0-windows-x64-20261011-115229` | Windows PowerShell 5.1 复核通过，中文 UTF-8 BOM 脚本和异步管道调用兼容 |
| `python .runtime/codex-acceptance-20261011-114415/inspect_loader.py artifacts/FireflyPet-v0.1.0-windows-x64-20261011-115229 --replace-isolated-source` | 仅更换本轮隔离市场，安装成功；缓存 exe 哈希与候选一致，三项 hooks 无加载错误、均未信任 |

开发中间失败均已区分：首次在受限环境运行隔离加载时返回 `failed to canonicalize CODEX_HOME ... os error 5`，正常用户环境复跑通过；首次直接把 `--exact` 传入包装脚本被 PowerShell 参数转发规则影响，改用上述 `-CargoArguments` 数组；测试初稿跨线程移动 Win32 handle 导致 `E0277`，改为创建线程持有并释放句柄；首次格式检查指出本轮文件排版问题，限定文件格式化后复核通过。这些不作为产品测试通过的证据。本记录补充后重新生成最终时间戳候选包，exe 和插件模板与初验包相同，最终 ZIP 文件名及哈希以交付清单为准。

### 真实会话验收步骤

1. 使用同一候选包中的桌宠和插件。退出旧桌宠后启动包根目录 `PetDesktop.exe`。设置中开启“接收 Codex 事件”，关闭勿扰，避免全屏/演示状态；语音可关闭。确保其他提醒已经播完。当前提醒队列最多 12 条、有效期 45 秒、每条展示 7 秒，不能把无气泡直接归因于 hook 未送达。
2. 在实际使用的本地 Codex 环境中从该包运行 README 的市场登记与安装命令。用 `codex plugin marketplace list --json` 核对市场根；用 `codex plugin add firefly-pet@firefly-local --json` 返回的 `installedPath` 查缓存 exe，分别对缓存 exe 和包根 exe 执行 `Get-FileHash -Algorithm SHA256 -LiteralPath <exe绝对路径>`，应与上表相同。若同名市场指向旧包，先记下旧来源路径，仅执行 `codex plugin marketplace remove firefly-local` 后从新包重跑登记和安装，不要删除整个插件缓存或其他配置。以安装结果的实际路径为准，不假定缓存目录名一定是 `local`。
3. 重启实际客户端并打开一个**本地编排且本地执行**的普通主会话，在 hooks 管理页（CLI 为 `/hooks`）核对三类事件、解析后的命令、启用状态和受信任状态。由用户审阅并信任当前定义；不使用跳过 hook 信任的启动参数，不修改审批/沙箱策略。CLI 安装环境不同、插件未启用或 hooks 未信任时先记录阻塞，不继续声称接通。
4. 按下表逐项触发，记录客户端版本、包/缓存 exe 哈希、当地时间与时区、会话及轮次标识、Codex hook 运行状态、桌宠设置页状态和气泡。只保存这些验收元数据及裁剪后的相关界面，不保存提示词、工具参数、回复正文、transcript 或凭据。每一项都要同时具备 **Codex 触发/运行证据和桌宠接收/显示证据**。
5. 退出桌宠，再触发结束、一个无害审批和主动中断，确认会话/审批不被桌宠离线阻塞。重新启动桌宠后触发新事件，确认恢复接收；不会补发离线时已丢弃的事件。

| 事件 | 真实触发方式 | 通过条件 | 当前状态 |
| --- | --- | --- | --- |
| Stop | 新会话发送“只回复：桌宠 Stop 验收”，等正常回复结束 | Codex 的 Stop hook 成功，桌宠接收状态为 Stop，气泡为“Codex 这一轮回复结束了。”；不据此宣称任务成功 | 未验证 |
| PermissionRequest | 发起需要正常审批的无害操作，例如请求通过审批执行仅输出 `FIREFLY_PERMISSION_ACCEPTANCE` 的 `Write-Output` 命令；没有审批路径的普通工具调用不算 | Codex 确认实际发起 PermissionRequest；桌宠提示审批请求；审批仍由正常机制决定。另在同一轮产生第二次独立审批并等待前条气泡结束，确认第二次也能提示 | 未验证 |
| Interrupt | 请求执行 `Start-Sleep -Seconds 20` 等无害等待，在主会话运行中点击停止；保持会话窗口开启 | Codex 确认 Interrupt hook 运行，桌宠显示“这一轮已中断。”；空闲时点停止、关闭窗口或子代理中断不算本项证据 | 未验证 |
| 离线与恢复 | 按步骤 5 关闭/重启桌宠并重复触发 | 无桌宠时会话正常，重启后新事件可达，无历史事件补播 | 未验证 |

审批被自动处理不意味着没有 PermissionRequest；反之，仅看到了审批 UI 也不能代替 hook 运行记录。异步 hooks 的完成顺序可能不同，会话直接结束会取消未完成后台 hook，真实验收时要留出处理时间。不要仅以任何 `simulated=false` 消息认定来源真实：同用户进程也能手工发消息，协议不具备 Codex 进程身份认证。

### 故障定位与尚未验证的边界

按“实际客户端发现市场 → 安装缓存哈希 → hooks 加载/信任 → 事件确实触发 → hook 运行/超时 → 管道 → 设置接收状态 → 气泡”定位。设置页已经显示接收事件而气泡没有显示时，检查接收开关、勿扰/全屏抑制、队列过期及已有气泡；这些条件不等于桥接失败。输入无效、exe 缺失和通道失败仍可中立退出，所以 exit 0 不是投递证明。

连接总期限、有限队列、桌宠离线和系统负载下仍是尽力通知，没有持久化、重放或应用层确认。尚未验证实际桌面客户端的信任界面、真实三类 hooks、气泡/语音和客户端更新后的兼容性。没有为了验证自动信任 hooks、替用户审批或改写日常 Codex 配置。

实施前读取指定项目记忆 README、既有验证记录及桥接/队列调用路径，保留鼠标跟随等已有未提交工作。原 README 的 150 ms 单次连接描述与旧代码一致；本轮同步为有界重试，并记录审批去重和测试隔离规则。历史记录本就注明真实接入未完成，未发现把模拟误标为真实的记忆冲突。没有新建独立项目记忆或修改全局记忆，模板和依赖保持原状。

## 鼠标转向与瞳孔追踪：2026-10-11

按确认方案实现角色、双手、键盘整体镜像和四种睁眼表情的独立瞳孔追踪。默认常开，不增加设置字段；暂停键盘同步、勿扰和穿透不参与跟随控制。复用现有 16 ms UI 定时器，每至少 32 ms 采样全屏鼠标坐标；固定中心线、左右各 16 逻辑像素缓冲区及 100 ms 持续判定用于转向，拖动期间锁定。坐标换算包含窗口物理位置、系统 DPI、角色缩放、顶部留白与呼吸偏移，失败时保持朝向、瞳孔平滑回中。

新增 `src/gaze.rs` 与 `src/gaze_render.rs`，分离跟随状态与眼部合成。通过内置 imagegen 新增 `assets/firefly/gaze.png`，原始 `atlas.png` 保持原字节；生成提示和校准规则记录于资源说明。眼白替换旧虹膜，活动虹膜受每种表情的多边形遮罩限制；遮罩外像素以及闭眼表情保持原样。素材只启动时解码，遮罩预计算，缓存最后一帧，未增加依赖、线程或鼠标钩子。

### 自动检查与原生渲染证据

以下命令均在工程目录执行：

| 命令 | 实际结果 |
| --- | --- |
| `scripts/native-cargo.ps1 test --lib --locked --offline`（初次受限会话） | 48 项通过，现有命名管道测试失败：`0x80070002`；受限会话无法发现测试自身的管道，随后正常用户环境复跑通过 |
| `scripts/native-cargo.ps1 test --locked --offline`（最终正常用户环境） | 50 项库测试、1 项应用绑定测试、1 项 hooks 进程测试、3 项额度进程测试通过，合计 55 项；真实账号测试保留默认忽略，本轮未查询真实账号 |
| `scripts/native-cargo.ps1 -CargoArguments @('clippy','--all-targets','--locked','--offline','--','-D','warnings')` | 最终通过；首次受限会话查找配置返回 `os error 5`，正常用户环境发现一处新增数组引用的 `into_iter` 提示，改为 `iter` 后复跑通过 |
| `scripts/native-cargo.ps1 -CargoArguments @('fmt','--all','--','--check')` | 正常用户环境通过；受限会话的 rustfmt 曾返回 `os error 5`，没有绕过检查 |
| `rustfmt --edition 2024 --check tests/support/usage_fixture.rs` | 通过 |
| `scripts/native-cargo.ps1 build --release --bins --examples --locked --offline` | 最终正式程序与两个原生示例构建通过 |
| `target/debug/examples/ui_check.exe .runtime/gaze-ui-final-20261011 --gaze` | 最终边界修正后退出码 0；保存 324 张自身渲染缓冲区截图，覆盖 3 个大小、2 个朝向、6 个表情及 9 个注视方向；左右手映射和拖动释放断言通过 |
| `python .runtime/validate-gaze-matrix.py .runtime/gaze-ui-final-20261011` | 216 组像素比较通过：闭眼表情不变，睁眼注视变化限制在脸部区域；代码单元测试进一步验证变化不超出眼睛遮罩 |
| `target/release/examples/ui_check.exe .runtime/gaze-ui-regression-20261011` | 退出码 0；保存 58 张原生回归截图，模块草稿、关于页保存按钮、右键分流、Esc、左键开关、拖动关闭及取消查询断言通过 |
| `scripts/smoke-desktop.ps1`（边界修正前单独运行） | 退出码 0，stderr 为空，隔离配置 v1 与位置写入通过，自动退出通过；目录 `.runtime/smoke-20261011-110735` |
| `scripts/smoke-desktop.ps1 -SampleSeconds 2`（边界修正前第二次独立复测） | 退出码 0，隔离配置创建与自动退出通过；目录 `.runtime/smoke-20261011-110841`，与前一行使用同一版本程序 |
| `scripts/smoke-desktop.ps1`（最终边界修正后单独运行） | 退出码 0，stderr 为空，隔离配置 v1 与位置写入、自动退出均通过；目录 `.runtime/smoke-20261011-111522`，对应本节末最终 SHA-256 |
| `git diff --check` | 通过 |

新增 10 项单元测试覆盖朝向持续判定、中心防抖、拖动结束、光标失效与恢复、镜像方向、负坐标/DPI/缩放换算、资产无效几何、闭眼保持、遮罩范围、合成缓存及量化后的偏移上限。复核时发现四分之一源像素取整可能超出声明上限约 0.02 逻辑像素，已在量化时限制整数格数，并以所有睁眼表情的正负极值和越界输入验证修正。原生截图为自动注入展示状态；人工查看浅/深底色、大小、左右朝向与各睁眼表情样例，未见明显双重瞳孔、眼睑越界或文字反向。截图未采集桌面或其他窗口。

### 启动复测与已知限制

首次 `scripts/smoke-desktop.ps1` 与增量 Release 构建重叠，使用前一轮构建的程序，出现“退出超时”；脚本按原逻辑仅回收自己启动的进程，资料目录为 `.runtime/smoke-20261011-110452`。随后确认构建完成及进程退出，保持原有超时，两次独立复测均通过；量化边界修正后的最终程序又单独复测通过。首次超时根因未确认，不能仅凭复跑成功认定该问题已被修复，也未将其归因为已证实的构建争用。

最终程序成功的 10 秒独立样本中，24 个逻辑处理器下程序占整机 CPU 约 0.456%，Working Set 304.6 MiB，Private Memory 393.1 MiB。这是短时启动样本，不代表长期内存、连续鼠标移动性能或性能提升。

本轮尚未完成物理鼠标跨屏、实际 Windows 100%/150%/200% DPI、真实拖动/穿透/前台焦点组合及长时稳定性验收。坐标单元测试、模拟 UI 事件和自身渲染截图不能替代这些设备操作。暂停、勿扰与跟随的独立性已从生产调用路径确认；未将静态检查表述成物理操作验收。

### 项目记忆与范围复核

实施前读取 README（指定项目记忆）、角色资源说明和本验证记录，保留已有 `.gitignore`、README、资源说明及验证记录的未提交清理改动。README 同步跟随规则、模块职责、预览方式和验证边界；资源说明中残留的“常驻整机指标条”与当前 UI 不符，经 `ui/app.slint` 和点击面板实现核对后更正。没有新建独立项目记忆系统或修改全局记忆。

原图集、`Cargo.toml`、`Cargo.lock` 的 SHA-256 与实施前基线一致。配置仍为 v1，未修改日常用户配置或历史交付包；测试与构建缓存按本轮验证需要重新生成。本轮没有 Git 提交、推送、安装、打包或发布。

最终本地程序：`target/release/PetDesktop.exe`；SHA-256：`0872ea0af1a25eae2278d8c694d4fd3cba57a602e7d6aab55bb882000460108e`。

## 本地目录清理：2026-10-11

按用户确认的清单清理产物并保留最近版本，没有重排源码目录。删除前工作区干净，58 个 Git 跟踪文件完整，未发现运行中的桌宠、预览或 Rust 构建进程。逐项核对绝对路径与工作区边界，检查重解析点并比对快照；仅删除清单文件，随后删除空目录，没有使用整个项目的递归清理命令，也没有停止用户进程。

共删除 18,346 个文件、3,592 个空目录，70 个删除目标均已不存在。删除文件的逻辑大小合计 14,925,130,215 字节，含编译产物硬链接的重复计数；删除前后观察到磁盘可用空间增加 13,380,468,736 字节，约 12.46 GiB。可用空间变化是本机采样值，可能受同盘其他活动影响。

清理后的本地保留项：

- `target/release/PetDesktop.exe`：当前源码构建，SHA-256 为 `261f4c29da79903e161e54667eba66373824a789aea8c20a086fc99c01a15ccf`；`target/` 中其他文件均已清理。
- `artifacts/FireflyPet-v0.1.0-windows-x64-20261010-233331` 及同名 `.zip`、`.zip.sha256`：最近一套完整交付包，早于当前 exe，保持原样。
- `.runtime/quota-ui-20261011-final`：当前版本完整界面验证截图。
- `.runtime/quota-panel-baseline-20261011-092933`：最近一次功能修改前的源码基线。
- `.runtime/smoke-20261011-095846`：最近一次启动验证资料。
- `.runtime/position-check-20261010-233138` 与 `.runtime/verify-position.ps1`：已通过的位置验证资料和现有验证脚本。
- `.runtime/git-source-baseline-20261011-100951.json`：首次入库前的文件哈希清单。

本轮另在 `.runtime/cleanup-20261011-102807` 保存路径、大小、哈希清单及清理验证结果，不保存文件正文。旧桌面截图 `docs/native-preview.jpg` 已删除，Git 忽略规则继续保留；全部 Git 跟踪图片仍在。下方历史记录中的旧交付、旧基线、截图矩阵及示例 exe 路径均表示当时的取证位置；除上述保留项与 Git 跟踪文件外，相关本地产物已清理，历史测试结论本身不作改写。

清理完成后、更新文档前，761 个保留文件的 SHA-256 全部与清理前一致，覆盖全部 58 个跟踪文件、当前程序、交付包和指定临时资料。交付包内 601 项文件与 `SHA256SUMS.txt` 一致，ZIP 与随附 `.sha256` 一致。以下格式检查通过：

```powershell
.\scripts\native-cargo.ps1 -CargoArguments @('fmt','--all','--','--check')
rustfmt --edition 2024 --check tests/support/usage_fixture.rs
```

本轮只更新 README、角色资源说明、本验证记录和 `.gitignore` 中的截图注释；业务源码、资源、依赖清单和锁文件未改动。未重新构建、运行应用、查询真实账号或重跑单元测试，以免重建刚清理的产物；程序与交付包的保留完整性通过哈希验证。后续测试、构建和预览需要重新生成缓存与示例程序。未修改全局依赖缓存、真实用户配置、Git 历史，也未提交或推送。

## 源码入库前复核：2026-10-11

按用户要求初始化本地 `main`，将 `origin` 配置为 `https://github.com/lkuliuying/agent-pet.git`；配置前通过 GitHub 仓库元数据和 `git ls-remote --symref` 确认远程为空、默认分支为 `main`。本地仅设置 `core.autocrlf=false` 保留源码换行，不修改全局 Git 配置，沿用已有提交身份。

提交范围为原生程序与角色资源、测试与预览示例、Codex 本地插件模板与交付脚本、项目文档和模拟验证图片，共 58 个文件。`target/`、`artifacts/`、`.runtime/` 与插件目录的 exe 继续忽略；`docs/native-preview.jpg` 含后方窗口内容，入库时增加精确忽略规则并保留本地原图，随后已按本页首节清理。对提交候选的凭据模式扫描未发现匹配，4 个 JSON 文件解析和 4 个 PowerShell 脚本语法检查通过；这些检查不等同于完整安全审计。

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
- 明确带模拟标记的 `PetDesktop.exe --codex-test` 经真实管道进入 UI，显示“联动演示”和“Codex 这一轮回复结束了。”，角色同步出现提醒表情。原始窗口截图 `native-preview.jpg` 背后可见其他窗口，属于当时的透明背景证据；该截图未纳入源码仓库，已在 2026-10-11 清理。
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
