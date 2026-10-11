"""使用已准备的 0.3 模型生成内置语音；本脚本不下载模型，也不启动网络服务。"""

import argparse
import hashlib
import importlib.metadata
import json
import os
from pathlib import Path
import re
import sys
import wave
import zipfile

ENGINE_REVISION = "1b47ca07ef43a284e0b8bd7f29d3e1e3ff76082a"
ENGINE_ARCHIVE_SHA256 = "713c9df5fa2c7b0a2fee96fa8954375c915d64229bb1fda10de37f588479e16c"
MODEL_NAME = "Firefly_v2ProPlus_20260403_DPO"
GPT_SHA256 = "acb3645ae21e31a53786beca1bec058881081aee2b4e8701b2435b15455ebb9b"
SOVITS_SHA256 = "ed7f718b0dc06671258fae058f1c01bf53567019e57c71474ae54cc65600ba28"
REFERENCE_SHA256 = "58918f42bdaac7cfc25a71644333120c4fdd8aa7dba50b5596d7a9e27f056a5f"
PROMPT = "我也一样。现实里的我，有着求而不得的愿望。它太过强烈，因此我诉诸梦境。"


def sha256(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine", type=Path, required=True)
    parser.add_argument("--engine-archive", type=Path, help="固定引擎源码归档，默认取引擎上级目录的 engine-source.zip")
    parser.add_argument("--gpt", type=Path, required=True)
    parser.add_argument("--sovits", type=Path, required=True)
    parser.add_argument("--reference", type=Path, required=True)
    parser.add_argument("--device", choices=("cpu", "cuda"), default="cpu")
    parser.add_argument("--only", help="仅重新生成指定音频，使用逗号分隔标识")
    parser.add_argument("--output", type=Path, default=Path(__file__).resolve().parents[1] / "assets/voice")
    args = parser.parse_args()
    engine, gpt, sovits, reference, output = (
        path.resolve(strict=True) for path in (args.engine, args.gpt, args.sovits, args.reference, args.output)
    )
    if MODEL_NAME not in gpt.name or MODEL_NAME not in sovits.name:
        raise ValueError("必须明确使用选定的 0.3 DPO 模型，不能回退到其他权重。")
    for path, expected in [(gpt, GPT_SHA256), (sovits, SOVITS_SHA256), (reference, REFERENCE_SHA256)]:
        if sha256(path) != expected:
            raise ValueError(f"选定模型或参考音频的 SHA256 不匹配：{path.name}")
    archive_path = args.engine_archive or engine.parent / "engine-source.zip"
    if sha256(archive_path) != ENGINE_ARCHIVE_SHA256:
        raise ValueError("引擎源码归档与锁定修订不一致。")
    with zipfile.ZipFile(archive_path) as archive:
        for name in archive.namelist():
            if name.endswith(".py"):
                relative = Path(*Path(name).parts[1:])
                if (engine / relative).read_bytes() != archive.read(name):
                    raise ValueError(f"引擎 Python 源码已修改：{relative}")
    lines = json.loads((output / "lines.json").read_text(encoding="utf-8"))
    ids = [line["id"] for line in lines]
    if len(ids) != len(set(ids)) or any(not re.fullmatch(r"[a-z0-9-]+", item) for item in ids):
        raise ValueError("音频标识重复或包含非法路径字符。")
    selected = set(args.only.split(",")) if args.only else set(ids)
    if not selected <= set(ids):
        raise ValueError("指定了不存在的音频标识。")
    for required in (
        "GPT_SoVITS/pretrained_models/fast_langdetect/lid.176.bin",
        "GPT_SoVITS/pretrained_models/sv/pretrained_eres2netv2w24s4ep4.ckpt",
        "GPT_SoVITS/text/G2PWModel/g2pW.onnx",
        "ffmpeg.exe",
        "nltk_data/corpora/cmudict/cmudict",
        "nltk_data/taggers/averaged_perceptron_tagger/averaged_perceptron_tagger.pickle",
        "nltk_data/taggers/averaged_perceptron_tagger_eng/averaged_perceptron_tagger_eng.weights.json",
    ):
        if not (engine / required).is_file():
            raise FileNotFoundError(f"请先准备引擎资源，生成过程不会自动下载：{required}")

    os.chdir(engine)
    sys.path[:0] = [str(engine), str(engine / "GPT_SoVITS")]
    os.environ["PATH"] = str(engine) + os.pathsep + os.environ["PATH"]
    os.environ["HF_HUB_OFFLINE"] = "1"
    os.environ["NLTK_DATA"] = str(engine / "nltk_data")
    import numpy as np
    import torch
    from TTS_infer_pack.TTS import TTS, TTS_Config

    torch.set_num_threads(min(8, os.cpu_count() or 1))
    settings = {
        "device": args.device, "is_half": False, "version": "v2ProPlus",
        "t2s_weights_path": str(gpt), "vits_weights_path": str(sovits),
        "bert_base_path": str(engine / "GPT_SoVITS/pretrained_models/chinese-roberta-wwm-ext-large"),
        "cnhuhbert_base_path": str(engine / "GPT_SoVITS/pretrained_models/chinese-hubert-base"),
    }
    for key in ("t2s_weights_path", "vits_weights_path", "bert_base_path", "cnhuhbert_base_path"):
        if not Path(settings[key]).exists():
            raise FileNotFoundError(settings[key])
    synthesizer = TTS(TTS_Config({"custom": settings}))
    parameters = {
        "text_lang": "zh", "prompt_lang": "zh", "prompt_text": PROMPT,
        "top_k": 15, "top_p": 1.0, "temperature": 1.0,
        "text_split_method": "cut3", "batch_size": 1, "split_bucket": True,
        "speed_factor": 1.0, "fragment_interval": 0.3, "seed": 20260403,
        "parallel_infer": True, "repetition_penalty": 1.35,
        "return_fragment": False, "streaming_mode": False,
    }
    report_path = output / "generation.json"
    previous = json.loads(report_path.read_text(encoding="utf-8")) if report_path.exists() else {}
    reports = {item["id"]: item for item in previous.get("clips", [])}
    for line in lines:
        if line["id"] not in selected:
            continue
        params = parameters | line.get("parameters", {})
        parts = list(synthesizer.run(params | {"text": line["text"], "ref_audio_path": str(reference)}))
        if len(parts) != 1:
            raise ValueError(f"{line['id']}：预期一个完整音频，实际 {len(parts)} 段。")
        rate, samples = parts[0]
        if samples.dtype != np.int16 or samples.ndim != 1 or not 8000 <= rate <= 48000:
            raise ValueError("引擎没有返回单声道 PCM16 音频。")
        audio = samples.astype(np.float64) / 32768
        active = np.flatnonzero(np.abs(audio) >= 0.004)
        if active.size == 0:
            raise ValueError(f"{line['id']}：生成了静音。")
        # 只收拢首尾静音并保留余量，不裁剪正文或句间停顿。
        first = max(0, int(active[0]) - int(rate * 0.1))
        last = min(len(audio), int(active[-1]) + int(rate * 0.2))
        audio = audio[first:last]
        duration = len(audio) / rate
        if not 0.5 <= duration <= 7.0:
            raise ValueError(f"{line['id']}：时长 {duration:.3f} 秒，请调整该条 parameters.speed_factor 后重新生成，禁止截断正文。")
        # 只用于拦截明显提前结束的结果，不能替代逐条听感与转写核对。
        speech_units = len(re.findall(r"[\u4e00-\u9fff]", line["text"])) + len(re.findall(r"[A-Za-z]", line["text"])) / 2
        if duration < speech_units / 8:
            raise ValueError(f"{line['id']}：语速异常或疑似漏读，请调整随机种子重新生成并核对全文。")
        peak = float(np.max(np.abs(audio)))
        audio *= (10 ** (-1 / 20)) / peak
        pcm = np.rint(audio * 32767).astype("<i2")
        target = output / (line["id"] + ".wav")
        with wave.open(str(target), "wb") as stream:
            stream.setnchannels(1)
            stream.setsampwidth(2)
            stream.setframerate(rate)
            stream.writeframes(pcm.tobytes())
        reports[line["id"]] = {
            "id": line["id"], "text": line["text"], "sha256": sha256(target),
            "sample_rate": rate, "duration_seconds": round(duration, 4),
            "peak_dbfs": -1, "parameters": params,
        }
        report = {
            "model": MODEL_NAME, "engine_revision": ENGINE_REVISION,
            "engine_archive_sha256": ENGINE_ARCHIVE_SHA256,
            "device": args.device, "precision": "float32",
            "gpt_sha256": sha256(gpt), "sovits_sha256": sha256(sovits),
            "reference_file": reference.name, "reference_sha256": sha256(reference),
            "python": sys.version.split()[0],
            "packages": {name: importlib.metadata.version(name) for name in ("torch", "torchaudio", "numpy", "transformers", "librosa")},
            "normalization": "首尾保留 100/200 毫秒余量，峰值统一 -1 dBFS，不裁剪正文。",
            "clips": [reports[item] for item in ids if item in reports],
        }
        report_path.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print(f"完成 {line['id']}: {duration:.3f} 秒", flush=True)


if __name__ == "__main__":
    main()
