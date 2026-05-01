#!/usr/bin/env python3
"""
generate.py — Call the Lemonade server images API to generate raw sprites.

Usage:
    python generate.py --all          # Generate everything
    python generate.py --category entities --key rover
    python generate.py --category resources
"""

import argparse
import base64
import io
import os
import sys
import time
from pathlib import Path

import requests
import yaml
from PIL import Image


CONFIG_PATH = Path(__file__).with_name("config.yaml")
RAW_DIR = Path(__file__).parent / "raw"


def trim_transparent_bounds(data: bytes, padding: int = 2) -> bytes:
    with Image.open(io.BytesIO(data)).convert("RGBA") as image:
        alpha = image.getchannel("A")
        bbox = alpha.getbbox()
        if bbox is None:
            out = io.BytesIO()
            image.save(out, format="PNG")
            return out.getvalue()
        left, top, right, bottom = bbox
        left = max(0, left - padding)
        top = max(0, top - padding)
        right = min(image.width, right + padding)
        bottom = min(image.height, bottom + padding)
        cropped = image.crop((left, top, right, bottom))
        out = io.BytesIO()
        cropped.save(out, format="PNG")
        return out.getvalue()


def load_config():
    with open(CONFIG_PATH, "r", encoding="utf-8") as f:
        return yaml.safe_load(f)


def ensure_model_loaded(cfg: dict) -> bool:
    """Ping the server and load the model if needed."""
    host = cfg["api"]["host"]
    model = cfg["api"]["model"]
    # Try a cheap health-like check via chat completions
    try:
        r = requests.post(
            f"{host}/api/v1/chat/completions",
            json={"model": model, "messages": [{"role": "user", "content": "hi"}]},
            timeout=30,
        )
        if r.status_code == 200:
            return True
        # Model not loaded; attempt to load via lemonade CLI
        print(f"Model {model} not loaded. Attempting to load...")
        os.system(f"lemonade-server load {model}")
        time.sleep(10)
        return True
    except Exception as e:
        print(f"Server unreachable: {e}")
        return False


def generate_one(cfg: dict, key: str, prompt: str, _size: int | list) -> bytes | None:
    host = cfg["api"]["host"]
    endpoint = cfg["api"]["endpoint"]
    model = cfg["api"]["model"]
    defaults = cfg["api"].get("defaults", {})

    width = defaults.get("width", 512)
    height = defaults.get("height", 512)

    full_prompt = f"{cfg['style_prefix'].strip()}\n\nSubject: {prompt}"

    payload = {
        "model": model,
        "prompt": full_prompt,
        "n": 1,
        "size": f"{width}x{height}",
    }

    try:
        resp = requests.post(f"{host}{endpoint}", json=payload, timeout=120)
        resp.raise_for_status()
        data = resp.json()
        # OpenAI-compatible format: data[0].b64_json
        b64 = data["data"][0]["b64_json"]
        return base64.b64decode(b64)
    except Exception as e:
        print(f"  ERROR generating {key}: {e}")
        return None


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--all", action="store_true", help="Generate every sprite")
    parser.add_argument("--category", type=str, help="Category to generate")
    parser.add_argument("--key", type=str, help="Specific sprite key")
    args = parser.parse_args()

    cfg = load_config()
    if not ensure_model_loaded(cfg):
        sys.exit(1)

    RAW_DIR.mkdir(parents=True, exist_ok=True)

    generated = 0
    failed = 0

    for category, items in cfg["sprites"].items():
        if args.category and category != args.category:
            continue
        for item in items:
            key = item["key"]
            if args.key and key != args.key:
                continue
            out_path = RAW_DIR / f"{key}.png"
            if out_path.exists():
                print(f"[SKIP] {key} already exists at {out_path}")
                continue

            print(f"[GEN] {category}/{key} ...")
            img_bytes = generate_one(cfg, key, item["prompt"], item["size"])
            if img_bytes:
                img_bytes = trim_transparent_bounds(
                    img_bytes,
                    int(cfg.get("trim_padding", 2)),
                )
                out_path.write_bytes(img_bytes)
                print(f"  -> {out_path}")
                generated += 1
            else:
                failed += 1
            time.sleep(1)  # Rate-limit politely

    print(f"\nDone. Generated: {generated}, Failed: {failed}")


if __name__ == "__main__":
    main()
