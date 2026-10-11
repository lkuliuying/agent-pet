# 内置流萤语音

本目录用于作者效果预览 **0.3 模型_v2ProPlus_20260403_DPO** 生成的固定播报。WAV 以单声道、16 位 PCM 编译进 exe；运行不需要模型、Python、网络或系统语音组件。音量只缩放当前音频的采样值，不修改 Windows 系统音量。

## 来源与版本

- 作者发布页：[流萤厨的小窝 · 语音合成](https://lovefirefly.moe/post_voicegen/)，具体使用其中的 0.3 版本。
- 模型及参考音频：[作者公开仓库](https://lovefirefly.moe/repo/)，路径为语音合成/模型；实际模型归档名为 `Firefly_v2ProPlus_20260403_DPO.zip`。
- 模型归档 SHA256：`4349b34b8e9ec86fba276f5467cb7f2072761408c72da1becd8d5faf04f8f4c1`。MD5 `380a3882a626bc76aa0412d513da9b66` 已与作者另一公开镜像一致性核对，并通过 ZIP CRC 检查。
- 参考语音归档 `参考语音.7z` SHA256：`b8e4e9d34b5db8f6df9fc3ec2ab78245b8903d6af27ac3731264982eb4218308`。
- 生成引擎：[RVC-Boss/GPT-SoVITS](https://github.com/RVC-Boss/GPT-SoVITS/tree/1b47ca07ef43a284e0b8bd7f29d3e1e3ff76082a)，固定修订 `1b47ca07ef43a284e0b8bd7f29d3e1e3ff76082a`，引擎 MIT 许可。使用原始 GPT/SoVITS 权重，未改用量化的其他模型。
- 参考音频为 `chapter3_3_firefly_212.wav`，提示文本为“我也一样。现实里的我，有着求而不得的愿望。它太过强烈，因此我诉诸梦境。”

角色及原始语音来自《崩坏：星穹铁道》，社区模型由作者分享。作者页面要求不用于非法用途，未单独列出模型与生成音频的商业授权。本说明记录素材来源，不代表获得额外授权。当前交付为本地试用，未发布模型或程序。

## 内容与生成

`lines.json` 是 18 条播报清单，包括 9 条系统提醒、4 条 Codex 提醒、3 条带“这是一条演示提醒。”前缀的 Codex 提醒、试听和通用错误提示。具体错误原因仍在文字提示中显示，不在运行时合成语音。

按作者 0.3 说明使用中文句号切分 `cut3`，其余推理参数以引擎默认值为基准。实际权重哈希、依赖版本、设备、参数、时长及每条 WAV 的 SHA256 在 `generation.json`，基础权重和工具来源在 `sources.json`。最终音频峰值统一为 -1 dBFS；仅收拢首尾静音并保留余量，正文不截断。超过 7 秒的内容必须调整该条生成参数后重新合成。

生成环境只放在项目忽略目录 `.runtime/voice-generation`。本次使用 Python 3.12.13、CPU float32；完整依赖版本见 `scripts/voice-generation-requirements.txt`，只安装到独立虚拟环境，不是桌宠运行依赖。先按上游文档准备上述引擎、原始权重、中文 RoBERTa/HuBERT、G2PW、说话人编码器、fastText `lid.176.bin`、NLTK 数据和 FFmpeg，再从项目根目录运行：

```powershell
.runtime\voice-generation\venv\Scripts\python.exe scripts\generate-voice.py `
  --engine .runtime\voice-generation\GPT-SoVITS-1b47ca07ef43a284e0b8bd7f29d3e1e3ff76082a `
  --gpt .runtime\voice-generation\model\GPT_weights_v2ProPlus\Firefly_v2ProPlus_20260403_DPO-e18.ckpt `
  --sovits .runtime\voice-generation\model\SoVITS_weights_v2ProPlus\Firefly_v2ProPlus_20260403_DPO_e30_s1590.pth `
  --reference .runtime\voice-generation\reference\参考语音\chapter3_3_firefly_212.wav
```

脚本本身不下载模型、不启动网络服务，缺少文件时明确报错。`--only preview` 可重做单条，使用逗号分隔可指定多条；`--device cuda` 仅适用于已准备匹配 CUDA 依赖的生成环境。固定随机种子用于追溯，不保证不同设备或依赖版本逐字节相同。

首轮 `network-on` 和 `demo-codex-stop` 曾提前结束，已保持原文、同一权重并改用种子 `20260404` 重新生成，参数保存在清单中。此前 16 条为 32 kHz / 单声道 / PCM16，时长 1.438–6.083 秒。逐条本地 Whisper 转写已排查明显整句遗漏，仍出现“流萤/刘莹”等同音识别差异和 `Codex` 英文识别误差；转写不能替代人工听感验收。全部录音的最长句内低电平间隔不超过 0.86 秒，最终峰值为 -1 dBFS，没有满幅采样。实际听感、英文发音及细微漏字仍需试听确认。

新增文案时应同时更新 `lines.json`、`src/voice.rs` 的音频标识和通知映射，生成音频，核对读音、漏字及停顿，执行资源测试并重新构建。模型文件和生成环境不能复制到便携包；包内仅附本说明、清单与溯源记录，音频从 exe 内部读取。

2026-10-11 本轮新增 GPU 高占用、Codex 等待回答两条录音，并把权限提醒及其模拟录音改为“发起了权限请求”。沿用相同流萤权重和离线生成环境，资源参数与哈希同步到 generation.json。新增录音的 PCM、峰值及长度由资源测试检查；本轮新增录音尚需人工试听验收。
