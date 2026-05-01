#!/usr/bin/env python3

from pathlib import Path
import argparse
import sys

from PIL import Image


def transparent_padding(image: Image.Image) -> tuple[int, int, int, int]:
    alpha = image.getchannel("A")
    bbox = alpha.getbbox()
    if bbox is None:
        return image.width, image.height, image.width, image.height
    left, top, right, bottom = bbox
    return left, top, image.width - right, image.height - bottom


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--max-padding", type=int, default=3)
    parser.add_argument(
        "--sprites-dir",
        type=Path,
        default=Path(__file__).parent / "../../client/public/sprites",
    )
    args = parser.parse_args()

    failures: list[str] = []
    for path in sorted(args.sprites_dir.rglob("*.png")):
        with Image.open(path).convert("RGBA") as image:
            left, top, right, bottom = transparent_padding(image)
        worst = max(left, top, right, bottom)
        if worst > args.max_padding:
            failures.append(
                f"{path}: padding l={left} t={top} r={right} b={bottom} > {args.max_padding}",
            )

    if failures:
        print("Sprite padding validation failed:")
        for failure in failures:
            print(f" - {failure}")
        return 1

    print("Sprite padding validation passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
